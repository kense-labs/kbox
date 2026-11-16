use super::*;
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_rollback_preserves_unrelated_vm_and_allocation_across_idle_mutation() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-rollback-native")).unwrap(), "owned-pool-rollback-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-rollback").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let config = BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: kbox_core::NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            };
            let pool = Arc::new(WarmPool::start(PoolConfig { min_idle: 0, max_size: 2, ..PoolConfig::default() }, config, EventEmitter::new(16)).await.unwrap());
            if let Some(task) = pool.replenish_handle.lock().await.take() { task.abort(); let _ = task.await; }
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let mut direct = Vec::new();
            let mut pending = None;
            let mut acquired = None;
            let mut target_state = None;
            let outcome = std::panic::AssertUnwindSafe(async {
                let results = WarmPool::boot_batch(BootBatch {
                    snapshot_fork: false, box_config: &pool.box_config,
                    event_emitter: &pool.event_emitter, template: &pool.template,
                    template_resources: &pool.template_resources, needed: 2,
                    max_concurrent_boots: 1, metrics: None,
                    boot_limiter: pool.boot_limiter.clone(), global_boot_limiter: None,
                    retained: &pool.retiring, shutdown: &pool.shutdown_tx,
                }).await;
                assert_eq!(results.len(), 2);
                for result in results {
                    let (owner, _handoff) = result.unwrap();
                    direct.push(pool.take_foreground_vm(owner).unwrap());
                }
                let id_b = direct[0].box_id().to_owned();
                let id_c = direct[1].box_id().to_owned();
                let handler_b = direct[0].handler.clone();
                let handler_c = direct[1].handler.clone();
                let pid_b = handler_b.read().await.as_ref().unwrap().pid();
                let pid_c = handler_c.read().await.as_ref().unwrap().pid();
                assert_eq!(direct[0].created_anonymous_volumes.len(), 1);
                assert_eq!(direct[1].created_anonymous_volumes.len(), 1);
                let name_b = direct[0].created_anonymous_volumes[0].clone();
                let name_c = direct[1].created_anonymous_volumes[0].clone();
                let allocation_b = store.require_anonymous(&name_b, &id_b).unwrap();
                let allocation_c = store.require_anonymous(&name_c, &id_c).unwrap();
                let saved_c = serde_json::to_value(&allocation_c).unwrap();
                let volume_c = PathBuf::from(&allocation_c.mount_point);
                let identity_c = (volume_c.metadata().unwrap().dev(), volume_c.metadata().unwrap().ino());
                let dir_b = home.join("boxes").join(&id_b);
                let dir_c = home.join("boxes").join(&id_c);
                let before = serde_json::json!({"target": {"execution":id_b,"pid":pid_b,"allocation":allocation_b},"unrelated": {"execution":id_c,"pid":pid_c,"allocation":saved_c,"volumeIdentity":identity_c}});
                std::fs::write(root.join("before-rollback.json"), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !volume_c.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                target_state = Some(direct[0].state.clone().write_owned().await);
                let mut idle = pool.idle.lock().await;
                pending = Some(direct.pop().unwrap());
                idle.push(WarmVm { vm: direct.pop().unwrap(), created_at: Instant::now() });
                pool.stats.lock().await.idle_count = 1;
                let selected = [id_b.clone()];
                let rollback = pool.remove_idle_vms(&selected);
                tokio::pin!(rollback);
                assert!(futures::poll!(rollback.as_mut()).is_pending());
                let change = async {
                    let mut members = pool.idle.lock().await;
                    let mut stats = pool.stats.lock().await;
                    let removed = members.pop().map(|warm| warm.vm);
                    members.push(WarmVm { vm: pending.take().unwrap(), created_at: Instant::now() });
                    stats.idle_count = members.len();
                    removed
                };
                tokio::pin!(change);
                assert!(futures::poll!(change.as_mut()).is_pending());
                drop(idle);
                assert!(futures::poll!(rollback.as_mut()).is_pending());
                acquired = change.await;
                drop(target_state.take());
                rollback.await;
                assert!(dir_c.is_dir() && volume_c.is_dir(), "unrelated native VM and allocation must survive rollback");
                assert_eq!((volume_c.metadata().unwrap().dev(), volume_c.metadata().unwrap().ino()), identity_c);
                assert_eq!(serde_json::to_value(store.require_anonymous(&name_c, &id_c).unwrap()).unwrap(), saved_c);
                assert_eq!(handler_c.read().await.as_ref().unwrap().pid(), pid_c);
                assert!(handler_c.read().await.as_ref().unwrap().is_running());
                assert!(acquired.is_none(), "atomic rollback wins its first queued critical section");
                assert!(handler_b.read().await.is_none());
                assert!(!dir_b.exists() && store.get(&name_b).unwrap().is_none());
                assert_eq!(pool.stats().await.idle_count, 1);
                let bytes = volume_c.join("writes").metadata().unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while volume_c.join("writes").metadata().unwrap().len() <= bytes {
                    assert!(tokio::time::Instant::now() < deadline, "unrelated native Guest must continue writing");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                serde_json::json!({"status":"PASS","before":before,"targetActuallyStopped":true,"unrelatedActualPidAndAllocationPreserved":true,"unrelatedGuestContinuedWriting":true,"runtimeRssMeasured":false})
            }).catch_unwind().await;
            drop(target_state.take());
            for vm in direct.drain(..).chain(pending.take()).chain(acquired.take()) {
                let (owner, handoff) = pool.retain_foreground_vm(vm);
                drop(owner);
                drop(handoff);
            }
            let cleanup = pool.drain_idle().await;
            let remaining = store.list().unwrap();
            let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-rollback"})).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    cleanup.unwrap();
                    assert!(remaining.is_empty() && dirs.is_empty());
                    let bytes = std::fs::read(caller.join("writes")).unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    assert_eq!(std::fs::read(caller.join("writes")).unwrap(), bytes);
                    proof["noLateWritesAfterOwnedCleanup"] = true.into();
                    proof["callerBytesRetained"] = bytes.len().into();
                    std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                }
                Err(error) => std::panic::resume_unwind(error),
            }
        });
    }).unwrap().join().unwrap();
}
