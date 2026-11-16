use super::*;
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_cancelled_drain_stats_wait_preserves_exact_vm_until_actual_retry() {
    run_cancelled_drain(false);
}

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_cancelled_maintenance_join_preserves_worker_and_vm_until_retry() {
    run_cancelled_drain(true);
}

fn run_cancelled_drain(join: bool) {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(move || {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-drain-native")).unwrap(), "owned-pool-drain-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-drain").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let mut cases = Vec::new();
            for shared in [false, true] {
                let config = BoxConfig {
                    image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                    network: kbox_core::NetworkMode::None,
                    cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                    volumes: vec![format!("{}:/owned:rw", caller.display())],
                    ..BoxConfig::default()
                };
                let mut pool = WarmPool::start(PoolConfig { min_idle: 0, max_size: 1, ..PoolConfig::default() }, config, EventEmitter::new(16)).await.unwrap();
                if let Some(task) = pool.replenish_handle.lock().await.take() { task.abort(); let _ = task.await; }
                let mut direct = None;
                let mut finish = None;
                let outcome = std::panic::AssertUnwindSafe(async {
                    let mut results = WarmPool::boot_batch(BootBatch {
                        snapshot_fork: false, box_config: &pool.box_config,
                        event_emitter: &pool.event_emitter, template: &pool.template,
                        template_resources: &pool.template_resources, needed: 1,
                        max_concurrent_boots: 1, metrics: None,
                        boot_limiter: pool.boot_limiter.clone(), global_boot_limiter: None,
                        retained: &pool.retiring, shutdown: &pool.shutdown_tx,
                    }).await;
                    assert_eq!(results.len(), 1);
                    let (owner, _handoff) = results.pop().unwrap().unwrap();
                    direct = Some(pool.take_foreground_vm(owner).unwrap());
                    let vm = direct.as_ref().unwrap();
                    let id = vm.box_id().to_owned();
                    let handler = vm.handler.clone();
                    let pid = handler.read().await.as_ref().unwrap().pid();
                    assert_eq!(vm.created_anonymous_volumes.len(), 1);
                    let name = vm.created_anonymous_volumes[0].clone();
                    let allocation = store.require_anonymous(&name, &id).unwrap();
                    let saved = serde_json::to_value(&allocation).unwrap();
                    let volume = PathBuf::from(&allocation.mount_point);
                    let identity = (volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino());
                    let box_dir = home.join("boxes").join(&id);
                    let before = serde_json::json!({"operation":if shared {"drain_idle"} else {"drain"},"execution":id,"pid":pid,"allocation":saved,"volumeIdentity":identity});
                    std::fs::write(root.join(format!("before-drain-{shared}.json")), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    while !volume.join("writes").is_file() {
                        assert!(tokio::time::Instant::now() < deadline);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    if join {
                        let (owner, handoff) = pool.retain_foreground_vm(direct.take().unwrap());
                        let retained = pool.retiring.clone();
                        let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
                        finish = Some(finish_tx);
                        *pool.replenish_handle.lock().await = Some(tokio::spawn(async move {
                            finish_rx.await.unwrap();
                            WarmPool::destroy_one(owner, retained, Some(2000)).await.1.unwrap();
                            drop(handoff);
                        }));
                    } else {
                        pool.idle.lock().await.push(WarmVm { vm: direct.take().unwrap(), created_at: Instant::now() });
                        pool.stats.lock().await.idle_count = 1;
                    }
                    let stats_arc = pool.stats.clone();
                    let stats = if join { None } else { Some(stats_arc.lock().await) };
                    {
                        let drain = async { if shared { pool.drain_idle().await } else { pool.drain().await } };
                        tokio::pin!(drain);
                        assert!(futures::poll!(drain.as_mut()).is_pending());
                    }
                    assert!(pool.ensure_admission_open().is_err());
                    if join {
                        assert!(pool.replenish_handle.lock().await.is_some(), "cancelled join must keep its actual worker handle");
                        let owners = pool.retiring.lock().clone();
                        assert_eq!(owners.len(), 1);
                        let actual = owners[0].lock().await;
                        assert_eq!(actual.box_id(), id);
                        assert!(Arc::ptr_eq(&actual.handler, &handler));
                        assert_eq!(pool.stats().await.idle_count, 0);
                    } else {
                        assert!(pool.retiring.lock().is_empty());
                        assert_eq!(stats.as_ref().unwrap().idle_count, 1);
                        let idle = pool.idle.lock().await;
                        assert_eq!(idle.len(), 1);
                        assert_eq!(idle[0].vm.box_id(), id);
                        assert!(Arc::ptr_eq(&idle[0].vm.handler, &handler));
                    }
                    assert_eq!(handler.read().await.as_ref().unwrap().pid(), pid);
                    assert!(handler.read().await.as_ref().unwrap().is_running());
                    assert!(box_dir.is_dir());
                    assert_eq!(serde_json::to_value(store.require_anonymous(&name, &id).unwrap()).unwrap(), saved);
                    assert_eq!((volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino()), identity);
                    let bytes = volume.join("writes").metadata().unwrap().len();
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    while volume.join("writes").metadata().unwrap().len() <= bytes {
                        assert!(tokio::time::Instant::now() < deadline, "owned native Guest must continue writing after cancelled drain");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    drop(stats);
                    if join {
                        {
                            let retry = async { if shared { pool.drain_idle().await } else { pool.drain().await } };
                            tokio::pin!(retry);
                            assert!(futures::poll!(retry.as_mut()).is_pending(), "retry cannot report drained while maintenance still owns the live VM");
                            assert_eq!(handler.read().await.as_ref().unwrap().pid(), pid);
                            assert!(handler.read().await.as_ref().unwrap().is_running());
                            assert!(volume.is_dir() && box_dir.is_dir());
                            finish.take().unwrap().send(()).unwrap();
                            retry.await.unwrap();
                        }
                        assert!(pool.replenish_handle.lock().await.is_none());
                        assert!(handler.read().await.is_none());
                    }
                    serde_json::json!({"before":before,"exactIdleOwnerPreserved":!join,"exactMaintenanceOwnerPreserved":join,"retryWaitedForActualMaintenance":join,"sameActualPidAndAllocationPreserved":true,"guestContinuedWriting":true,"admissionClosed":true})
                }).catch_unwind().await;
                if let Some(finish) = finish.take() { let _ = finish.send(()); }
                if let Some(vm) = direct.take() {
                    let (owner, handoff) = pool.retain_foreground_vm(vm);
                    drop(owner);
                    drop(handoff);
                }
                let cleanup = if shared { pool.drain_idle().await } else { pool.drain().await };
                let remaining = store.list().unwrap();
                let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
                std::fs::write(root.join(format!("native-cleanup-{shared}.json")), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-drain"})).unwrap()).unwrap();
                match outcome {
                    Ok(mut proof) => {
                        cleanup.unwrap();
                        assert!(remaining.is_empty() && dirs.is_empty() && pool.retiring.lock().is_empty());
                        let bytes = std::fs::read(caller.join("writes")).unwrap();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        assert_eq!(std::fs::read(caller.join("writes")).unwrap(), bytes);
                        proof["actualRetryFinalized"] = true.into();
                        proof["noLateWritesAfterOwnedCleanup"] = true.into();
                        proof["callerBytesRetained"] = bytes.len().into();
                        cases.push(proof);
                    }
                    Err(error) => std::panic::resume_unwind(error),
                }
            }
            std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","cases":cases,"runtimeRssMeasured":false})).unwrap()).unwrap();
        });
    }).unwrap().join().unwrap();
}
