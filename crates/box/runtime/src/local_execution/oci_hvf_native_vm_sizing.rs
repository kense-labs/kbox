//! Actual Guest CPU and boot-RAM readback through the installed HVF owner.
use super::*;
use kbox_core::{ExecutionGeneration, ExecutionId, FileOp};

async fn hardware(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    cpus: u32,
    memory: u32,
    label: &str,
) -> serde_json::Value {
    let bytes = fs::read(
        std::env::var_os("K_BOX_TEST_VM_HARDWARE_PROBE").expect("fixed Guest hardware probe"),
    )
    .unwrap();
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                behavior::file(FileOp::Upload, "/vm-hardware-probe", Some(bytes))
            )
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        manager
            .execute(
                id,
                generation,
                exec(&["/bin/chmod", "755", "/vm-hardware-probe"])
            )
            .await
            .unwrap()
            .exit_code,
        0
    );
    let observed = manager
        .execute(id, generation, exec(&["/vm-hardware-probe"]))
        .await
        .unwrap();
    fs::write(
        root.join(format!(
            "hardware-{label}-g{}-probe.stdout",
            generation.get()
        )),
        &observed.stdout,
    )
    .unwrap();
    fs::write(
        root.join(format!(
            "hardware-{label}-g{}-probe.stderr",
            generation.get()
        )),
        &observed.stderr,
    )
    .unwrap();
    assert_eq!(
        observed.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&observed.stdout).unwrap();
    let proof = json!({"execution":id,"generation":generation,"requestedVcpus":cpus,"requestedMemoryMib":memory,"guest":actual});
    fs::write(
        root.join(format!("hardware-{label}-g{}.json", generation.get())),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
    assert_eq!(actual["onlineCpus"].as_u64(), Some(u64::from(cpus)));
    assert_eq!(actual["presentCpus"].as_u64(), Some(u64::from(cpus)));
    assert_eq!(
        actual["bootRamBytes"].as_u64(),
        Some(u64::from(memory) * 1024 * 1024)
    );
    assert_eq!(actual["freshReadonlySysfs"], true);
    assert_eq!(actual["guestMountRemoved"], true);
    let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(id)
        .unwrap()
        .unwrap();
    assert_eq!((record.cpus, record.memory_mb), (cpus, memory));
    let metadata = record.managed_execution.unwrap();
    assert_eq!(metadata.runtime_route, ManagedRuntimeRoute::OciSdk);
    assert_eq!(
        (
            metadata.request.config.resources.vcpus,
            metadata.request.config.resources.memory_mb
        ),
        (cpus, memory)
    );
    proof
}

#[test]
#[ignore = "requires sentinel-owned private home and fixed installed HVF/Box assets"]
fn native_vm_hardware_sizes_match_persisted_intent() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let mode=std::env::var("K_BOX_TEST_VM_SIZING_MODE").unwrap_or_else(|_|"matrix".into());
                let cases=match mode.as_str(){"baseline"=>vec![(1,512)],"matrix"=>vec![(1,128),(2,512),(4,768),(8,1024),(1,8192),(8,128)],_=>panic!("unknown hardware qualification mode")};
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                let mut rows=Vec::new();
                for (index,(cpus,memory)) in cases.into_iter().enumerate() {
                    let label=format!("size-{index}");
                    let mut req=request(&image,&label);
                    req.config.resources.vcpus=cpus;req.config.resources.memory_mb=memory;
                    req.config.resources.disk_mb=128;
                    req.config.cap_add=vec!["SYS_ADMIN".into()];
                    let execution=manager.create_and_start(req,&OperationId::new(format!("vm-{label}")).unwrap()).await.unwrap();
                    rows.push(hardware(&manager,&root,&execution.execution_id,execution.generation,cpus,memory,&label).await);
                    manager.kill(&execution.execution_id,execution.generation).await.unwrap();
                    manager.remove(&execution.execution_id,execution.generation).await.unwrap();
                }
                if mode=="matrix" {
                    let mut req=request(&image,"sized-main");
                    behavior::configure(&root,&mut req);
                    files::configure(&root,"main",&mut req);
                    req.config.resources.vcpus=2;req.config.resources.memory_mb=768;req.config.resources.disk_mb=128;
                    let main=manager.create_and_start(req,&OperationId::new("vm-main").unwrap()).await.unwrap();
                    let mut other=request(&image,"sized-other");
                    other.config.resources.vcpus=4;other.config.resources.memory_mb=1024;other.config.resources.disk_mb=128;
                    other.config.cap_add=vec!["SYS_ADMIN".into()];
                    let other=manager.create_and_start(other,&OperationId::new("vm-other").unwrap()).await.unwrap();
                    let (left,right)=tokio::join!(
                        hardware(&manager,&root,&main.execution_id,main.generation,2,768,"concurrent-main"),
                        hardware(&manager,&root,&other.execution_id,other.generation,4,1024,"concurrent-other")
                    );
                    rows.extend([left,right]);
                    manager.kill(&other.execution_id,other.generation).await.unwrap();
                    manager.remove(&other.execution_id,other.generation).await.unwrap();
                    behavior::exercise(&manager,&root,&main.execution_id,main.generation).await;
                    files::check(&manager,&root,"main",&main.execution_id,main.generation).await;
                    rows.push(hardware(&manager,&root,&main.execution_id,main.generation,2,768,"after-cgroup-update").await);
                    let restarted=manager.restart(&main.execution_id,main.generation,&OperationId::new("vm-normal-restart").unwrap()).await.unwrap();
                    assert_eq!(restarted.generation.get(),main.generation.get()+1);
                    assert!(manager.execute(&main.execution_id,main.generation,exec(&["/bin/touch","/writable/stale-before-recovery"])).await.is_err());
                    assert!(!root.join("writable/stale-before-recovery").exists());
                    rows.push(hardware(&manager,&root,&main.execution_id,restarted.generation,2,768,"normal-restart").await);
                    let legacy_manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;
                    let legacy=legacy_manager.create_and_start(request(&image,"sizing-legacy"),&OperationId::new("vm-legacy").unwrap()).await.unwrap();
                    faults::exercise(&root,&mut host,&main.execution_id,restarted.generation,&legacy.execution_id,legacy.generation).await;
                    let replacement=connect(&root,OciMigrationPolicy::LegacyOnly).await;
                    let generation=ManagedExecutionStore::new(root.join("home/boxes.json")).get(&main.execution_id).unwrap().unwrap().managed_execution.unwrap().generation;
                    assert_eq!(generation.get(),restarted.generation.get()+1);
                    rows.push(hardware(&replacement,&root,&main.execution_id,generation,2,768,"stopped-recovery").await);
                    cli_hardware(&replacement,&root,&image,&mut rows).await;
                }
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","mode":mode,"hardwareRows":rows,"cliChecked":mode=="matrix","runtimeRssMeasured":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;
            if let Err(error)=result {std::panic::resume_unwind(error);}
        });
        drop(host);
        assert!(!root.join("service/runtime.sock").exists());
    }).unwrap().join().unwrap();
}

async fn cli_hardware(
    manager: &LocalExecutionManager,
    root: &Path,
    image: &str,
    rows: &mut Vec<serde_json::Value>,
) {
    let binary = std::env::var_os("K_BOX_TEST_HVF_CLI").expect("installed CLI is required");
    let run = |policy: &str, args: &[&str]| {
        let output = Command::new(&binary)
            .current_dir(root)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", policy)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "CLI {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let id = String::from_utf8(run(
        "microvm",
        &[
            "run",
            "-d",
            "--name",
            "cli-vm-sizing",
            "--cpus",
            "4",
            "--memory",
            "1024",
            "--cap-add",
            "SYS_ADMIN",
            "--network",
            "none",
            image,
            "--",
            "/bin/sleep",
            "2147483647",
        ],
    ))
    .unwrap()
    .trim()
    .to_string();
    let execution_id = ExecutionId::new(id.clone()).unwrap();
    let generation = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(&execution_id)
        .unwrap()
        .unwrap()
        .managed_execution
        .unwrap()
        .generation;
    rows.push(
        hardware(
            manager,
            root,
            &execution_id,
            generation,
            4,
            1024,
            "cli-created",
        )
        .await,
    );
    let observed: serde_json::Value =
        serde_json::from_slice(&run("off", &["exec", &id, "--", "/vm-hardware-probe"])).unwrap();
    assert_eq!(observed["onlineCpus"], 4);
    assert_eq!(observed["bootRamBytes"], 1024_u64 * 1024 * 1024);
    assert_eq!(observed["guestMountRemoved"], true);
    run("off", &["rm", "-f", &id]);
    fs::write(root.join("cli.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","binary":binary,"created":id,"guest":observed,"persistedOciRoute":true})).unwrap()).unwrap();
}

#[test]
#[ignore = "requires sentinel-owned private home and fixed installed old/current assets"]
fn native_old_vm_sizing_assets_refuse_before_workload_without_fallback() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let old_host=std::env::var("K_BOX_TEST_VM_SIZING_OLD_ASSET").as_deref()==Ok("old-host");
        let mut host=Some(Host::start(&root,1));
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let entry=root.join("compat-entry");fs::create_dir(&entry).unwrap();
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&entry,fs::Permissions::from_mode(0o777)).unwrap();
                let marker=entry.join("entered");
                let mut req=request(&std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),"vm-sizing-compatibility");
                req.config.resources.vcpus=2;req.config.resources.memory_mb=768;req.config.resources.disk_mb=128;
                req.config.volumes.push(format!("{}:/compat:rw",entry.display()));
                req.config.cmd=vec!["/bin/sh".into(),"-c".into(),"printf forbidden > /compat/entered; exec /bin/sleep 2147483647".into()];
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let refused=manager.create_and_start(req,&OperationId::new("vm-sizing-compat-create").unwrap()).await.expect_err("old assets must refuse explicit hardware intent");
                fs::write(root.join("vm-sizing-compatibility-refusal.txt"),format!("{refused:?}\n")).unwrap();
                if old_host {
                    assert!(matches!(refused,kbox_core::ExecutionManagerError::InvalidRequest(ref reason) if reason.contains(koci_sdk::VM_RESOURCES_EXTENSION)),"{refused}");
                } else {
                    assert!(matches!(refused,kbox_core::ExecutionManagerError::Unavailable(_)),"{refused}");
                }
                assert!(!marker.exists());
                let records=ManagedExecutionStore::new(root.join("home/boxes.json")).list().unwrap();
                fs::write(root.join("vm-sizing-compatibility-records.json"),serde_json::to_vec_pretty(&records).unwrap()).unwrap();
                if old_host { assert!(records.is_empty()); }
                else if !records.is_empty() {
                    assert_eq!(records.len(),1);
                    assert_eq!(records[0].managed_execution.as_ref().unwrap().runtime_route,ManagedRuntimeRoute::OciSdk);
                    drop(manager);drop(host.take());
                    let shim=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_SHIM").unwrap());
                    let manifest=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_MANIFEST").unwrap());
                    host=Some(Host::start_with_assets(&root,2,&shim,&manifest));
                    assert_eq!(pending::drain(&root).await,1);
                    let proof:serde_json::Value=serde_json::from_slice(&fs::read(root.join("pending-create-cleanup.json")).unwrap()).unwrap();
                    assert_eq!(proof["workloadStartsAdded"],0);
                }
                cleanup(&root).await;assert!(!marker.exists());
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","oldHost":old_host,"workloadMarkerAbsent":true,"noRouteOrSizingFallback":true,"cleanupWithoutStart":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;if let Err(error)=result{std::panic::resume_unwind(error);}
        });drop(host);assert!(!root.join("service/runtime.sock").exists());
    }).unwrap().join().unwrap();
}
