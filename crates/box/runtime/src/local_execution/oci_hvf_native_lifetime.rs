//! Product ownership must outlive the standalone diagnostic deadline.
use super::*;

#[test]
#[ignore = "requires sentinel-owned root, installed HVF assets and 95 seconds"]
fn native_box_hvf_owner_outlives_smoke_deadline() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
            assert_eq!(root.canonicalize().unwrap(), root);
            assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
            let host = Host::start(&root, 1);
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let result = std::panic::AssertUnwindSafe(async {
                        let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                        let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                        let execution = manager.create_and_start(request(&image, "owner-lifetime"), &OperationId::new("owner-lifetime-create").unwrap()).await.unwrap();
                        let start = std::time::Instant::now();
                        let before = manager.execute(&execution.execution_id, execution.generation, exec(&["/bin/echo", "before-deadline"])).await.unwrap();
                        assert_eq!((before.exit_code, before.stdout), (0, b"before-deadline\n".to_vec()));
                        tokio::time::sleep(Duration::from_secs(95)).await;
                        let after = manager.execute(&execution.execution_id, execution.generation, exec(&["/bin/echo", "after-deadline"])).await.unwrap();
                        assert_eq!((after.exit_code, after.stdout), (0, b"after-deadline\n".to_vec()));
                        assert!(start.elapsed() > Duration::from_secs(90));
                        fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","ownerLifetimeOnly":true,"execution":execution.execution_id,"generation":execution.generation,"elapsedSeconds":start.elapsed().as_secs_f64()})).unwrap()).unwrap();
                    }).catch_unwind().await;
                    cleanup(&root).await;
                    if let Err(error) = result { std::panic::resume_unwind(error); }
                });
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
        })
        .unwrap()
        .join()
        .unwrap();
}
