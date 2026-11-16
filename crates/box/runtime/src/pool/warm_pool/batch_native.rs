use super::*;
use futures::FutureExt;
use std::path::PathBuf;

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_completed_batch_owner_survives_consumer_cancellation_and_actual_drain() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-batch-native")).unwrap(), "owned-pool-batch-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-batch").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let config = BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: kbox_core::NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            };
            let pool = Arc::new(WarmPool::start(PoolConfig { min_idle: 0, max_size: 1, ..PoolConfig::default() }, config, EventEmitter::new(16)).await.unwrap());
            let (prepared, observed) = tokio::sync::oneshot::channel();
            let (finish, finished) = tokio::sync::oneshot::channel();
            let source = pool.clone();
            let mut worker = Some(tokio::spawn(async move {
                let results = WarmPool::boot_batch(BootBatch {
                    snapshot_fork: false,
                    box_config: &source.box_config,
                    event_emitter: &source.event_emitter,
                    template: &source.template,
                    template_resources: &source.template_resources,
                    needed: 1,
                    max_concurrent_boots: 1,
                    metrics: None,
                    boot_limiter: source.boot_limiter.clone(),
                    global_boot_limiter: None,
                    retained: &source.retiring,
                    shutdown: &source.shutdown_tx,
                }).await;
                assert_eq!(results.len(), 1);
                let (owner, _) = results[0].as_ref().unwrap();
                let vm = owner.lock().await;
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                let metadata = serde_json::json!({
                    "execution": vm.box_id(), "pid": pid,
                    "socketDir": vm.socket_dir(),
                    "authenticatedReady": vm.state().await == crate::vm::BoxState::Ready,
                    "anonymousVolumes": vm.created_anonymous_volumes,
                });
                drop(vm);
                prepared.send(metadata).unwrap();
                let _ = finished.await;
                // An invariant failure keeps the worker available to finalize
                // its exact results before reporting the failure.
                for result in results {
                    let (owner, _handoff) = result.unwrap();
                    let (_, closed) = WarmPool::destroy_one(owner, source.retiring.clone(), None).await;
                    closed.unwrap();
                }
            }));
            let outcome = std::panic::AssertUnwindSafe(async {
                let metadata = tokio::time::timeout(Duration::from_secs(180), observed).await.unwrap().unwrap();
                assert_eq!(metadata["authenticatedReady"], true);
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !caller.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let count = pool.retiring.lock().len();
                std::fs::write(root.join("before-cancel.json"), serde_json::to_vec_pretty(&serde_json::json!({"actualVm":metadata,"registeredOwners":count,"callerBytes":std::fs::metadata(caller.join("writes")).unwrap().len()})).unwrap()).unwrap();
                assert_eq!(count, 1, "completed producer must register its actual VM before handing results to a cancellable consumer");
                let task = worker.take().unwrap();
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
                assert!(pool.ensure_admission_open().is_err());
                let owner = pool.retiring.lock()[0].clone();
                {
                    let vm = owner.lock().await;
                    let handler = vm.handler.read().await;
                    assert_eq!(handler.as_ref().unwrap().pid(), metadata["pid"].as_u64().unwrap() as u32);
                    assert!(handler.as_ref().unwrap().is_running());
                }
                let bytes = std::fs::metadata(caller.join("writes")).unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while std::fs::metadata(caller.join("writes")).unwrap().len() <= bytes {
                    assert!(tokio::time::Instant::now() < deadline, "retained VM is live, not falsely stopped");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                pool.drain_idle().await.unwrap();
                assert!(pool.retiring.lock().is_empty());
                assert!(owner.lock().await.handler.read().await.is_none());
                let bytes = std::fs::read(caller.join("writes")).unwrap();
                tokio::time::sleep(Duration::from_millis(500)).await;
                assert_eq!(std::fs::read(caller.join("writes")).unwrap(), bytes);
                assert_eq!(std::fs::read(caller.join("canary")).unwrap(), b"caller-owned-batch");
                std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","actualVm":metadata,"completedConsumerCancelled":true,"exactOwnerRetained":true,"liveUntilActualDrain":true,"retryActuallyStoppedVm":true,"noLateWrites":true,"callerBytesRetained":bytes.len(),"runtimeRssMeasured":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            let _ = finish.send(());
            if let Some(task) = worker.take() { task.await.unwrap(); }
            let cleanup = pool.drain_idle().await;
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let remaining = store.list().unwrap();
            let box_dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":box_dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-batch"})).unwrap()).unwrap();
            if let Err(error) = outcome { std::panic::resume_unwind(error); }
            cleanup.unwrap();
            assert!(remaining.is_empty() && box_dirs.is_empty());
        });
    }).unwrap().join().unwrap();
}
