use super::*;
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_prepared_startup_owner_survives_cancelled_first_ready_fill_and_drain() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-startup-native")).unwrap(), "owned-pool-startup-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-pool-startup").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let config = BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: kbox_core::NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            };
            let pool = Arc::new(WarmPool::prepare_with_metrics_and_boot_limiter(
                PoolConfig { min_idle: 1, max_size: 1, ..PoolConfig::default() }, config, EventEmitter::new(16), None, None,
            ).unwrap());
            assert!(pool.replenish_handle.lock().await.is_none());
            let mut stats = Some(pool.stats.clone().lock_owned().await);
            let owner = pool.clone();
            let mut worker = Some(tokio::spawn(async move { owner.initialize_first_ready().await }));
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let outcome = std::panic::AssertUnwindSafe(async {
                let actual = tokio::time::timeout(Duration::from_secs(180), async {
                    loop {
                        let actual = pool.retiring.lock().first().cloned();
                        if let Some(actual) = actual {
                            let vm = actual.lock().await;
                            if vm.state().await == crate::vm::BoxState::Ready && vm.handler.read().await.is_some() {
                                drop(vm); break actual;
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }).await.unwrap();
                assert_eq!(pool.retiring.lock().len(), 1);
                let vm = actual.lock().await;
                let id = vm.box_id().to_owned();
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                let handler = vm.handler.clone();
                assert_eq!(vm.created_anonymous_volumes.len(), 1);
                let name = vm.created_anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, &id).unwrap();
                let saved = serde_json::to_value(&allocation).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino());
                drop(vm);
                let before = serde_json::json!({"execution":id,"pid":pid,"allocation":saved,"volumeIdentity":identity,"callerOwnedPoolPreparedBeforeAwait":true});
                std::fs::write(root.join("before-cancel.json"), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                assert_eq!(stats.as_ref().unwrap().idle_count, 0);
                tokio::time::timeout(Duration::from_secs(5), async {
                    while pool.idle.try_lock().is_ok() { tokio::task::yield_now().await; }
                }).await.unwrap();
                assert!(pool.idle.try_lock().is_err(), "initial fill must be awaiting publication while caller still owns its pool");
                let task = worker.take().unwrap();
                task.abort();
                assert!(matches!(task.await, Err(error) if error.is_cancelled()));
                drop(stats.take());
                assert!(pool.replenish_handle.lock().await.is_none());
                assert!(pool.ensure_admission_open().is_err());
                assert!(matches!(pool.initialize_first_ready().await, Err(BoxError::PoolError(_))));
                assert_eq!(pool.retiring.lock().len(), 1);
                assert!(Arc::ptr_eq(&pool.retiring.lock()[0], &actual));
                assert_eq!(handler.read().await.as_ref().unwrap().pid(), pid);
                assert!(handler.read().await.as_ref().unwrap().is_running());
                assert_eq!(serde_json::to_value(store.require_anonymous(&name, &id).unwrap()).unwrap(), saved);
                assert_eq!((volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino()), identity);
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !volume.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let bytes = volume.join("writes").metadata().unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while volume.join("writes").metadata().unwrap().len() <= bytes {
                    assert!(tokio::time::Instant::now() < deadline, "same prepared-owner VM must remain live after initializer cancellation");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                serde_json::json!({"status":"PASS","before":before,"preparedPoolOwnerSurvivedCancellation":true,"sameActualPidAndAllocationPreserved":true,"guestContinuedWriting":true,"admissionClosed":true,"closedOwnerInitializationRefused":true,"runtimeRssMeasured":false})
            }).catch_unwind().await;
            drop(stats.take());
            // Release the publication wait and await a still-owning initializer on failure.
            if let Some(worker) = worker.take() { let _ = worker.await; }
            let cleanup = pool.drain_idle().await;
            let remaining = store.list().unwrap();
            let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-pool-startup"})).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    cleanup.unwrap(); assert!(remaining.is_empty() && dirs.is_empty() && pool.retiring.lock().is_empty());
                    let bytes = std::fs::read(caller.join("writes")).unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    assert_eq!(std::fs::read(caller.join("writes")).unwrap(), bytes);
                    proof["productPhysicallyFinalized"] = true.into();
                    proof["noLateWritesAfterOwnedCleanup"] = true.into();
                    proof["callerBytesRetained"] = bytes.len().into();
                    std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                }
                Err(error) => std::panic::resume_unwind(error),
            }
        });
    }).unwrap().join().unwrap();
}
