//! A failed native admission retains authority until exact Create reconciliation.
use super::*;
use kbox_core::{ExecutionGeneration, ExecutionManagerError};
use std::os::unix::fs::MetadataExt;

#[test]
#[ignore = "requires sentinel-owned root and installed HVF assets; withdraws entitlement on an owned copy"]
fn native_failed_create_preserves_authority_and_reconciles_owned_resources() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
        let installed = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_SHIM").unwrap());
        let negative = root.join("no-hypervisor-entitlement");
        fs::create_dir(&negative).unwrap();
        let shim = negative.join("koci-kshim");
        fs::copy(&installed, &shim).unwrap();
        let libraries = negative.join("koci-kbox-runtime");
        fs::create_dir(&libraries).unwrap();
        for name in ["libkrun.1.17.0.dylib", "libkrunfw.5.dylib"] {
            fs::copy(installed.parent().unwrap().join("koci-kbox-runtime").join(name), libraries.join(name)).unwrap();
        }
        let entitlements = negative.join("empty.plist");
        fs::write(&entitlements, b"<?xml version=\"1.0\"?><plist version=\"1.0\"><dict/></plist>").unwrap();
        assert!(Command::new("/usr/bin/codesign").args(["--force", "--sign", "-", "--entitlements"]).arg(&entitlements).arg(&shim).status().unwrap().success());
        let inspect = Command::new("/usr/bin/codesign").args(["-d", "--entitlements", "-"]).arg(&shim).output().unwrap();
        assert!(inspect.status.success());
        assert!(!String::from_utf8_lossy(&inspect.stdout).contains("com.apple.security.hypervisor"));
        fs::write(root.join("negative-shim-entitlements.log"), [&inspect.stdout[..], &inspect.stderr[..]].concat()).unwrap();
        let host = Host::start_with_shim(&root, 1, &shim);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
            let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
            let mut req = request(&image, "failed-create");
            files::configure(&root, "failed", &mut req);
            let operation = OperationId::new("failed-create-operation").unwrap();
            assert!(matches!(manager.create_and_start(req, &operation).await, Err(ExecutionManagerError::Unavailable(_))));
            let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
            let records = store.list().unwrap();
            assert_eq!(records.len(), 1);
            let id = kbox_core::ExecutionId::new(records[0].id.clone()).unwrap();
            let metadata = records[0].managed_execution.as_ref().unwrap();
            assert_eq!(metadata.caller_files.len(), 2);
            assert!(metadata.oci_runtime.is_none());
            if let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_CLI") {
                let output = Command::new(binary)
                    .env("K_HOME", root.join("home"))
                    .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
                    .env("K_BOX_OCI_MIGRATION", "off")
                    .args(["kill", id.as_str()]).output().unwrap();
                fs::write(root.join("unbound-kill-cli.stderr"), output.stderr).unwrap();
                assert!(!output.status.success());
            } else {
                assert!(matches!(manager.kill(&id, ExecutionGeneration::INITIAL).await, Err(ExecutionManagerError::Conflict { .. })));
            }
            assert_eq!(store.get(&id).unwrap().unwrap().status, crate::ManagedExecutionState::Killing.as_status());
            assert!(manager.remove(&id, ExecutionGeneration::INITIAL).await.is_err());
            drop(manager);
            drop(host);
            let host = Host::start(&root, 2);
            assert_eq!(pending::drain(&root).await, 1);
            let proof: serde_json::Value = serde_json::from_slice(&fs::read(root.join("pending-create-cleanup.json")).unwrap()).unwrap();
            assert_eq!(proof["records"][0]["createResumed"], true);
            let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
            assert_eq!(manager.kill(&id, ExecutionGeneration::INITIAL).await.unwrap(), kbox_core::KillOutcome::AlreadyStopped);
            assert!(manager.remove(&id, ExecutionGeneration::INITIAL).await.unwrap());
            let parent = root.join("caller-files-failed");
            for (name, bytes) in [("只读 input", b"initial-file".as_slice()), ("empty rw", b"".as_slice()), ("canary", b"private-sibling".as_slice())] {
                let path = parent.join(name);
                assert_eq!(fs::read(&path).unwrap(), bytes);
                assert_eq!(fs::metadata(path).unwrap().nlink(), 1);
            }
            cleanup(&root).await;
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
            fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","failedCreateCleanupOnly":true,"unboundKillRefused":true,"reservationRetained":true,"exactCreateReconciled":true,"callerFilesRetained":true,"workloadStartsAdded":0})).unwrap()).unwrap();
        });
    }).unwrap().join().unwrap();
}
