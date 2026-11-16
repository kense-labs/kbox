use super::*;
use crate::vm::BoxState;
use std::sync::atomic::AtomicUsize;

struct RefuseFirstStop {
    calls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl crate::vmm::VmHandler for RefuseFirstStop {
    fn stop(&mut self, _signal: i32, _timeout_ms: u64) -> Result<()> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(BoxError::StateError("test pool finalizer refused".into()))
        } else {
            Ok(())
        }
    }
    fn metrics(&self) -> crate::vmm::VmMetrics {
        crate::vmm::VmMetrics::default()
    }
    fn is_running(&self) -> bool {
        true
    }
    fn has_exited(&self) -> bool {
        false
    }
    fn pid(&self) -> u32 {
        42
    }
}

impl Drop for RefuseFirstStop {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

async fn fixture() -> (
    tempfile::TempDir,
    VmManager,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let mut vm = VmManager::new(BoxConfig::default(), EventEmitter::new(16));
    vm.home_dir = root.path().join("home 空格");
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    std::fs::create_dir_all(box_dir.join("rootfs")).unwrap();
    std::fs::write(box_dir.join("rootfs/canary"), b"owned-working-data").unwrap();
    *vm.handler.write().await = Some(Box::new(RefuseFirstStop {
        calls: calls.clone(),
        drops: drops.clone(),
    }));
    *vm.state.write().await = BoxState::Ready;
    (root, vm, calls, drops)
}

#[tokio::test]
async fn published_pool_drain_retains_failed_manager_and_reports_failure_until_retry() {
    let (_root, vm, calls, drops) = fixture().await;
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    pool.idle.lock().await.push(WarmVm {
        vm,
        created_at: Instant::now(),
    });
    pool.stats.lock().await.idle_count = 1;

    let error = pool
        .drain_idle()
        .await
        .expect_err("unfinalized VM must not be reported clean");
    assert!(matches!(error, BoxError::StateError(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "the actual handler must remain owned"
    );
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    assert_eq!(
        pool.stats().await.idle_count,
        0,
        "failed finalization is not authenticated idle capacity"
    );
    pool.drain_idle().await.unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "retry must use the retained exact handler"
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn cancelled_guest_stop_delivery_keeps_the_actual_handler_for_retry() {
    let (root, mut vm, calls, drops) = fixture().await;
    let path = root.path().join("blocked-stop.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let (entered, observed) = tokio::sync::oneshot::channel();
    let (finish, finished) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        entered.send(()).unwrap();
        let _ = finished.await;
    });
    vm.exec_socket_path = Some(path);
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    let pool = Arc::new(
        WarmPool::start(
            PoolConfig {
                min_idle: 0,
                max_size: 1,
                ..PoolConfig::default()
            },
            BoxConfig::default(),
            EventEmitter::new(16),
        )
        .await
        .unwrap(),
    );
    pool.idle.lock().await.push(WarmVm {
        vm,
        created_at: Instant::now(),
    });
    let owner = pool.clone();
    let draining = tokio::spawn(async move { owner.drain_idle().await });
    tokio::time::timeout(Duration::from_secs(2), observed)
        .await
        .unwrap()
        .unwrap();
    draining.abort();
    assert!(draining.await.unwrap_err().is_cancelled());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "guest-control cancellation must not drop the actual handler"
    );
    let owner = pool.retiring.lock()[0].clone();
    {
        let mut vm = owner.lock().await;
        assert!(vm.handler.read().await.is_some());
        vm.exec_socket_path = None;
    }
    finish.send(()).unwrap();
    server.await.unwrap();
    assert!(matches!(
        pool.drain_idle().await,
        Err(BoxError::StateError(_))
    ));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

struct RefuseFirstRootfsCleanup {
    calls: Arc<AtomicUsize>,
    allocation: std::path::PathBuf,
}

impl crate::rootfs::RootfsProvider for RefuseFirstRootfsCleanup {
    fn prepare(
        &self,
        box_dir: &std::path::Path,
        _: &std::path::Path,
    ) -> Result<std::path::PathBuf> {
        Ok(box_dir.join("rootfs"))
    }
    fn cleanup(&self, box_dir: &std::path::Path, persistent: bool) -> Result<()> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(BoxError::StateError("test rootfs finalizer refused".into()));
        }
        crate::rootfs::CopyProvider.cleanup(box_dir, persistent)?;
        std::fs::remove_file(&self.allocation)?;
        Ok(())
    }
    fn name(&self) -> &'static str {
        "refuse-first-test-rootfs"
    }
}

#[tokio::test]
async fn stopped_vm_does_not_hide_unfinished_host_cleanup_or_wipe_its_working_data() {
    let (root, mut vm, calls, drops) = fixture().await;
    calls.store(1, Ordering::SeqCst);
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    let allocation = root.path().join("provider-allocation");
    std::fs::write(&allocation, b"still-owned").unwrap();
    let cleanups = Arc::new(AtomicUsize::new(0));
    let metrics = crate::prom::RuntimeMetrics::try_new().unwrap();
    vm.prom = Some(metrics.clone());
    vm.rootfs_provider = Arc::new(RefuseFirstRootfsCleanup {
        calls: cleanups.clone(),
        allocation: allocation.clone(),
    });
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    pool.idle.lock().await.push(WarmVm {
        vm,
        created_at: Instant::now(),
    });
    assert!(matches!(
        pool.drain_idle().await,
        Err(BoxError::StateError(_))
    ));
    assert_eq!(cleanups.load(Ordering::SeqCst), 1);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "provider stop actually succeeded"
    );
    assert!(allocation.exists());
    let owner = pool.retiring.lock()[0].clone();
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data",
        "refused rootfs cleanup must preserve working data"
    );
    pool.drain_idle().await.unwrap();
    assert_eq!(
        cleanups.load(Ordering::SeqCst),
        2,
        "Stopped is not proof of host cleanup"
    );
    assert!(!allocation.exists() && !box_dir.exists());
    owner.lock().await.destroy_with_timeout(0).await.unwrap();
    assert_eq!(
        cleanups.load(Ordering::SeqCst),
        2,
        "successful cleanup remains idempotent"
    );
    assert_eq!(
        metrics.vm_destroyed_total.get(),
        1,
        "cleanup retries must not duplicate workload stop metrics"
    );
}

#[tokio::test]
async fn ephemeral_close_retries_only_its_created_anonymous_allocations() {
    let (_root, mut vm, calls, _drops) = fixture().await;
    calls.store(1, Ordering::SeqCst);
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    let store = crate::volume::VolumeStore::new(
        vm.home_dir.join("volumes.json"),
        vm.home_dir.join("volumes"),
    );
    let (owned, created) = store.claim_anonymous("anon_created", vm.box_id()).unwrap();
    assert!(created);
    store.claim_anonymous("anon_reused", vm.box_id()).unwrap();
    let data = std::path::PathBuf::from(&owned.mount_point).join("canary");
    std::fs::write(&data, b"anonymous-data-retained-on-refusal").unwrap();
    let mut wrong_binding = owned.clone();
    wrong_binding
        .labels
        .insert("kbox.volume.owner".into(), "different-execution".into());
    store.update(&wrong_binding).unwrap();
    vm.created_anonymous_volumes = vec!["anon_created".into()];
    vm.anonymous_volumes = vec!["anon_created".into(), "anon_reused".into()];
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    pool.idle.lock().await.push(WarmVm {
        vm,
        created_at: Instant::now(),
    });
    let error = pool
        .drain_idle()
        .await
        .expect_err("anonymous owner mismatch must not become clean close");
    let retained = pool.retiring.lock()[0].clone();
    assert_eq!(
        retained.lock().await.created_anonymous_volumes,
        ["anon_created"],
        "failed cleanup must retain exact allocation identity for retry"
    );
    assert!(
        matches!(error, BoxError::ConfigError(_)),
        "preserve the typed volume refusal"
    );
    assert_eq!(
        std::fs::read(&data).unwrap(),
        b"anonymous-data-retained-on-refusal"
    );
    assert!(box_dir.exists() && store.get("anon_reused").unwrap().is_some());
    store.update(&owned).unwrap();
    pool.drain_idle().await.unwrap();
    assert!(store.get("anon_created").unwrap().is_none());
    assert!(!data.exists() && !box_dir.exists());
    assert!(
        store.get("anon_reused").unwrap().is_some(),
        "reused/caller allocations remain outside this cleanup"
    );
    assert!(retained.lock().await.created_anonymous_volumes.is_empty());
}

#[tokio::test]
async fn completed_preserving_close_can_still_remove_its_ephemeral_resources() {
    let (_root, mut vm, calls, _) = fixture().await;
    calls.store(1, Ordering::SeqCst);
    vm.rootfs_provider = Arc::new(crate::rootfs::CopyProvider);
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    let store = crate::volume::VolumeStore::new(
        vm.home_dir.join("volumes.json"),
        vm.home_dir.join("volumes"),
    );
    store
        .claim_anonymous("anon_preserved", vm.box_id())
        .unwrap();
    vm.created_anonymous_volumes = vec!["anon_preserved".into()];
    vm.anonymous_volumes = vec!["anon_preserved".into()];
    vm.destroy_preserving_rootfs().await.unwrap();
    assert_eq!(vm.state().await, BoxState::Stopped);
    assert!(box_dir.join("rootfs/canary").exists());
    assert!(store.get("anon_preserved").unwrap().is_some());
    vm.destroy_with_timeout(0).await.unwrap();
    assert!(!box_dir.exists());
    assert!(store.get("anon_preserved").unwrap().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn incomplete_persistent_handoff_does_not_become_clean_on_retry() {
    let (_root, mut vm, calls, _) = fixture().await;
    calls.store(1, Ordering::SeqCst);
    vm.config.persistent = true;
    vm.rootfs_provider = Arc::new(crate::rootfs::CopyProvider);
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    // Metadata-presence boundary only; this is not a bootable/native disk fixture.
    std::fs::create_dir(box_dir.join("rootfs-ext4-v1")).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            vm.destroy_with_timeout(0).await,
            Err(BoxError::StateError(_))
        ));
        assert_eq!(vm.state().await, BoxState::Stopped);
        assert_eq!(
            std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
            b"owned-working-data"
        );
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "handler finalized only once"
    );
}

#[tokio::test]
async fn shutdown_admission_refuses_idle_and_queued_cold_acquisition() {
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            max_concurrent_boots: 1,
            ..PoolConfig::default()
        },
        BoxConfig {
            image: "invalid-image@@@".into(),
            ..BoxConfig::default()
        },
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    let permit = pool.boot_limiter.acquire().await.unwrap();
    let pending = pool.boot_new_vm();
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut pending)
            .await
            .is_err()
    );
    pool.signal_shutdown();
    drop(permit);
    assert!(
        matches!(pending.await, Err(BoxError::PoolError(_))),
        "a queued boot must recheck shutdown before VM entry"
    );
    assert!(matches!(pool.acquire().await, Err(BoxError::PoolError(_))));
    pool.drain_idle().await.unwrap();
    assert_eq!(pool.stats().await.total_created, 0);
}
