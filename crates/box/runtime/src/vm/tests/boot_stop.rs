use super::*;

struct InjectedStopFailure {
    inner: Box<dyn VmHandler>,
    remaining: usize,
}

impl VmHandler for InjectedStopFailure {
    fn stop(&mut self, signal: i32, timeout_ms: u64) -> Result<()> {
        if self.remaining != 0 {
            self.remaining -= 1;
            return Err(BoxError::StateError("injected stop refusal".into()));
        }
        self.inner.stop(signal, timeout_ms)
    }

    fn metrics(&self) -> crate::vmm::VmMetrics {
        self.inner.metrics()
    }
    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
    fn has_exited(&self) -> bool {
        self.inner.has_exited()
    }
    fn pid(&self) -> u32 {
        self.inner.pid()
    }
    fn exit_code(&self) -> Option<i32> {
        self.inner.exit_code()
    }
    fn try_wait_exit(&mut self) -> Result<Option<i32>> {
        self.inner.try_wait_exit()
    }
}

async fn fixture(
    handler: Box<dyn VmHandler>,
) -> (tempfile::TempDir, VmManager, crate::volume::VolumeStore) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home 空格");
    let id = uuid::Uuid::new_v4().to_string();
    let mut vm = VmManager::with_box_id(BoxConfig::default(), EventEmitter::new(16), id.clone());
    vm.home_dir = home.clone();
    *vm.handler.write().await = Some(handler);
    let box_dir = home.join("boxes").join(&id);
    std::fs::create_dir_all(box_dir.join("logs")).unwrap();
    std::fs::create_dir(box_dir.join("rootfs")).unwrap();
    std::fs::write(box_dir.join("rootfs/canary"), b"working-data").unwrap();
    std::fs::write(box_dir.join("logs/console.log"), b"failed-boot-log").unwrap();
    std::fs::write(root.path().join("caller-canary"), b"caller-owned").unwrap();
    let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    for name in ["anon_created", "anon_reused"] {
        store.claim_anonymous(name, &id).unwrap();
    }
    vm.created_anonymous_volumes = vec!["anon_created".into()];
    vm.anonymous_volumes = vec!["anon_created".into(), "anon_reused".into()];
    (root, vm, store)
}

#[tokio::test]
async fn failed_boot_stop_refusal_preserves_the_exact_handler_data_and_allocations() {
    let stopped = Arc::new(AtomicBool::new(false));
    let (root, mut vm, store) = fixture(Box::new(InjectedStopFailure {
        inner: Box::new(RecordingHandler {
            stopped: stopped.clone(),
        }),
        remaining: usize::MAX,
    }))
    .await;
    let before = serde_json::to_value(store.get("anon_created").unwrap()).unwrap();
    let handler_slot = vm.handler.clone();
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(3), vm.cleanup_boot_failure())
            .await
            .unwrap();
        assert!(
            box_dir.exists(),
            "an unfinalized backend must retain working data"
        );
        assert_eq!(
            std::fs::read(box_dir.join("rootfs/canary")).unwrap(),
            b"working-data"
        );
        assert!(Arc::ptr_eq(&handler_slot, &vm.handler));
        assert!(vm.handler.read().await.as_ref().unwrap().is_running());
        assert_eq!(vm.created_anonymous_volumes, ["anon_created"]);
        assert_eq!(
            serde_json::to_value(store.get("anon_created").unwrap()).unwrap(),
            before
        );
        assert!(store.get("anon_reused").unwrap().is_some());
        assert!(!stopped.load(Ordering::SeqCst));
        assert_eq!(vm.state().await, BoxState::Created);
        assert_eq!(vm.exit_code(), None);
        assert_eq!(
            std::fs::read(root.path().join("caller-canary")).unwrap(),
            b"caller-owned"
        );
    }
}

#[tokio::test]
async fn failed_boot_cleanup_retries_the_retained_handler_before_releasing_resources() {
    let stopped = Arc::new(AtomicBool::new(false));
    let (_root, mut vm, store) = fixture(Box::new(InjectedStopFailure {
        inner: Box::new(RecordingHandler {
            stopped: stopped.clone(),
        }),
        remaining: 1,
    }))
    .await;
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    vm.cleanup_boot_failure().await;
    assert!(!stopped.load(Ordering::SeqCst));
    assert!(
        vm.handler.read().await.is_some(),
        "the same owner must be retryable"
    );
    assert!(store.get("anon_created").unwrap().is_some());
    assert_eq!(
        std::fs::read(
            vm.home_dir
                .join("diag/failed-boots")
                .join(vm.box_id())
                .join("logs/console.log")
        )
        .unwrap(),
        b"failed-boot-log"
    );
    vm.cleanup_boot_failure().await;
    assert!(stopped.load(Ordering::SeqCst));
    assert!(vm.handler.read().await.is_none());
    assert!(!box_dir.exists());
    assert!(store.get("anon_created").unwrap().is_none());
    assert!(store.get("anon_reused").unwrap().is_some());
}

#[tokio::test]
async fn exited_provider_still_retains_owner_when_backend_finalization_refuses() {
    let (_root, mut vm, store) = fixture(Box::new(InjectedStopFailure {
        inner: Box::new(CompletedHandler { code: 137 }),
        remaining: 1,
    }))
    .await;
    let box_dir = vm.home_dir.join("boxes").join(vm.box_id());
    vm.cleanup_boot_failure().await;
    assert!(
        vm.handler.read().await.is_some(),
        "process exit does not finalize backend resources"
    );
    assert!(box_dir.exists());
    assert!(store.get("anon_created").unwrap().is_some());
    vm.cleanup_boot_failure().await;
    assert!(vm.handler.read().await.is_none());
    assert!(store.get("anon_created").unwrap().is_none());
    assert_eq!(vm.exit_code(), Some(137));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "requires sentinel-owned home, VOLUME image and installed Box companions"]
fn native_failed_stop_preserves_live_vm_then_retries_real_finalization() {
    use futures::FutureExt;
    use std::os::unix::fs::MetadataExt;

    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-boot-stop-native")).unwrap(), "owned-boot-stop-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-retained").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let config = BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: kbox_core::NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            };
            let mut vm = VmManager::new(config, EventEmitter::new(16));
            assert_eq!(vm.home_dir, home);
            let id = vm.box_id().to_owned();
            let box_dir = home.join("boxes").join(&id);
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let result = std::panic::AssertUnwindSafe(async {
                vm.boot().await.unwrap();
                assert_eq!(vm.state().await, BoxState::Ready);
                assert!(vm.exec_client().is_some());
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                let socket_dir = vm.socket_dir().to_path_buf();
                assert_eq!(vm.created_anonymous_volumes.len(), 1, "real image VOLUME must allocate");
                let name = vm.created_anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, &id).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (std::fs::metadata(&volume).unwrap().dev(), std::fs::metadata(&volume).unwrap().ino());
                let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                while !caller.join("writes").is_file() || !volume.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline, "actual Guest must write both grants");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                let before = serde_json::to_value(&allocation).unwrap();
                std::fs::write(box_dir.join("logs/injected-stop.log"), b"stop-error-keeps-owner").unwrap();
                std::fs::write(root.join("before-fault.json"), serde_json::to_vec_pretty(&serde_json::json!({"execution":id,"pid":pid,"socketDir":socket_dir,"allocation":before,"volumeIdentity":identity,"authenticated":true})).unwrap()).unwrap();
                let original = vm.handler.write().await.take().unwrap();
                *vm.handler.write().await = Some(Box::new(InjectedStopFailure { inner: original, remaining: 1 }));
                vm.cleanup_boot_failure().await;
                assert_eq!(vm.state().await, BoxState::Ready);
                assert!(box_dir.exists() && socket_dir.exists());
                let handler = vm.handler.read().await;
                assert_eq!(handler.as_ref().unwrap().pid(), pid);
                assert!(handler.as_ref().unwrap().is_running());
                drop(handler);
                assert_eq!(vm.created_anonymous_volumes, [name.clone()]);
                assert_eq!(serde_json::to_value(store.require_anonymous(&name, &id).unwrap()).unwrap(), before);
                assert_eq!((std::fs::metadata(&volume).unwrap().dev(), std::fs::metadata(&volume).unwrap().ino()), identity);
                let retained = home.join("diag/failed-boots").join(&id).join("logs/injected-stop.log");
                assert_eq!(std::fs::read(retained).unwrap(), b"stop-error-keeps-owner");
                let writes = std::fs::metadata(caller.join("writes")).unwrap().len();
                let anonymous_writes = std::fs::metadata(volume.join("writes")).unwrap().len();
                let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                while std::fs::metadata(caller.join("writes")).unwrap().len() <= writes || std::fs::metadata(volume.join("writes")).unwrap().len() <= anonymous_writes {
                    assert!(tokio::time::Instant::now() < deadline, "injected refusal must leave the actual VM running");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                std::fs::write(root.join("after-refusal.json"), serde_json::to_vec_pretty(&serde_json::json!({"pid":pid,"liveAfterInjectedStopError":true,"sameAllocation":true,"sameDirectoryIdentity":identity,"workloadWroteAfterRefusal":true,"sourceRetained":true})).unwrap()).unwrap();
                vm.cleanup_boot_failure().await;
                assert!(vm.handler.read().await.is_none());
                assert!(!box_dir.exists() && !socket_dir.exists());
                assert!(store.get(&name).unwrap().is_none());
                assert!(!volume.exists());
                let writes = std::fs::read(caller.join("writes")).unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                assert_eq!(std::fs::read(caller.join("writes")).unwrap(), writes);
                assert_eq!(std::fs::read(caller.join("canary")).unwrap(), b"caller-owned-retained");
                std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","execution":id,"pid":pid,"socketDir":socket_dir,"syntheticOneShotStopError":true,"actualOsStopFailureClaimed":false,"liveWritesAfterRefusal":true,"exactOwnerAndAllocationRetained":true,"retryActuallyStoppedVm":true,"boxDirectoryAbsent":true,"socketDirectoryAbsent":true,"anonymousVolumeRemoved":name,"noLateWrites":true,"callerBytesRetained":writes.len(),"callerCanaryRetained":true,"runtimeRssMeasured":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            let cleanup = vm.destroy_with_timeout(0).await;
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"boxDirectoryAbsent":!box_dir.exists(),"socketDirectoryAbsent":!vm.socket_dir().exists(),"remainingVolumes":store.list().unwrap()})).unwrap()).unwrap();
            if let Err(error) = result { std::panic::resume_unwind(error); }
            cleanup.unwrap();
        });
    }).unwrap().join().unwrap();
}
