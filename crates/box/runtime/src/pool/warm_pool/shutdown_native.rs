use super::*;
use crate::vm::BoxState;
use crate::vmm::{VmHandler, VmMetrics};
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

struct RefuseOnce {
    inner: Box<dyn VmHandler>,
    refused: bool,
}

impl VmHandler for RefuseOnce {
    fn stop(&mut self, signal: i32, timeout_ms: u64) -> Result<()> {
        if !std::mem::replace(&mut self.refused, true) {
            return Err(BoxError::StateError(
                "native pool test injected stop refusal".into(),
            ));
        }
        self.inner.stop(signal, timeout_ms)
    }
    fn metrics(&self) -> VmMetrics {
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

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and final installed companions"]
fn native_published_pool_retains_vm_across_cancel_refusal_and_actual_close_retry() {
    native_pool_owner_close(false);
}

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and final installed companions"]
fn native_foreground_release_retains_vm_across_cancel_refusal_and_actual_close_retry() {
    native_pool_owner_close(true);
}

fn native_pool_owner_close(foreground: bool) {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(move || {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-stop-native")).unwrap(), "owned-pool-shutdown-native-test\n");
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
            let pool = Arc::new(WarmPool::start(PoolConfig { min_idle: 0, max_size: 1, ..PoolConfig::default() }, config.clone(), EventEmitter::new(16)).await.unwrap());
            let directory = TemplateDirectory::create(&home.join("pool")).unwrap();
            let template_path = directory.path().to_path_buf();
            std::fs::write(template_path.join("template.ram"), b"owned-input-until-close").unwrap();
            let mut vm = VmManager::new(config, EventEmitter::new(16));
            vm.template_use = Some(directory.claim().unwrap());
            pool.template_resources.lock().await.directory = Some(directory);
            let id = vm.box_id().to_owned();
            let box_dir = home.join("boxes").join(&id);
            let socket_dir = vm.socket_dir();
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let mut source = Some(vm);
            let blocked = root.join("blocked-stop.sock");
            let listener = tokio::net::UnixListener::bind(&blocked).unwrap();
            let (entered, mut observed) = tokio::sync::oneshot::channel();
            let (finish, finished) = tokio::sync::oneshot::channel();
            let blocker = tokio::spawn(async move {
                let mut finished = finished;
                tokio::select! {
                    accepted = listener.accept() => {
                        let (_stream, _) = accepted.unwrap();
                        let _ = entered.send(());
                        let _ = finished.await;
                    }
                    _ = &mut finished => {}
                }
            });
            let mut draining = None;
            let outcome = std::panic::AssertUnwindSafe(async {
                source.as_mut().unwrap().boot().await.unwrap();
                if foreground {
                    pool.release(source.take().unwrap()).await.unwrap();
                    assert_eq!(pool.stats().await.idle_count, 1);
                    let acquired = pool.acquire().await.unwrap();
                    assert_eq!(acquired.box_id(), id);
                    assert_eq!(pool.stats().await.total_acquired, 1);
                    assert_eq!(pool.stats().await.total_released, 1);
                    source = Some(acquired);
                }
                let vm = source.as_mut().unwrap();
                assert_eq!(vm.state().await, BoxState::Ready);
                assert!(vm.exec_client().is_some());
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                assert_eq!(vm.created_anonymous_volumes.len(), 1);
                let name = vm.created_anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, &id).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (std::fs::metadata(&volume).unwrap().dev(), std::fs::metadata(&volume).unwrap().ino());
                let before = serde_json::to_value(&allocation).unwrap();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !caller.join("writes").is_file() || !volume.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                std::fs::write(root.join("before-fault.json"), serde_json::to_vec_pretty(&serde_json::json!({"execution":id,"pid":pid,"socketDir":socket_dir,"allocation":before,"volumeIdentity":identity,"authenticated":true})).unwrap()).unwrap();
                let actual = vm.handler.write().await.take().unwrap();
                *vm.handler.write().await = Some(Box::new(RefuseOnce { inner: actual, refused: false }));
                // Only the test control endpoint is withheld; the actual Guest keeps writing.
                vm.exec_socket_path = Some(blocked.clone());
                if foreground {
                    vm.exec_client = None;
                    let vm = source.take().unwrap();
                    let owner = pool.clone();
                    draining = Some(tokio::spawn(async move { owner.release(vm).await }));
                } else {
                    let mut idle = pool.idle.lock().await;
                    idle.push(WarmVm { vm: source.take().unwrap(), created_at: Instant::now() });
                    drop(idle);
                    let owner = pool.clone();
                    draining = Some(tokio::spawn(async move { owner.drain_idle().await }));
                }
                tokio::time::timeout(Duration::from_secs(5), &mut observed).await.unwrap().unwrap();
                let task = draining.take().unwrap();
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
                let owner = {
                    let owners = pool.retiring.lock();
                    assert_eq!(owners.len(), 1);
                    owners[0].clone()
                };
                {
                    let mut vm = owner.lock().await;
                    let handler = vm.handler.read().await;
                    assert_eq!(handler.as_ref().unwrap().pid(), pid);
                    assert!(handler.as_ref().unwrap().is_running());
                    drop(handler);
                    vm.exec_socket_path = None;
                    vm.exec_client = None;
                }
                let writes = std::fs::metadata(caller.join("writes")).unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while std::fs::metadata(caller.join("writes")).unwrap().len() <= writes {
                    assert!(tokio::time::Instant::now() < deadline, "actual VM must remain live after cancellation");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let (refused, owner) = if foreground {
                    let vm = pool.take_foreground_vm(owner).unwrap();
                    let refused = pool.release(vm).await;
                    let owner = pool.retiring.lock()[0].clone();
                    (refused, owner)
                } else {
                    (pool.drain_idle().await, owner)
                };
                assert!(matches!(refused, Err(BoxError::StateError(_))));
                assert_eq!(pool.retiring.lock().len(), 1);
                assert_eq!(store.require_anonymous(&name, &id).map(|v| serde_json::to_value(v).unwrap()).unwrap(), before);
                assert_eq!((std::fs::metadata(&volume).unwrap().dev(), std::fs::metadata(&volume).unwrap().ino()), identity);
                assert_eq!(std::fs::read(template_path.join("template.ram")).unwrap(), b"owned-input-until-close");
                let writes = std::fs::metadata(caller.join("writes")).unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while std::fs::metadata(caller.join("writes")).unwrap().len() <= writes {
                    assert!(tokio::time::Instant::now() < deadline, "synthetic stop refusal must retain live VM");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                pool.drain_idle().await.unwrap();
                assert!(pool.retiring.lock().is_empty());
                assert!(owner.lock().await.handler.read().await.is_none());
                assert!(!box_dir.exists() && !socket_dir.exists() && !template_path.exists() && !volume.exists());
                assert!(store.get(&name).unwrap().is_none());
                let writes = std::fs::read(caller.join("writes")).unwrap();
                tokio::time::sleep(Duration::from_millis(500)).await;
                assert_eq!(std::fs::read(caller.join("writes")).unwrap(), writes);
                assert_eq!(std::fs::read(caller.join("canary")).unwrap(), b"caller-owned-retained");
                std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","execution":id,"pid":pid,"socketDir":socket_dir,"templatePath":template_path,"foregroundRelease":foreground,"healthyReleaseAndAcquireProved":foreground,"cancelledHealthWait":foreground,"cancelledGuestControlWait":!foreground,"syntheticOneShotStopError":true,"actualOsStopFailureClaimed":false,"liveWritesAfterCancellationAndRefusal":true,"exactOwnerAndAllocationRetained":true,"retryActuallyStoppedVm":true,"templateRetainedUntilBackendFinalization":true,"nativeSnapshotRestoreClaimed":false,"noLateWrites":true,"callerBytesRetained":writes.len(),"callerCanaryRetained":true,"runtimeRssMeasured":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            if let Some(task) = draining.take() { task.abort(); let _ = task.await; }
            let _ = finish.send(());
            blocker.await.unwrap();
            std::fs::remove_file(&blocked).unwrap();
            if let Some(vm) = source.as_mut() { vm.destroy_with_timeout(0).await.unwrap(); }
            // Bounded recovery of the deliberately refused attempt on assertion failure.
            let mut cleanup = pool.drain_idle().await;
            if cleanup.is_err() { cleanup = pool.drain_idle().await; }
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"boxDirectoryAbsent":!box_dir.exists(),"socketDirectoryAbsent":!socket_dir.exists(),"templateDirectoryAbsent":!template_path.exists(),"remainingVolumes":store.list().unwrap()})).unwrap()).unwrap();
            if let Err(error) = outcome { std::panic::resume_unwind(error); }
            cleanup.unwrap();
        });
    }).unwrap().join().unwrap();
}
