//! Owned image VOLUME allocation, persistence and cleanup through the actual Host.
use super::*;
use kbox_core::{volume::VolumeConfig, ExecutionGeneration, ExecutionId};
use std::os::unix::fs::MetadataExt;

fn allocations(root: &Path, id: &ExecutionId) -> Vec<String> {
    ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(id)
        .unwrap()
        .unwrap()
        .anonymous_volumes
}

fn inventory(
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) -> Vec<serde_json::Value> {
    let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(id)
        .unwrap()
        .unwrap();
    let metadata = record.managed_execution.as_ref().unwrap();
    assert_eq!(metadata.runtime_route, ManagedRuntimeRoute::OciSdk);
    assert_eq!(metadata.generation, generation);
    let binding = metadata.oci_runtime.as_ref().unwrap();
    let share = root
        .join("service/runtime/shares")
        .join(binding.target.id.as_str())
        .join(binding.target.generation.unwrap().0.to_string());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(share.join(".koci-agent-vm-attachments.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["schemaVersion"], "koci.agent-vm-attachments.v7");
    assert_eq!(
        manifest["target"],
        serde_json::to_value(&binding.target).unwrap()
    );
    assert!(manifest["rootfsImage"].is_object());
    let storage = manifest["storage"].as_array().unwrap();
    assert_eq!(storage.len(), record.anonymous_volumes.len());
    let volumes =
        crate::VolumeStore::new(root.join("home/volumes.json"), root.join("home/volumes"));
    let mut result = Vec::new();
    for name in &record.anonymous_volumes {
        let volume = volumes.require_anonymous(name, id.as_str()).unwrap();
        let source = crate::volume::anonymous_ext4_disk_path(Path::new(&volume.mount_point));
        let entries: Vec<_> = storage
            .iter()
            .filter(|entry| entry["hostSource"] == source.to_string_lossy().as_ref())
            .collect();
        assert_eq!(entries.len(), 1);
        let entry = entries[0];
        let source_metadata = fs::metadata(&source).unwrap();
        assert_eq!(entry["sourceIdentity"]["device"], source_metadata.dev());
        assert_eq!(entry["sourceIdentity"]["inode"], source_metadata.ino());
        assert_eq!(
            entry["sourceIdentity"]["size"],
            u64::from(ResourceConfig::default().disk_mb) * 1024 * 1024
        );
        assert_eq!(entry["ownership"], "caller");
        assert_eq!(entry["cleanup"], "detach-only");
        assert_eq!(entry["accessMode"], "read-write");
        let allocation =
            uuid::Uuid::parse_str(&volume.labels[crate::volume::ALLOCATION_ID_LABEL]).unwrap();
        assert_eq!(
            entry["identity"],
            format!("kbox.volume.v1.{}", allocation.simple())
        );
        assert_eq!(source_metadata.uid(), fs::metadata(root).unwrap().uid());
        assert_eq!(source_metadata.mode() & 0o777, 0o600);
        let artifact: serde_json::Value = serde_json::from_slice(
            &fs::read(source.parent().unwrap().join("artifact.json")).unwrap(),
        )
        .unwrap();
        result.push(json!({"name":name,"identity":entry["identity"],"source":source,"sourceIdentity":entry["sourceIdentity"],"artifact":artifact}));
    }
    fs::write(
        root.join(format!(
            "anonymous-manifest-{id}-g{}.json",
            generation.get()
        )),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    result
}

async fn retained_data(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let result = manager.execute(id, generation, exec(&["/bin/sh", "-c", "set -eu; test \"$(cat /ownership/secret)\" = \"$(printf 'seed\\nchanged')\"; test \"$(cat /ownership/created)\" = owned; test /ownership/secret -ef /ownership/alias; test \"$(readlink /ownership/link)\" = secret; test \"$(stat -c '%u:%g:%a' /ownership/secret)\" = 1001:2002:600; test \"$(cat /gid-owned/created)\" = group-write; test \"$(cat '/空 卷/marker')\" = persistent"])).await.unwrap();
    assert_eq!(
        result.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn cli(root: &Path, image: &str) -> bool {
    let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_CLI") else {
        return false;
    };
    let run = |args: &[&str]| {
        let output = Command::new(&binary)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", "microvm")
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
    let id = String::from_utf8(run(&[
        "run",
        "-d",
        "--cpus",
        "1",
        "--memory",
        "512",
        "--network",
        "none",
        image,
        "--",
        "/bin/sleep",
        "2147483647",
    ]))
    .unwrap()
    .trim()
    .to_string();
    run(&[
        "exec",
        &id,
        "--",
        "/bin/sh",
        "-c",
        "printf cli-volume > '/空 卷/cli'",
    ]);
    run(&["restart", &id]);
    assert_eq!(
        run(&["exec", &id, "--", "/bin/cat", "/空 卷/cli"]),
        b"cli-volume"
    );
    let inspected: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(
        inspected[0]["managed_execution"]["runtime_route"],
        "oci_sdk"
    );
    assert_eq!(
        inspected[0]["anonymous_volumes"].as_array().unwrap().len(),
        3
    );
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned root and fixed installed independent HVF assets"]
fn native_anonymous_volumes_preserve_guest_metadata_identity_restart_and_owned_cleanup() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host = Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let volumes = crate::VolumeStore::new(root.join("home/volumes.json"),root.join("home/volumes"));
            let result = std::panic::AssertUnwindSafe(async {
                let image = std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let manager = connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let mut req = request(&image,"anonymous-a");
                files::configure(&root,"anonymous-a",&mut req);
                let operation = OperationId::new("anonymous-a-create").unwrap();
                let first = manager.create(req.clone(),&operation).await.unwrap();
                let replay = manager.create(req,&operation).await.unwrap();
                assert_eq!((first.execution_id.clone(),first.generation),(replay.execution_id,replay.generation));
                assert_eq!(allocations(&root,&first.execution_id).len(),3);
                assert!(volumes.list().unwrap().is_empty(),"reservation must not materialize volume data");
                manager.start(&first.execution_id,first.generation).await.unwrap();
                let first_inventory = inventory(&root,&first.execution_id,first.generation);
                let ownership = ownership::check(&manager,&first.execution_id,first.generation).await;
                files::check(&manager,&root,"anonymous-a",&first.execution_id,first.generation).await;
                assert_eq!(manager.execute(&first.execution_id,first.generation,exec(&["/bin/sh","-c","printf persistent > '/空 卷/marker'"])).await.unwrap().exit_code,0);
                let second = manager.create_and_start(request(&image,"anonymous-b"),&OperationId::new("anonymous-b-create").unwrap()).await.unwrap();
                let second_names = allocations(&root,&second.execution_id);
                let second_inventory = inventory(&root,&second.execution_id,second.generation);
                for a in &first_inventory { for b in &second_inventory { assert_ne!(a["identity"],b["identity"]); assert_ne!(a["sourceIdentity"],b["sourceIdentity"]); } }
                let isolated = manager.execute(&second.execution_id,second.generation,exec(&["/bin/sh","-c","set -eu; test \"$(cat /ownership/secret)\" = seed; test ! -e /ownership/created; test ! -e '/空 卷/marker'; printf separate > /ownership/secret"])).await.unwrap();
                assert_eq!(isolated.exit_code,0,"{}",String::from_utf8_lossy(&isolated.stderr));
                retained_data(&manager,&first.execution_id,first.generation).await;
                let restarted = manager.restart(&first.execution_id,first.generation,&OperationId::new("anonymous-normal-restart").unwrap()).await.unwrap();
                assert_eq!(restarted.generation.get(),first.generation.get()+1);
                assert_eq!(inventory(&root,&first.execution_id,restarted.generation),first_inventory);
                retained_data(&manager,&first.execution_id,restarted.generation).await;
                assert!(manager.execute(&first.execution_id,first.generation,exec(&["/bin/touch","/ownership/stale"])).await.is_err());
                files::check(&manager,&root,"anonymous-a",&first.execution_id,restarted.generation).await;
                manager.kill(&second.execution_id,second.generation).await.unwrap();
                for name in &second_names { assert!(volumes.get(name).unwrap().is_some()); }
                manager.remove(&second.execution_id,second.generation).await.unwrap();
                for name in &second_names { assert!(volumes.get(name).unwrap().is_none()); assert!(!root.join("home/volumes").join(name).exists()); }
                retained_data(&manager,&first.execution_id,restarted.generation).await;
                let caller = root.join("override-caller"); fs::create_dir(&caller).unwrap();fs::write(caller.join("input"),b"caller-retained").unwrap();
                let named = volumes.create(VolumeConfig::new("override-named","")).unwrap();
                fs::write(Path::new(&named.mount_point).join("input"),b"named-retained").unwrap();
                let mut override_req = request(&image,"anonymous-overrides");
                override_req.config.volumes=vec![format!("{}:/ownership:rw",caller.display()),format!("{}:/gid-owned:rw",named.mount_point)];
                override_req.policy.volume_names=vec![named.name.clone()];
                let overridden = manager.create_and_start(override_req,&OperationId::new("anonymous-overrides").unwrap()).await.unwrap();
                assert_eq!(allocations(&root,&overridden.execution_id).len(),1);
                let check = manager.execute(&overridden.execution_id,overridden.generation,exec(&["/bin/sh","-c","set -eu; test \"$(cat /ownership/input)\" = caller-retained; test \"$(cat /gid-owned/input)\" = named-retained; test ! -e /ownership/secret; printf override-write > /ownership/output; printf named-write > /gid-owned/output"])).await.unwrap();
                assert_eq!(check.exit_code,0,"{}",String::from_utf8_lossy(&check.stderr));
                manager.kill(&overridden.execution_id,overridden.generation).await.unwrap();manager.remove(&overridden.execution_id,overridden.generation).await.unwrap();
                assert_eq!(fs::read(caller.join("output")).unwrap(),b"override-write");assert_eq!(fs::read(Path::new(&named.mount_point).join("output")).unwrap(),b"named-write");
                let mut ephemeral = request(&image,"anonymous-auto-remove");ephemeral.policy.auto_remove=true;
                let ephemeral = manager.create_and_start(ephemeral,&OperationId::new("anonymous-auto-remove").unwrap()).await.unwrap();
                let ephemeral_names=allocations(&root,&ephemeral.execution_id);
                assert_eq!(manager.execute(&ephemeral.execution_id,ephemeral.generation,exec(&["/bin/sh","-c","printf retained > '/空 卷/ephemeral'"])).await.unwrap().exit_code,0);
                let ephemeral_restart=manager.restart(&ephemeral.execution_id,ephemeral.generation,&OperationId::new("anonymous-auto-restart").unwrap()).await.unwrap();
                assert_eq!(allocations(&root,&ephemeral.execution_id),ephemeral_names);
                assert_eq!(manager.execute(&ephemeral.execution_id,ephemeral_restart.generation,exec(&["/bin/cat","/空 卷/ephemeral"])).await.unwrap().stdout,b"retained");
                manager.kill(&ephemeral.execution_id,ephemeral_restart.generation).await.unwrap();
                for name in &ephemeral_names { assert!(volumes.get(name).unwrap().is_none()); }
                manager.remove(&ephemeral.execution_id,ephemeral_restart.generation).await.unwrap();
                let cli_checked=cli(&root,&image);
                let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(&first.execution_id).unwrap().unwrap();let binding=record.managed_execution.unwrap().oci_runtime.unwrap();
                host.sigkill();assert!(manager.inspect(&first.execution_id).await.is_err());
                let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));
                let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline { tokio::time::sleep(Duration::from_millis(100)).await; }assert!(recovery.is_file());
                host=Host::start(&root,2);let reconstructed=connect(&root,OciMigrationPolicy::LegacyOnly).await;
                assert_eq!(reconstructed.inspect(&first.execution_id).await.unwrap().state,ExecutionState::Stopped);
                let recovered=reconstructed.restart(&first.execution_id,restarted.generation,&OperationId::new("anonymous-owner-restart").unwrap()).await.unwrap();assert_eq!(recovered.generation.get(),restarted.generation.get()+1);
                assert_eq!(inventory(&root,&first.execution_id,recovered.generation),first_inventory);retained_data(&reconstructed,&first.execution_id,recovered.generation).await;
                files::check(&reconstructed,&root,"anonymous-a",&first.execution_id,recovered.generation).await;
                reconstructed.kill(&first.execution_id,recovered.generation).await.unwrap();reconstructed.remove(&first.execution_id,recovered.generation).await.unwrap();
                assert_eq!(volumes.list().unwrap().len(),1);assert_eq!(volumes.get(&named.name).unwrap().unwrap().in_use_by.len(),0);
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","cliChecked":cli_checked,"ownership":ownership,"initialAllocations":first_inventory,"independentAllocations":second_inventory,"restartRetainsData":true,"stoppedRecoveryRetainsData":true,"oldGenerationRefused":true,"autoRemoveAndExplicitRestart":true,"overridesRetained":true,"fileRoProbeCount":3})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none() { host=Host::start(&root,99); }
            cleanup(&root).await;
            let retained=volumes.list().unwrap();fs::write(root.join("anonymous-retained-after-runtime-cleanup.json"),serde_json::to_vec_pretty(&retained).unwrap()).unwrap();
            for volume in retained { assert_eq!(volume.name,"override-named");assert!(volume.in_use_by.is_empty());volumes.remove(&volume.name,false).unwrap(); }
            assert!(volumes.list().unwrap().is_empty());drop(host);assert!(!root.join("service/runtime.sock").exists());
            if let Err(error)=result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires sentinel-owned root, the previous installed Host and archived unaware CLI"]
fn native_old_host_refuses_raw_volume_capability_before_allocation_or_workload() {
    use koci_sdk::{ListRequest, RuntimeClient, RuntimeTransportClient};
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let reserved=manager.create(request(&image,"anonymous-old-host"),&OperationId::new("anonymous-old-host").unwrap()).await.unwrap();
                let store=ManagedExecutionStore::new(root.join("home/boxes.json"));assert_eq!(allocations(&root,&reserved.execution_id).len(),3);assert!(store.path().is_dir());
                let data_path=crate::BoxStateStore::load_readonly(store.path()).unwrap().data_path().unwrap();let before=fs::read(&data_path).unwrap();
                let output=Command::new(std::env::var_os("K_BOX_TEST_HVF_OLD_CLI").unwrap()).env("K_HOME",root.join("home")).env("K_BOX_OCI_HVF_ENDPOINT",root.join("service/runtime.sock")).env("K_BOX_OCI_MIGRATION","off").args(["start",reserved.execution_id.as_str()]).output().unwrap();
                fs::write(root.join("old-cli.stderr"),&output.stderr).unwrap();assert!(!output.status.success());assert_eq!(fs::read(&data_path).unwrap(),before);
                let error=manager.start(&reserved.execution_id,reserved.generation).await.unwrap_err();fs::write(root.join("old-host-error.txt"),format!("{error:?}\n")).unwrap();
                let record=store.get(&reserved.execution_id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);assert!(metadata.oci_runtime.is_none());
                let volumes=crate::VolumeStore::new(root.join("home/volumes.json"),root.join("home/volumes"));assert!(volumes.list().unwrap().is_empty());
                let stream=tokio::net::UnixStream::connect(root.join("service/runtime.sock")).await.unwrap();let client=RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());assert!(client.list(ListRequest::default()).await.unwrap().is_empty());
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","oldCliRefused":true,"oldHostRefused":true,"noAllocation":true,"noWorkload":true,"route":"oci_sdk"})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
