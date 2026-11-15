//! Run the real Linux Guest enforcement regression inside the owned native VM.
use super::*;

#[test]
#[ignore = "requires sentinel-owned root, installed HVF assets and a fixed Linux Guest test binary"]
fn native_guest_readonly_guards_refuse_unenforced_root_before_exec_or_pty() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                let unit=PathBuf::from(std::env::var_os("K_BOX_TEST_READONLY_GUEST_TESTS").unwrap());
                assert!(unit.is_file() && unit.is_absolute());
                let init_unit=PathBuf::from(std::env::var_os("K_BOX_TEST_READONLY_GUEST_BIN_TESTS").unwrap());
                assert!(init_unit.is_absolute() && init_unit.is_file());
                let readonly=root.join("readonly-source");fs::create_dir(&readonly).unwrap();fs::write(readonly.join("canary"),b"readonly-source").unwrap();
                let mut req=request(&image,"readonly-guest-regression");req.config.cap_add=vec!["SYS_ADMIN".into()];req.config.volumes=vec![format!("{}:/guest-readonly-tests:ro",unit.display()),format!("{}:/guest-init-tests:ro",init_unit.display()),format!("{}:/readonly-root-source:ro",readonly.display())];
                let reserved=manager.create_and_start(req,&OperationId::new("readonly-guest-regression").unwrap()).await.unwrap();
                let mut command=exec(&["/guest-readonly-tests","readonly_rootfs","--nocapture"]);command.timeout_ns=30_000_000_000;
                let output=manager.execute(&reserved.execution_id,reserved.generation,command).await.unwrap();
                fs::write(root.join("guest-tests.stdout"),&output.stdout).unwrap();fs::write(root.join("guest-tests.stderr"),&output.stderr).unwrap();
                fs::write(root.join("guest-tests-exit.json"),serde_json::to_vec_pretty(&json!({"exit":output.exit_code,"execution":reserved.execution_id,"generation":reserved.generation,"binary":unit})).unwrap()).unwrap();
                assert_eq!(output.exit_code,0,"{}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
                let counts=String::from_utf8_lossy(&output.stdout);
                assert!(counts.contains("3 passed; 0 failed; 4 ignored;"));
                assert_eq!(counts.matches("1 passed; 0 failed; 0 ignored;").count(),2);
                for (label,binary,test) in [
                    ("already-ro","/guest-readonly-tests","exec_server::readonly_tests::readonly_rootfs_already_readonly_is_idempotent_without_mount_authority"),
                    ("enforcement","/guest-readonly-tests","exec_server::readonly_tests::readonly_rootfs_enforces_writes_preserves_inner_mounts_and_is_idempotent"),
                    ("init-refusal","/guest-init-tests","linux::tests::readonly_rootfs_init_refuses_without_mount_authority"),
                ] {
                    let mut command=exec(&[binary,"--ignored","--exact",test,"--nocapture"]);command.env=vec!["K_BOX_TEST_READONLY_RO_DIR=/readonly-root-source".into()];command.timeout_ns=30_000_000_000;
                    let output=manager.execute(&reserved.execution_id,reserved.generation,command).await.unwrap();
                    fs::write(root.join(format!("guest-{label}.stdout")),&output.stdout).unwrap();fs::write(root.join(format!("guest-{label}.stderr")),&output.stderr).unwrap();
                    assert_eq!(output.exit_code,0,"{label}: {}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
                    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed; 0 ignored;"));
                }
                manager.kill(&reserved.execution_id,reserved.generation).await.unwrap();manager.remove(&reserved.execution_id,reserved.generation).await.unwrap();
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","realLinuxGuestTests":true,"execPreparationRefused":true,"noWorkloadMarker":true,"ptyFramedRefusal":true,"initRefused":true,"readonlyIdempotent":true,"writableInnerMount":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
