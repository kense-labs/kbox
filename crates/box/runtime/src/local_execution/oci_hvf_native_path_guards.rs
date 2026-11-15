//! Exercise the real Guest path guard in an owned native VM.
use super::*;

#[path = "oci_hvf_native_path_guards_binary.rs"]
mod binary;

#[test]
#[ignore = "requires sentinel-owned root, installed HVF assets and a fixed Linux Guest test binary"]
fn native_guest_path_guards_enforce_before_exec_and_pty() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
        let host = Host::start(&root, 1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let result = std::panic::AssertUnwindSafe(async {
                let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                let binary = PathBuf::from(std::env::var_os("K_BOX_TEST_PATH_GUARD_GUEST_TESTS").unwrap());
                assert!(binary.is_absolute() && binary.is_file());
                let mut req = request(&image, "guest-path-guards");
                req.config.cap_add = vec!["SYS_ADMIN".into()];
                req.config.volumes = vec![format!("{}:/guest-path-tests:ro", binary.display())];
                let reserved = manager.create_and_start(req, &OperationId::new("guest-path-guards").unwrap()).await.unwrap();
                let mut failures = Vec::new();
                let mut receipts = Vec::new();
                for (label, args) in [
                    ("refusals", vec!["/guest-path-tests", "path_guard", "--nocapture"]),
                    ("lookalike", vec!["/guest-path-tests", "--ignored", "--exact", "exec_server::path_guards_tests::path_guard_does_not_trust_caller_readonly_tmpfs_with_writable_alias", "--nocapture"]),
                    ("mask-effects", vec!["/guest-path-tests", "--ignored", "--exact", "exec_server::path_guards_tests::path_guard_masks_files_and_directories_idempotently_without_fd_leaks", "--nocapture"]),
                    ("readonly-effects", vec!["/guest-path-tests", "--ignored", "--exact", "exec_server::path_guards_tests::path_guard_readonly_files_directories_and_nested_mounts_are_idempotent", "--nocapture"]),
                    ("mounted-target", vec!["/guest-path-tests", "--ignored", "--exact", "exec_server::path_guards_tests::path_guard_masks_preexisting_unrestricted_mount_instead_of_skipping_it", "--nocapture"]),
                ] {
                    let mut command = exec(&args); command.timeout_ns = 30_000_000_000;
                    let output = manager.execute(&reserved.execution_id, reserved.generation, command).await.unwrap();
                    fs::write(root.join(format!("guest-{label}.stdout")), &output.stdout).unwrap();
                    fs::write(root.join(format!("guest-{label}.stderr")), &output.stderr).unwrap();
                    receipts.push(json!({"case":label,"exitCode":output.exit_code,"execution":reserved.execution_id,"generation":reserved.generation}));
                    fs::write(root.join("guest-results.json"), serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
                    if output.exit_code != 0 { failures.push(format!("{label}: {}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))); }
                    else {
                        let text = String::from_utf8_lossy(&output.stdout);
                        if label == "refusals" {
                            assert!(text.contains("4 passed; 0 failed; 6 ignored;"));
                            assert_eq!(text.matches("1 passed; 0 failed; 0 ignored;").count(), 4);
                        } else { assert!(text.contains("1 passed; 0 failed; 0 ignored;")); }
                    }
                }
                assert!(failures.is_empty(), "{}", failures.join("\n"));
                manager.kill(&reserved.execution_id, reserved.generation).await.unwrap();
                manager.remove(&reserved.execution_id, reserved.generation).await.unwrap();
                fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","realLinuxGuestTests":true,"execPtyRefusedBeforeLaunch":true,"mountedTargetActuallyMasked":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await; drop(host); assert!(!root.join("service/runtime.sock").exists());
            if let Err(error) = result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
