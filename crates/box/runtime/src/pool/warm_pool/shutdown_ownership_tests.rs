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

#[tokio::test]
async fn foreground_release_retains_the_exact_owner_after_stop_refusal() {
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
    let error = pool
        .release(vm)
        .await
        .expect_err("failed close must refuse release");
    assert!(matches!(error, BoxError::StateError(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "release must retain the actual handler after refused stop"
    );
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    assert_eq!(pool.retiring.lock().len(), 1);
    assert_eq!(pool.stats().await.total_released, 0);
    assert_eq!(pool.stats().await.idle_count, 0);
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn foreground_acquire_stop_refusal_closes_admission_without_launching_a_replacement() {
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
    assert!(matches!(pool.acquire().await, Err(BoxError::StateError(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(pool.retiring.lock().len(), 1);
    assert_eq!(pool.stats().await.total_acquired, 0);
    assert_eq!(pool.stats().await.total_created, 0);
    assert_eq!(pool.stats().await.idle_count, 0);
    assert!(pool.ensure_admission_open().is_err());
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn cancelled_foreground_stats_wait_keeps_the_original_idle_member_and_count() {
    let (_root, vm, calls, drops) = fixture().await;
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
    pool.stats.lock().await.idle_count = 1;
    let stats = pool.stats.lock().await;
    let owner = pool.clone();
    let waiting = tokio::spawn(async move { owner.acquire().await.map(drop) });
    tokio::time::timeout(Duration::from_secs(2), async {
        while pool.idle.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
    assert_eq!(stats.idle_count, 1);
    assert_eq!(pool.idle.lock().await.len(), 1);
    assert!(pool.retiring.lock().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(stats);
    assert!(pool.drain_idle().await.is_err());
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancelled_foreground_health_wait_retains_idle_acquire_and_release_owners() {
    for acquire in [false, true] {
        let (_root, vm, calls, drops) = fixture().await;
        let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
        let state = vm.state.clone();
        let state_guard = state.write().await;
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
        let owner = pool.clone();
        let waiting = if acquire {
            pool.idle.lock().await.push(WarmVm {
                vm,
                created_at: Instant::now(),
            });
            pool.stats.lock().await.idle_count = 1;
            tokio::spawn(async move { owner.acquire().await.map(drop) })
        } else {
            tokio::spawn(async move { owner.release(vm).await })
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            while pool.retiring.lock().is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        drop(state_guard);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            0,
            "cancelled health must preserve the actual handler"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(
            pool.ensure_admission_open().is_err(),
            "cancelled owner transfer must close admission before more VMs can be launched"
        );
        assert_eq!(pool.stats().await.idle_count, 0);
        assert_eq!(pool.stats().await.total_acquired, 0);
        assert_eq!(pool.stats().await.total_released, 0);
        assert_eq!(
            std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
            b"owned-working-data"
        );
        let close = pool.drain_idle().await;
        assert!(
            matches!(close, Err(BoxError::StateError(_))),
            "acquire={acquire}: {close:?}, stop calls={}",
            calls.load(Ordering::SeqCst)
        );
        pool.drain_idle().await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(!box_dir.exists());
    }
}

#[tokio::test]
async fn foreground_transfer_requires_exclusive_actual_ownership_and_preserves_retry() {
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
    let (owner, _handoff) = pool.retain_foreground_vm(vm);
    let closing = owner.clone();
    assert!(matches!(
        pool.take_foreground_vm(owner),
        Err(BoxError::PoolError(_))
    ));
    assert_eq!(pool.retiring.lock().len(), 1);
    assert!(Arc::ptr_eq(&pool.retiring.lock()[0], &closing));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(pool.ensure_admission_open().is_err());
    drop(closing);
    assert!(pool.drain_idle().await.is_err());
    pool.drain_idle().await.unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn exclusive_foreground_transfer_returns_the_same_actual_manager() {
    let (_root, vm, calls, drops) = fixture().await;
    let id = vm.box_id().to_owned();
    let box_dir = vm.home_dir.join("boxes").join(&id);
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
    let (owner, _handoff) = pool.retain_foreground_vm(vm);
    let mut transferred = pool.take_foreground_vm(owner).unwrap();
    assert!(pool.retiring.lock().is_empty());
    assert_eq!(transferred.box_id(), id);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(pool.ensure_admission_open().is_ok());
    assert!(transferred.destroy().await.is_err());
    transferred.destroy().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn selected_maintenance_cleanup_does_not_finalize_a_foreground_owner() {
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
    let (owner, _handoff) = pool.retain_foreground_vm(vm);
    WarmPool::destroy_vms(
        Vec::new(),
        &pool.retiring,
        None,
        "test selection",
        PoolCloseScope::Selected,
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(Arc::ptr_eq(&pool.retiring.lock()[0], &owner));
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    drop(owner);
    assert!(pool.drain_idle().await.is_err());
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn completed_batch_task_output_drop_keeps_the_registered_actual_owner() {
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
    let completed = WarmPool::register_vm(vm, &pool.retiring, &pool.shutdown_tx);
    let produced = tokio::spawn(async move { completed });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !produced.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(produced);
    tokio::time::timeout(Duration::from_secs(2), async {
        while pool.ensure_admission_open().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(pool.retiring.lock().len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
        b"owned-working-data"
    );
    assert!(pool.drain_idle().await.is_err());
    pool.drain_idle().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!box_dir.exists());
}

#[tokio::test]
async fn completed_batch_vector_drop_preserves_every_actual_manager_until_retry() {
    let (_root1, vm1, calls1, drops1) = fixture().await;
    let (_root2, vm2, calls2, drops2) = fixture().await;
    let dirs = [
        vm1.home_dir.join("boxes").join(vm1.box_id()),
        vm2.home_dir.join("boxes").join(vm2.box_id()),
    ];
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 2,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    let results: Vec<Result<CompletedBoot>> = vec![
        Ok(WarmPool::register_vm(
            vm1,
            &pool.retiring,
            &pool.shutdown_tx,
        )),
        Ok(WarmPool::register_vm(
            vm2,
            &pool.retiring,
            &pool.shutdown_tx,
        )),
    ];
    drop(results);
    assert!(pool.ensure_admission_open().is_err());
    assert_eq!(pool.retiring.lock().len(), 2);
    assert_eq!(
        (drops1.load(Ordering::SeqCst), drops2.load(Ordering::SeqCst)),
        (0, 0)
    );
    for dir in &dirs {
        assert_eq!(
            std::fs::read(dir.join("rootfs/canary")).unwrap(),
            b"owned-working-data"
        );
    }
    assert!(pool.drain_idle().await.is_err());
    assert_eq!(pool.retiring.lock().len(), 2);
    pool.drain_idle().await.unwrap();
    assert_eq!(
        (calls1.load(Ordering::SeqCst), calls2.load(Ordering::SeqCst)),
        (2, 2)
    );
    assert_eq!(
        (drops1.load(Ordering::SeqCst), drops2.load(Ordering::SeqCst)),
        (1, 1)
    );
    assert!(dirs.iter().all(|dir| !dir.exists()));
}

#[tokio::test]
async fn completed_batch_closed_admission_refuses_before_waiting_on_boot_permits() {
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            max_concurrent_boots: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap();
    let permit = pool.boot_limiter.clone().acquire_owned().await.unwrap();
    pool.signal_shutdown();
    let results = tokio::time::timeout(
        Duration::from_secs(2),
        WarmPool::boot_batch(BootBatch {
            snapshot_fork: false,
            box_config: &pool.box_config,
            event_emitter: &pool.event_emitter,
            template: &pool.template,
            template_resources: &pool.template_resources,
            needed: 1,
            max_concurrent_boots: 1,
            metrics: None,
            boot_limiter: pool.boot_limiter.clone(),
            global_boot_limiter: None,
            retained: &pool.retiring,
            shutdown: &pool.shutdown_tx,
        }),
    )
    .await
    .unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], Err(BoxError::PoolError(_))));
    assert!(pool.retiring.lock().is_empty());
    assert_eq!(pool.stats().await.total_created, 0);
    drop(permit);
    pool.drain_idle().await.unwrap();
}

#[tokio::test]
async fn completed_batch_queued_boot_rechecks_admission_after_its_permit() {
    let pool = Arc::new(
        WarmPool::start(
            PoolConfig {
                min_idle: 0,
                max_size: 1,
                max_concurrent_boots: 1,
                ..PoolConfig::default()
            },
            BoxConfig::default(),
            EventEmitter::new(16),
        )
        .await
        .unwrap(),
    );
    let permit = pool.boot_limiter.clone().acquire_owned().await.unwrap();
    let source = pool.clone();
    let produced = tokio::spawn(async move {
        WarmPool::boot_batch(BootBatch {
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
        })
        .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !produced.is_finished(),
        "the worker must actually be queued behind the held permit"
    );
    pool.signal_shutdown();
    drop(permit);
    let results = tokio::time::timeout(Duration::from_secs(2), produced)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], Err(BoxError::PoolError(_))));
    assert!(pool.retiring.lock().is_empty());
    assert_eq!(pool.stats().await.total_created, 0);
    pool.drain_idle().await.unwrap();
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
