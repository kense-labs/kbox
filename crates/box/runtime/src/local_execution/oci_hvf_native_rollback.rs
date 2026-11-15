//! Actual older CLI must not reinterpret a reserved file as a directory.
use super::*;
use koci_sdk::{ListRequest, RuntimeClient, RuntimeTransportClient};

fn state_bytes(path: &Path) -> Vec<u8> {
    fs::read(
        crate::BoxStateStore::load_readonly(path)
            .unwrap()
            .data_path()
            .unwrap(),
    )
    .unwrap()
}

#[test]
#[ignore = "requires sentinel-owned root, installed HVF assets and the accepted older Box CLI"]
fn native_old_cli_refuses_file_authority_without_activation_or_rewrite() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
        let old = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_OLD_CLI").unwrap());
        assert!(old.is_absolute() && old.is_file());
        let host = Host::start(&root, 1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let result = std::panic::AssertUnwindSafe(async {
                let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                let source = root.join("授权 file");
                fs::write(&source, b"original-authorized-file").unwrap();
                let mut req = request(&std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(), "rollback-file");
                req.config.volumes = vec![format!("{}:/exact-ro:ro", source.display())];
                let reserved = manager.create(req, &OperationId::new("rollback-file-create").unwrap()).await.unwrap();
                let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
                let before = state_bytes(store.path());
                fs::write(root.join("state-before-old-cli.json"), &before).unwrap();
                assert_eq!(store.get(&reserved.execution_id).unwrap().unwrap().managed_execution.unwrap().caller_files.len(), 1);
                let mut cli_adopted = false;
                if let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_ADOPT_CLI") {
                    // Materialize the exact previous flat format before any VM exists.
                    let fixture = root.join("fresh-store-fixture");
                    fs::rename(store.path(), &fixture).unwrap();
                    fs::write(store.path(), &before).unwrap();
                    let output = Command::new(binary).env("K_HOME", root.join("home"))
                        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
                        .env("K_BOX_OCI_MIGRATION", "off")
                        .args(["inspect", reserved.execution_id.as_str()]).output().unwrap();
                    fs::write(root.join("current-cli-adopt.stdout"), &output.stdout).unwrap();
                    fs::write(root.join("current-cli-adopt.stderr"), &output.stderr).unwrap();
                    assert!(output.status.success());
                    assert!(store.path().is_dir());
                    assert_eq!(before, state_bytes(store.path()));
                    fs::remove_file(fixture.join("records.v1.json")).unwrap();
                    fs::remove_dir(fixture).unwrap();
                    cli_adopted = true;
                }
                fs::rename(&source, root.join("original-retained")).unwrap();
                fs::create_dir(&source).unwrap();
                fs::write(source.join("canary"), b"directory-never-authorized").unwrap();
                let output = Command::new(&old).env("K_HOME", root.join("home"))
                    .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
                    .env("K_BOX_OCI_MIGRATION", "off")
                    .args(["start", reserved.execution_id.as_str()]).output().unwrap();
                fs::write(root.join("old-cli-start.stdout"), &output.stdout).unwrap();
                fs::write(root.join("old-cli-start.stderr"), &output.stderr).unwrap();
                let after = state_bytes(store.path());
                fs::write(root.join("state-after-old-cli.json"), &after).unwrap();
                let observed = store.get(&reserved.execution_id).unwrap().unwrap();
                let metadata = observed.managed_execution.as_ref().unwrap();
                let exposed = if metadata.oci_runtime.is_some() {
                    manager.execute(&reserved.execution_id, reserved.generation, exec(&["/bin/cat", "/exact-ro/canary"])).await.ok()
                        .is_some_and(|value| value.exit_code == 0 && value.stdout == b"directory-never-authorized")
                } else { false };
                fs::write(root.join("rollback-observation.json"), serde_json::to_vec_pretty(&json!({"oldCli":old,"oldStartExit":output.status.code(),"stateUnchanged":before==after,"callerFileBindings":metadata.caller_files.len(),"runtimeBinding":metadata.oci_runtime,"directoryExposed":exposed,"execution":reserved.execution_id,"generation":reserved.generation})).unwrap()).unwrap();
                assert!(!output.status.success(), "older CLI activated a file reservation as a directory");
                assert_eq!(before, after, "older reader discarded or rewrote file authority");
                assert!(!exposed && metadata.oci_runtime.is_none());
                for (name, args) in [
                    ("inspect", vec!["inspect", reserved.execution_id.as_str()]),
                    ("rename", vec!["rename", reserved.execution_id.as_str(), "must-not-rename"]),
                    ("run", vec!["run", "-d", "--cpus", "1", "--memory", "512", "--network", "none", &std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(), "--", "/bin/sleep", "2147483647"]),
                ] {
                    let output = Command::new(&old).env("K_HOME", root.join("home"))
                        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
                        .env("K_BOX_OCI_MIGRATION", "off").args(args).output().unwrap();
                    fs::write(root.join(format!("old-cli-{name}.stdout")), &output.stdout).unwrap();
                    fs::write(root.join(format!("old-cli-{name}.stderr")), &output.stderr).unwrap();
                    assert!(!output.status.success(), "old CLI {name} bypassed the store fence");
                    assert_eq!(before, state_bytes(store.path()));
                    assert!(store.path().is_dir());
                }
                let stream = tokio::net::UnixStream::connect(root.join("service/runtime.sock")).await.unwrap();
                let client = RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());
                assert!(client.list(ListRequest::default()).await.unwrap().is_empty());
                assert_eq!(fs::read(root.join("original-retained")).unwrap(), b"original-authorized-file");
                assert_eq!(fs::read(source.join("canary")).unwrap(), b"directory-never-authorized");
                fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","oldCliActivationRefused":true,"stateUnchanged":true,"cliAdopted":cli_adopted,"oldCommandsRefused":["start","inspect","rename","run"],"hostContainers":0})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
            if let Err(error) = result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
