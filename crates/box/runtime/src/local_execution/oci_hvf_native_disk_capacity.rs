//! Actual custom root/anonymous disk capacity, ENOSPC, replay and recovery.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};
use std::os::unix::fs::MetadataExt;

fn configured(root: &Path, image: &str, label: &str, mib: u32) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.resources.disk_mb = mib;
    req.config.user = Some("0".into());
    req.config.cap_drop = vec!["ALL".into()];
    let source = root.join(format!("{label}-probe"));
    fs::copy(
        std::env::var_os("K_BOX_TEST_DISK_CAPACITY_PROBE").unwrap(),
        &source,
    )
    .unwrap();
    req.config.volumes = vec![format!("{}:/capacity-probe:ro", source.display())];
    req.config.cmd = vec!["/bin/sh".into(), "-c".into(), format!("set -eu; /capacity-probe {} / > /capacity-init-ready.json; mv /capacity-init-ready.json /capacity-init.json; exec /bin/sleep 2147483647", u64::from(mib)*1024*1024)];
    req
}

fn verify(bytes: &[u8], mib: u32, tty: bool, fill: bool) -> serde_json::Value {
    let text = String::from_utf8_lossy(bytes);
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["rawBytes"], u64::from(mib) * 1024 * 1024);
    assert_eq!(value["uid"], 0);
    assert_eq!(value["tty"], tty);
    assert_eq!(value["fill"], fill);
    assert_eq!(value["emptyCapabilities"], true);
    assert_eq!(value["noNewPrivileges"], true);
    assert!(value["filesystemBytes"].as_u64().unwrap() <= u64::from(mib) * 1024 * 1024);
    if fill {
        assert_eq!(value["fullErrno"], 28);
        assert_eq!(value["reuseSucceeded"], true);
        assert!(value["writtenBytes"].as_u64().unwrap() > 1024 * 1024);
        assert!(value["writtenBytes"].as_u64().unwrap() < value["rawBytes"].as_u64().unwrap());
    }
    value
}

async fn measure(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    mib: u32,
    path: &str,
    fill: bool,
) -> serde_json::Value {
    let size = (u64::from(mib) * 1024 * 1024).to_string();
    let mut args = vec!["/capacity-probe", &size, path];
    if fill {
        args.push("--fill");
    }
    let mut command = exec(&args);
    command.timeout_ns = 60_000_000_000;
    let output = manager.execute(id, generation, command).await.unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{path}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    verify(&output.stdout, mib, false, fill)
}

fn inventory(
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    mib: u32,
) -> serde_json::Value {
    let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(id)
        .unwrap()
        .unwrap();
    let metadata = record.managed_execution.as_ref().unwrap();
    assert_eq!(metadata.runtime_route, ManagedRuntimeRoute::OciSdk);
    assert_eq!(metadata.generation, generation);
    assert_eq!(metadata.request.config.resources.disk_mb, mib);
    let binding = metadata.oci_runtime.as_ref().unwrap();
    let share = root
        .join("service/runtime/shares")
        .join(binding.target.id.as_str())
        .join(binding.target.generation.unwrap().0.to_string());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(share.join(".koci-agent-vm-attachments.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["target"],
        serde_json::to_value(&binding.target).unwrap()
    );
    let bytes = u64::from(mib) * 1024 * 1024;
    assert_eq!(manifest["rootfsImage"]["image"]["size"], bytes);
    let disk = share.with_extension("rootfs.ext4");
    let info = fs::symlink_metadata(&disk).unwrap();
    assert!(info.is_file() && !info.file_type().is_symlink());
    assert_eq!(info.len(), bytes);
    assert_eq!(info.mode() & 0o777, 0o600);
    assert_eq!(info.uid(), fs::metadata(root).unwrap().uid());
    assert_eq!(
        manifest["rootfsImage"]["sourceIdentity"]["device"],
        info.dev()
    );
    assert_eq!(
        manifest["rootfsImage"]["sourceIdentity"]["inode"],
        info.ino()
    );
    let volumes =
        crate::VolumeStore::new(root.join("home/volumes.json"), root.join("home/volumes"));
    let entries = manifest
        .get("storage")
        .map(|value| value.as_array().unwrap().as_slice())
        .unwrap_or(&[]);
    assert_eq!(entries.len(), record.anonymous_volumes.len());
    for name in &record.anonymous_volumes {
        let allocation = volumes.require_anonymous(name, id.as_str()).unwrap();
        let source = crate::volume::anonymous_ext4_disk_path(Path::new(&allocation.mount_point));
        let entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry["hostSource"] == source.to_string_lossy().as_ref())
            .collect();
        assert_eq!(entries.len(), 1);
        let entry = entries[0];
        let info = fs::metadata(&source).unwrap();
        assert_eq!(info.len(), bytes);
        assert_eq!(info.uid(), fs::metadata(root).unwrap().uid());
        assert_eq!(info.mode() & 0o777, 0o600);
        assert_eq!(entry["sourceIdentity"]["device"], info.dev());
        assert_eq!(entry["sourceIdentity"]["inode"], info.ino());
        assert_eq!(entry["sourceIdentity"]["size"], bytes);
        assert_eq!(entry["accessMode"], "read-write");
        assert_eq!(entry["ownership"], "caller");
        assert_eq!(entry["cleanup"], "detach-only");
        let uuid =
            uuid::Uuid::parse_str(&allocation.labels[crate::volume::ALLOCATION_ID_LABEL]).unwrap();
        assert_eq!(
            entry["identity"],
            format!("kbox.volume.v1.{}", uuid.simple())
        );
        let artifact: serde_json::Value = serde_json::from_slice(
            &fs::read(source.parent().unwrap().join("artifact.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(artifact["capacity_bytes"], bytes);
    }
    fs::write(
        root.join(format!("capacity-manifest-{id}-g{}.json", generation.get())),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    json!({"rootDisk":disk,"rootIdentity":{"device":info.dev(),"inode":info.ino(),"size":info.len()},"rootAllocatedBytes":info.blocks()*512,"storage":entries})
}

async fn paths(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
) {
    let ready = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let output = manager
            .execute(
                id,
                generation,
                exec(&["/bin/sh", "-c", "test -s /capacity-init.json"]),
            )
            .await
            .unwrap();
        if output.exit_code == 0 {
            break;
        }
        assert!(tokio::time::Instant::now() < ready);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let initial = manager
        .execute(id, generation, exec(&["/bin/cat", "/capacity-init.json"]))
        .await
        .unwrap();
    assert_eq!(initial.exit_code, 0);
    let mut observations = vec![verify(&initial.stdout, 128, false, false)];
    observations.push(measure(manager, id, generation, 128, "/", false).await);
    let child = manager
        .execute(
            id,
            generation,
            exec(&["/bin/sh", "-c", "/capacity-probe 134217728 /"]),
        )
        .await
        .unwrap();
    assert_eq!(
        child.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    observations.push(verify(&child.stdout, 128, false, false));
    for tty in [false, true] {
        let mut session = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/capacity-probe".into(), "134217728".into(), "/".into()],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: None,
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut command = exec(&["/capacity-probe", "134217728", "/", "--stdin"]);
            command.stdin_streaming = true;
            manager
                .start_process(id, generation, command)
                .await
                .unwrap()
        };
        if !tty {
            session.input().write_stdin(b"streamed\n").await.unwrap();
            session.input().close_stdin().await.unwrap();
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match session.next_event().await.unwrap().unwrap() {
                    ExecEvent::Chunk(chunk) => {
                        if chunk.stream == kbox_core::exec::StreamType::Stdout {
                            stdout.extend(chunk.data)
                        } else {
                            stderr.extend(chunk.data)
                        }
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(exit.exit_code, 0, "{}", String::from_utf8_lossy(&stderr));
        observations.push(verify(&stdout, 128, tty, false));
    }
    let mut block_devices = std::collections::BTreeSet::new();
    block_devices.insert((
        observations[0]["major"].clone().to_string(),
        observations[0]["minor"].clone().to_string(),
    ));
    for path in ["/ownership", "/gid-owned", "/空 卷"] {
        let value = measure(manager, id, generation, 128, path, false).await;
        assert!(block_devices.insert((value["major"].to_string(), value["minor"].to_string())));
        observations.push(value);
    }
    assert_eq!(block_devices.len(), 4);
    fs::write(
        root.join(format!("capacity-{label}-observations.json")),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

fn cli(root: &Path, id: &ExecutionId) {
    let binary = std::env::var_os("K_BOX_TEST_HVF_CLI").unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(&binary)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", "off")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let inspect: serde_json::Value =
        serde_json::from_slice(&run(&["inspect", id.as_str()])).unwrap();
    assert_eq!(inspect[0]["managed_execution"]["runtime_route"], "oci_sdk");
    assert_eq!(
        inspect[0]["managed_execution"]["request"]["config"]["resources"]["disk_mb"],
        256
    );
    verify(
        &run(&[
            "exec",
            id.as_str(),
            "--",
            "/capacity-probe",
            "268435456",
            "/",
        ]),
        256,
        false,
        false,
    );
    run(&[
        "exec",
        id.as_str(),
        "--",
        "/bin/sh",
        "-c",
        "printf cli-root > /cli-capacity-marker; printf cli-volume > '/空 卷/cli-capacity-marker'",
    ]);
    assert_eq!(
        run(&[
            "exec",
            id.as_str(),
            "--",
            "/bin/cat",
            "/cli-capacity-marker"
        ]),
        b"cli-root"
    );
    verify(
        &run(&[
            "exec",
            id.as_str(),
            "--",
            "/capacity-probe",
            "268435456",
            "/空 卷",
        ]),
        256,
        false,
        false,
    );
    run(&["restart", id.as_str()]);
    run(&[
        "exec",
        id.as_str(),
        "--",
        "/bin/sh",
        "-c",
        "test ! -e /cli-capacity-marker",
    ]);
    assert_eq!(
        run(&[
            "exec",
            id.as_str(),
            "--",
            "/bin/cat",
            "/空 卷/cli-capacity-marker"
        ]),
        b"cli-volume"
    );
    verify(
        &run(&[
            "exec",
            id.as_str(),
            "--",
            "/capacity-probe",
            "268435456",
            "/",
        ]),
        256,
        false,
        false,
    );
    verify(
        &run(&[
            "exec",
            id.as_str(),
            "--",
            "/capacity-probe",
            "268435456",
            "/空 卷",
        ]),
        256,
        false,
        false,
    );
    fs::write(root.join("capacity-cli.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","binary":binary,"inspectPreservedRequest":true,"explicitOffRetainsOciRoute":true,"ephemeralRootReset":true,"anonymousDataRetained":true,"restartAndCapacityRetained":true})).unwrap()).unwrap();
}

async fn seed_permissions(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    for (user, script) in [
        (None, "set -eu; if cat /ownership/secret; then exit 41; fi; test ! -w /ownership/secret"),
        (Some("1001:2002"), "set -eu; test \"$(cat /ownership/secret)\" = seed; test /ownership/secret -ef /ownership/alias; test \"$(readlink /ownership/link)\" = secret; test \"$(stat -c '%u:%g:%a' /ownership/secret)\" = 1001:2002:600; printf owned > /ownership/created; printf changed >> /ownership/secret; test ! -r /root-owned"),
        (Some("1002:2003"), "set -eu; if cat /ownership/secret; then exit 42; fi; if printf illegal > /ownership/denied; then exit 43; fi; test ! -e /ownership/denied"),
        (Some("1002:3003"), "set -eu; test \"$(cat /gid-owned/group)\" = group-secret; printf group-write > /gid-owned/created; if printf illegal >> /gid-owned/group; then exit 44; fi"),
    ] {
        let mut req=exec(&["/bin/sh","-c",script]);req.user=user.map(str::to_owned);
        let output=manager.execute(id,generation,req).await.unwrap();
        assert_eq!(output.exit_code,0,"{user:?}: {}",String::from_utf8_lossy(&output.stderr));
    }
    fs::write(root.join("capacity-owner-controls.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","ownerUid":1001,"ownerGid":2002,"ownerMode":600,"ownerReadWriteAndLinks":true,"otherUidDenied":true,"foreignGidReadWrite":true,"emptyCapabilityRootDenied":true})).unwrap()).unwrap();
}

async fn retained(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let root=manager.execute(id,generation,exec(&["/bin/sh","-c","set -eu; test ! -e /capacity-marker; test \"$(cat '/空 卷/capacity-marker')\" = main-volume; test ! -e /stale-capacity; test ! -e /capacity-fill; test ! -e '/空 卷/capacity-fill'"])).await.unwrap();
    assert_eq!(
        root.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&root.stderr)
    );
    let mut owner=exec(&["/bin/sh","-c","set -eu; test \"$(cat /ownership/secret)\" = \"$(printf 'seed\\nchanged')\"; test \"$(cat /ownership/created)\" = owned; test /ownership/secret -ef /ownership/alias; test \"$(readlink /ownership/link)\" = secret; test \"$(stat -c '%u:%g:%a' /ownership/secret)\" = 1001:2002:600"]);
    owner.user = Some("1001:2002".into());
    let owner = manager.execute(id, generation, owner).await.unwrap();
    assert_eq!(
        owner.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&owner.stderr)
    );
}

fn rebound_storage(before: &serde_json::Value, after: &serde_json::Value) {
    let before = before.as_array().unwrap();
    let after = after.as_array().unwrap();
    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(after) {
        assert_ne!(before["blockId"], after["blockId"]);
        let mut expected = before.clone();
        expected["blockId"] = after["blockId"].clone();
        assert_eq!(*after, expected);
    }
}

#[test]
#[ignore = "requires sentinel-owned root and fixed installed independent HVF assets"]
fn native_custom_disk_capacity_enforces_root_and_anonymous_disks_replay_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                let volume_image=std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let store=ManagedExecutionStore::new(root.join("home/boxes.json"));
                let volumes=crate::VolumeStore::new(root.join("home/volumes.json"),root.join("home/volumes"));
                for mib in [0,15,65537,u32::MAX] {
                    let mut req=request(&volume_image,&format!("capacity-invalid-{mib}"));req.config.resources.disk_mb=mib;
                    let error=manager.create(req,&OperationId::new(format!("capacity-invalid-{mib}")).unwrap()).await.unwrap_err();
                    assert!(matches!(error,ExecutionManagerError::InvalidRequest(_)));
                    assert!(store.list().unwrap().is_empty()&&volumes.list().unwrap().is_empty());
                    fs::write(root.join(format!("capacity-invalid-{mib}.txt")),format!("{error:?}\n")).unwrap();
                }
                let oversized=std::env::var("K_BOX_TEST_CAPACITY_OVERSIZED_IMAGE").unwrap();
                let mut too_small=request(&oversized,"capacity-too-small");too_small.config.resources.disk_mb=16;
                let small=manager.create(too_small,&OperationId::new("capacity-too-small").unwrap()).await.unwrap();
                let error=manager.start(&small.execution_id,small.generation).await.unwrap_err();
                fs::write(root.join("capacity-too-small.txt"),format!("{error:?}\n")).unwrap();
                assert!(matches!(error,ExecutionManagerError::Unavailable(_)));
                manager.remove(&small.execution_id,small.generation).await.unwrap();
                assert!(!root.join("home/boxes").join(small.execution_id.as_str()).exists());
                assert!(store.list().unwrap().is_empty()&&volumes.list().unwrap().is_empty());
                use koci_sdk::{ListRequest,RuntimeClient,RuntimeTransportClient};
                let stream=tokio::net::UnixStream::connect(root.join("service/runtime.sock")).await.unwrap();
                let client=RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());
                assert!(client.list(ListRequest::default()).await.unwrap().is_empty());
                let req=configured(&root,&volume_image,"capacity-main",128);
                let op=OperationId::new("capacity-main-create").unwrap();
                let reserved=manager.create(req.clone(),&op).await.unwrap();
                let replay=manager.create(req.clone(),&op).await.unwrap();
                assert_eq!((reserved.execution_id.clone(),reserved.generation),(replay.execution_id,replay.generation));
                let mut changed=req;changed.config.resources.disk_mb=256;
                assert!(matches!(manager.create(changed,&op).await,Err(ExecutionManagerError::Conflict {..})));
                let first=manager.start(&reserved.execution_id,reserved.generation).await.unwrap();
                let initial=inventory(&root,&first.execution_id,first.generation,128);
                assert_eq!(initial["storage"].as_array().unwrap().len(),3);
                paths(&manager,&root,&first.execution_id,first.generation,"first").await;
                let root_full=measure(&manager,&first.execution_id,first.generation,128,"/",true).await;
                let volume_full=measure(&manager,&first.execution_id,first.generation,128,"/空 卷",true).await;
                fs::write(root.join("capacity-full-observations.json"),serde_json::to_vec_pretty(&json!([root_full,volume_full])).unwrap()).unwrap();
                seed_permissions(&manager,&root,&first.execution_id,first.generation).await;
                assert_eq!(manager.execute(&first.execution_id,first.generation,exec(&["/bin/sh","-c","printf main-root > /capacity-marker; printf main-volume > '/空 卷/capacity-marker'"])).await.unwrap().exit_code,0);
                let second=manager.create_and_start(configured(&root,&volume_image,"capacity-second",256),&OperationId::new("capacity-second").unwrap()).await.unwrap();
                let other=inventory(&root,&second.execution_id,second.generation,256);
                assert_ne!(other["rootIdentity"],initial["rootIdentity"]);
                for a in other["storage"].as_array().unwrap() {for b in initial["storage"].as_array().unwrap() {assert_ne!(a["identity"],b["identity"]);assert_ne!(a["sourceIdentity"],b["sourceIdentity"]);}}
                assert_eq!(manager.execute(&second.execution_id,second.generation,exec(&["/bin/sh","-c","test ! -e /capacity-marker; printf separate > /capacity-marker"])).await.unwrap().exit_code,0);
                cli(&root,&second.execution_id);
                let current=manager.inspect(&second.execution_id).await.unwrap();
                assert_eq!(current.generation.get(),2);
                manager.kill(&second.execution_id,current.generation).await.unwrap();manager.remove(&second.execution_id,current.generation).await.unwrap();
                let big=manager.create_and_start(configured(&root,&image,"capacity-big",8192),&OperationId::new("capacity-big").unwrap()).await.unwrap();
                let big_disk=inventory(&root,&big.execution_id,big.generation,8192);
                assert!(big_disk["rootAllocatedBytes"].as_u64().unwrap()<1024*1024*1024);
                let big_guest=measure(&manager,&big.execution_id,big.generation,8192,"/",false).await;
                assert!(big_guest["filesystemBytes"].as_u64().unwrap()>4*1024*1024*1024);
                fs::write(root.join("capacity-large-control.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","host":big_disk,"guest":big_guest})).unwrap()).unwrap();
                manager.kill(&big.execution_id,big.generation).await.unwrap();manager.remove(&big.execution_id,big.generation).await.unwrap();
                assert_eq!(manager.execute(&first.execution_id,first.generation,exec(&["/bin/cat","/capacity-marker"])).await.unwrap().stdout,b"main-root");
                let restarted=manager.restart(&first.execution_id,first.generation,&OperationId::new("capacity-normal-restart").unwrap()).await.unwrap();
                assert_eq!(restarted.generation.get(),2);
                let after=inventory(&root,&first.execution_id,restarted.generation,128);rebound_storage(&initial["storage"],&after["storage"]);
                paths(&manager,&root,&first.execution_id,restarted.generation,"restart").await;
                retained(&manager,&first.execution_id,restarted.generation).await;
                assert!(manager.execute(&first.execution_id,first.generation,exec(&["/bin/touch","/stale-capacity"])).await.is_err());
                let record=store.get(&first.execution_id).unwrap().unwrap();let binding=record.managed_execution.unwrap().oci_runtime.unwrap();
                host.sigkill();assert!(manager.inspect(&first.execution_id).await.is_err());
                let report=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));
                let deadline=tokio::time::Instant::now()+Duration::from_secs(20);
                while !report.exists()&&tokio::time::Instant::now()<deadline {tokio::time::sleep(Duration::from_millis(100)).await;}
                assert!(report.is_file());host=Host::start(&root,2);
                let reconstructed=connect(&root,OciMigrationPolicy::LegacyOnly).await;
                assert_eq!(reconstructed.inspect(&first.execution_id).await.unwrap().state,ExecutionState::Stopped);
                let recovered=reconstructed.restart(&first.execution_id,restarted.generation,&OperationId::new("capacity-owner-restart").unwrap()).await.unwrap();
                assert_eq!(recovered.generation.get(),3);
                let final_disks=inventory(&root,&first.execution_id,recovered.generation,128);rebound_storage(&after["storage"],&final_disks["storage"]);
                paths(&reconstructed,&root,&first.execution_id,recovered.generation,"recovery").await;
                retained(&reconstructed,&first.execution_id,recovered.generation).await;
                reconstructed.kill(&first.execution_id,recovered.generation).await.unwrap();
                for item in initial["storage"].as_array().unwrap() {assert!(Path::new(item["hostSource"].as_str().unwrap()).is_file());}
                reconstructed.remove(&first.execution_id,recovered.generation).await.unwrap();
                for item in initial["storage"].as_array().unwrap() {assert!(!Path::new(item["hostSource"].as_str().unwrap()).exists());}
                assert!(volumes.list().unwrap().is_empty());
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","cliChecked":true,"generations":[1,2,3],"rootAndAnonymousExactCapacity":true,"hostDiskOwnerAndPrivateMode":true,"actualEnospcAndReuse":true,"initExecChildStreamRealPty":true,"independentCapacities":true,"largeSparseControl":true,"undersizedImageFailedBeforeVm":true,"invalidRefusedBeforeClaims":true,"replayOneIdentity":true,"changedRequestRefused":true,"staleGenerationRefused":true,"stoppedRecovery":true,"ephemeralRootReset":true,"anonymousDataOwnerLinksAndCapacityRetained":true,"ownedRawDisksRemoved":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none() {host=Host::start(&root,99);}
            cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());
            if let Err(error)=result {std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
