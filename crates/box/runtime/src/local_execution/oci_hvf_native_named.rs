//! Actual named-volume lifetime, live data and Host PEP qualification.
use super::*;
use kbox_core::{volume::VolumeConfig, ExecutionGeneration, ExecutionId};
use std::os::unix::fs::MetadataExt;

fn storage_identity(root: &Path, id: &ExecutionId, source: &Path) -> String {
    let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(id)
        .unwrap()
        .unwrap();
    let binding = record.managed_execution.unwrap().oci_runtime.unwrap();
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
    let entries: Vec<_> = manifest["directories"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["hostSource"] == source.to_string_lossy().as_ref())
        .collect();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    let metadata = fs::metadata(source).unwrap();
    assert_eq!(entry["sourceIdentity"]["device"], metadata.dev());
    assert_eq!(entry["sourceIdentity"]["inode"], metadata.ino());
    assert_eq!(entry["attachment"]["ownership"], "caller");
    assert_eq!(entry["attachment"]["cleanup"], "detach-only");
    let identity = entry["attachment"]["identity"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(identity.starts_with("kbox.volume.v1."));
    fs::write(
        root.join(format!(
            "named-manifest-{}-{}.json",
            id,
            binding.target.generation.unwrap().0
        )),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    identity
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
    run(&["volume", "create", "cli-named"]);
    let id = String::from_utf8(run(&[
        "run",
        "-d",
        "--cpus",
        "1",
        "--memory",
        "512",
        "--network",
        "none",
        "--user",
        "0",
        "-v",
        "cli-named:/data",
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
        "printf cli-data > /data/cli",
    ]);
    let record: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(record[0]["managed_execution"]["runtime_route"], "oci_sdk");
    run(&["rm", "-f", &id]);
    let store = crate::VolumeStore::new(root.join("home/volumes.json"), root.join("home/volumes"));
    let volume = store.get("cli-named").unwrap().unwrap();
    assert!(volume.in_use_by.is_empty());
    assert_eq!(
        fs::read(Path::new(&volume.mount_point).join("cli")).unwrap(),
        b"cli-data"
    );
    true
}

#[test]
#[ignore = "requires sentinel-owned home and fixed installed independent HVF assets"]
fn native_named_volumes_preserve_identity_live_data_ro_and_shared_lifetime() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
        let mut host = Host::start(&root, 1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let volumes = crate::VolumeStore::new(root.join("home/volumes.json"), root.join("home/volumes"));
            let ro = volumes.create(VolumeConfig::new("qual-named-ro", "")).unwrap();
            let rw = volumes.create(VolumeConfig::new("qual-named-rw", "")).unwrap();
            let readonly = PathBuf::from(&ro.mount_point);
            let writable = PathBuf::from(&rw.mount_point);
            std::os::unix::fs::symlink(&readonly, root.join("readonly")).unwrap();
            std::os::unix::fs::symlink(&writable, root.join("writable")).unwrap();
            fs::create_dir(readonly.join("empty")).unwrap();
            fs::write(readonly.join("input"), b"caller-owned").unwrap();
            fs::write(readonly.join("replacement"), b"replacement").unwrap();
            fs::write(readonly.join("live"), b"short").unwrap();
            fs::write(root.join("ungranted"), b"outside-caller-grants").unwrap();
            std::os::unix::fs::symlink(root.join("ungranted"), writable.join("ungranted-link")).unwrap();
            let before = fs::metadata(readonly.join("input")).unwrap();
            let result = std::panic::AssertUnwindSafe(async {
                let image = std::env::var("K_BOX_TEST_HVF_NAMED_IMAGE").unwrap();
                let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                let mut req = request(&image, "named-a");
                req.config.user = Some("0".into());
                req.config.cap_add = vec!["SYS_ADMIN".into()];
                req.config.volumes = vec![format!("{}:/readonly:ro", readonly.display()), format!("{}:/writable:rw", writable.display())];
                req.policy.volume_names = vec![ro.name.clone(), rw.name.clone()];
                let operation = OperationId::new("named-a-create").unwrap();
                let first = manager.create(req.clone(), &operation).await.unwrap();
                let replay = manager.create(req, &operation).await.unwrap();
                assert_eq!(first.execution_id, replay.execution_id);
                manager.start(&first.execution_id, first.generation).await.unwrap();
                let ro_id = storage_identity(&root, &first.execution_id, &readonly);
                let rw_id = storage_identity(&root, &first.execution_id, &writable);
                assert_ne!(ro_id, rw_id);
                behavior::readonly_probe(&manager, &root, &first.execution_id, first.generation).await;
                assert_eq!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/cat", "/readonly/live"])).await.unwrap().stdout, b"short");
                let live = b"host update after guest first read has a different length";
                fs::write(readonly.join("live"), live).unwrap();
                let observed = manager.execute(&first.execution_id, first.generation, exec(&["/bin/cat", "/readonly/live"])).await.unwrap();
                fs::write(root.join("named-live-observation.json"), serde_json::to_vec_pretty(&json!({"expected":String::from_utf8_lossy(live),"actual":String::from_utf8_lossy(&observed.stdout),"exit":observed.exit_code})).unwrap()).unwrap();
                assert_eq!(observed.exit_code, 0);
                assert_eq!(observed.stdout, live);
                let mut second_request = request(&image, "named-b");
                second_request.config.user = Some("0".into());
                second_request.config.volumes = vec![format!("{}:/data:rw", writable.display())];
                second_request.policy.volume_names = vec![rw.name.clone()];
                let second = manager.create_and_start(second_request, &OperationId::new("named-b-create").unwrap()).await.unwrap();
                assert_eq!(storage_identity(&root, &second.execution_id, &writable), rw_id);
                let references = volumes.get(&rw.name).unwrap().unwrap().in_use_by;
                assert!(references.contains(&first.execution_id.to_string()) && references.contains(&second.execution_id.to_string()));
                assert!(volumes.remove(&rw.name, false).is_err());
                assert_eq!(manager.execute(&second.execution_id, second.generation, exec(&["/bin/sh", "-c", "printf shared-data > /data/shared"])).await.unwrap().exit_code, 0);
                assert_eq!(fs::read(writable.join("shared")).unwrap(), b"shared-data");
                assert_eq!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/cat", "/writable/shared"])).await.unwrap().stdout, b"shared-data");
                let restarted = manager.restart(&first.execution_id, first.generation, &OperationId::new("named-normal-restart").unwrap()).await.unwrap();
                assert_eq!(restarted.generation.get(), first.generation.get() + 1);
                assert!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/sh", "-c", "echo stale > /writable/shared"])).await.is_err());
                assert_eq!(storage_identity(&root, &first.execution_id, &readonly), ro_id);
                assert_eq!(storage_identity(&root, &first.execution_id, &writable), rw_id);
                behavior::readonly_probe(&manager, &root, &first.execution_id, restarted.generation).await;
                manager.kill(&second.execution_id, second.generation).await.unwrap();
                manager.remove(&second.execution_id, second.generation).await.unwrap();
                assert_eq!(volumes.get(&rw.name).unwrap().unwrap().in_use_by, vec![first.execution_id.to_string()]);
                assert_eq!(fs::read(writable.join("shared")).unwrap(), b"shared-data");
                let cli_checked = cli(&root, &image);
                let record = ManagedExecutionStore::new(root.join("home/boxes.json")).get(&first.execution_id).unwrap().unwrap();
                let binding = record.managed_execution.unwrap().oci_runtime.unwrap();
                host.sigkill();
                assert!(manager.inspect(&first.execution_id).await.is_err());
                let recovery = root.join("service/runtime/recovery").join(format!("{}-{}.json", binding.target.id, binding.target.generation.unwrap().0));
                let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
                while !recovery.exists() && tokio::time::Instant::now() < deadline { tokio::time::sleep(Duration::from_millis(100)).await; }
                assert!(recovery.is_file());
                host = Host::start(&root, 2);
                let reconstructed = connect(&root, OciMigrationPolicy::LegacyOnly).await;
                assert_eq!(reconstructed.inspect(&first.execution_id).await.unwrap().state, ExecutionState::Stopped);
                let recovered = reconstructed.restart(&first.execution_id, restarted.generation, &OperationId::new("named-owner-restart").unwrap()).await.unwrap();
                assert_eq!(recovered.generation.get(), restarted.generation.get() + 1);
                assert_eq!(storage_identity(&root, &first.execution_id, &readonly), ro_id);
                assert_eq!(storage_identity(&root, &first.execution_id, &writable), rw_id);
                behavior::readonly_probe(&reconstructed, &root, &first.execution_id, recovered.generation).await;
                assert_eq!(reconstructed.execute(&first.execution_id, recovered.generation, exec(&["/bin/cat", "/writable/shared"])).await.unwrap().stdout, b"shared-data");
                reconstructed.kill(&first.execution_id, recovered.generation).await.unwrap();
                reconstructed.remove(&first.execution_id, recovered.generation).await.unwrap();
                fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","cliChecked":cli_checked,"namedStorage":[ro_id,rw_id],"liveData":true,"sharedReferences":true,"restart":true,"stoppedRecovery":true,"hostRoRepeated":3})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none() { host = Host::start(&root, 99); }
            cleanup(&root).await;
            let retained = volumes.list().unwrap();
            for volume in &retained { assert!(volume.in_use_by.is_empty()); }
            assert_eq!(fs::read(readonly.join("input")).unwrap(), b"caller-owned");
            let after = fs::metadata(readonly.join("input")).unwrap();
            assert_eq!((before.dev(),before.ino(),before.uid(),before.gid(),before.mode(),before.nlink()), (after.dev(),after.ino(),after.uid(),after.gid(),after.mode(),after.nlink()));
            fs::write(root.join("named-retained-after-runtime-cleanup.json"), serde_json::to_vec_pretty(&json!({"volumes":retained,"roSourceRetained":true,"roInputUnchanged":true,"liveHostBytes":String::from_utf8_lossy(&fs::read(readonly.join("live")).unwrap())})).unwrap()).unwrap();
            for volume in retained {
                assert!(["qual-named-ro","qual-named-rw","cli-named"].contains(&volume.name.as_str()));
                volumes.remove(&volume.name, false).unwrap();
            }
            assert!(volumes.list().unwrap().is_empty());
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
            fs::write(root.join("named-fixture-cleanup.json"), b"{\"volumes\":[],\"onlyExplicitTestVolumesRemoved\":true}\n").unwrap();
            if let Err(error) = result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
