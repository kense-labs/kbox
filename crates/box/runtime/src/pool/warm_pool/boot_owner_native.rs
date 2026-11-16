use super::*;
use crate::vmm::{InstanceSpec, VmController, VmHandler, VmmProvider};
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::Mutex as StdMutex;

struct LaunchGate {
    inner: VmController,
    entered: StdMutex<Option<tokio::sync::oneshot::Sender<()>>>,
    proceed: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
    returned: StdMutex<Option<tokio::sync::oneshot::Sender<(String, u32)>>>,
}

#[async_trait::async_trait]
impl VmmProvider for LaunchGate {
    async fn start(&self, spec: &InstanceSpec) -> Result<Box<dyn VmHandler>> {
        self.entered
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(())
            .unwrap();
        let proceed = self.proceed.lock().await.take().unwrap();
        proceed.await.map_err(|_| {
            BoxError::StateError("owned launch gate cancelled before acquisition".into())
        })?;
        let handler = self.inner.start(spec).await?;
        // No new await/fallible operation may hold the returned actual handler here.
        let _ = self
            .returned
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send((spec.box_id.clone(), handler.pid()));
        Ok(handler)
    }
}

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_in_progress_pool_boot_retains_actual_vm_across_cancellation_and_drain() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-boot-native")).unwrap(), "owned-pool-boot-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-pool-boot").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let config = BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: kbox_core::NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            };
            let pool = Arc::new(WarmPool::start(PoolConfig { min_idle: 0, max_size: 1, ..PoolConfig::default() }, config, EventEmitter::new(16)).await.unwrap());
            if let Some(task) = pool.replenish_handle.lock().await.take() { task.abort(); let _ = task.await; }
            let mut vm = VmManager::new(pool.box_config.clone(), EventEmitter::new(16));
            let id = vm.box_id().to_owned();
            let slot = vm.handler.clone();
            let state = vm.state.clone();
            let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
            let (proceed_tx, proceed_rx) = tokio::sync::oneshot::channel();
            let (returned_tx, returned_rx) = tokio::sync::oneshot::channel();
            vm.provider = Some(Box::new(LaunchGate {
                inner: VmController::new(VmController::find_shim().unwrap()).unwrap(),
                entered: StdMutex::new(Some(entered_tx)), proceed: Mutex::new(Some(proceed_rx)), returned: StdMutex::new(Some(returned_tx)),
            }));
            let owner = pool.clone();
            let mut task = Some(tokio::spawn(async move { WarmPool::boot_cold_vm(vm, &owner.template_resources, &owner.retiring, &owner.shutdown_tx).await }));
            let mut proceed = Some(proceed_tx);
            let mut destination = None;
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let outcome = std::panic::AssertUnwindSafe(async {
                tokio::time::timeout(Duration::from_secs(60), entered_rx).await.unwrap().unwrap();
                assert_eq!(pool.retiring.lock().len(), 1, "register before provider entry, never cancel an unregistered native VM");
                destination = Some(slot.clone().write_owned().await);
                proceed.take().unwrap().send(()).unwrap();
                let (actual_id, pid) = tokio::time::timeout(Duration::from_secs(30), returned_rx).await.unwrap().unwrap();
                assert_eq!(actual_id, id);
                assert_eq!(pool.retiring.lock().len(), 1);
                let worker = task.take().unwrap();
                worker.abort();
                assert!(matches!(worker.await, Err(error) if error.is_cancelled()));
                drop(destination.take());
                let owner = pool.retiring.lock()[0].clone();
                let vm = owner.lock().await;
                assert_eq!(vm.box_id(), id);
                assert!(Arc::ptr_eq(&vm.handler, &slot));
                assert_eq!(*state.read().await, crate::vm::BoxState::Created, "cancellation must not manufacture Ready");
                assert!(!vm.cleanup_is_complete());
                assert_eq!(vm.created_anonymous_volumes.len(), 1);
                let name = vm.created_anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, &id).unwrap();
                let saved = serde_json::to_value(&allocation).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino());
                drop(vm);
                let command = std::process::Command::new("/bin/ps").args(["-p", &pid.to_string(), "-o", "command="]).output().unwrap();
                assert!(command.status.success());
                let command = String::from_utf8(command.stdout).unwrap();
                assert!(command.contains("kbox-shim --config") && command.contains(&id));
                let before = serde_json::json!({"execution":id,"pid":pid,"allocation":saved,"volumeIdentity":identity,"command":command});
                std::fs::write(root.join("before-cancel.json"), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                assert!(pool.ensure_admission_open().is_err());
                assert_eq!(pool.stats().await.idle_count, 0);
                let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
                while !volume.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline, "retained boot VM must start its actual Guest workload");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let bytes = volume.join("writes").metadata().unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while volume.join("writes").metadata().unwrap().len() <= bytes {
                    assert!(tokio::time::Instant::now() < deadline, "same retained native Guest must continue writing");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                assert_eq!(serde_json::to_value(store.require_anonymous(&name, &id).unwrap()).unwrap(), saved);
                assert_eq!((volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino()), identity);
                assert!(home.join("boxes").join(&id).is_dir());
                serde_json::json!({"status":"PASS","before":before,"actualInProgressOwnerPreserved":true,"sameActualPidAndAllocationPreserved":true,"guestContinuedWriting":true,"admissionClosed":true,"cancelledBootNeverReportedReady":true,"runtimeRssMeasured":false})
            }).catch_unwind().await;
            drop(destination.take());
            if let Some(proceed) = proceed.take() { let _ = proceed.send(()); }
            // A failed pre-cancel assertion leaves its still-owning worker running to completion.
            if let Some(worker) = task.take() { let _ = worker.await; }
            let cleanup = pool.drain_idle().await;
            let remaining = store.list().unwrap();
            let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-pool-boot"})).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    cleanup.unwrap();
                    assert!(remaining.is_empty() && dirs.is_empty() && pool.retiring.lock().is_empty());
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

#[test]
#[ignore = "requires owned sentinel home, pinned image and installed companions; refusal case launches zero VMs"]
fn native_queued_template_shutdown_refuses_before_actual_provider_entry() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-admission-native")).unwrap(), "owned-pool-admission-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-pool-admission").unwrap();
        let metadata = caller.join("canary").metadata().unwrap();
        let identity = (metadata.dev(), metadata.ino());
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let pool = Arc::new(WarmPool::start(PoolConfig { min_idle: 0, max_size: 1, ..PoolConfig::default() }, BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(), network: kbox_core::NetworkMode::None,
                volumes: vec![format!("{}:/owned:rw", caller.display())], ..BoxConfig::default()
            }, EventEmitter::new(16)).await.unwrap());
            if let Some(task) = pool.replenish_handle.lock().await.take() { task.abort(); let _ = task.await; }
            let mut vm = VmManager::new(pool.box_config.clone(), EventEmitter::new(16));
            let (entered_tx, mut entered_rx) = tokio::sync::oneshot::channel();
            let (proceed_tx, proceed_rx) = tokio::sync::oneshot::channel();
            let (returned_tx, mut returned_rx) = tokio::sync::oneshot::channel();
            vm.provider = Some(Box::new(LaunchGate {
                inner: VmController::new(VmController::find_shim().unwrap()).unwrap(),
                entered: StdMutex::new(Some(entered_tx)), proceed: Mutex::new(Some(proceed_rx)), returned: StdMutex::new(Some(returned_tx)),
            }));
            let mut resources = Some(pool.template_resources.lock().await);
            let mut proceed = Some(proceed_tx);
            let owner = pool.clone();
            let mut worker = Some(tokio::spawn(async move { WarmPool::boot_cold_vm(vm, &owner.template_resources, &owner.retiring, &owner.shutdown_tx).await }));
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let outcome = std::panic::AssertUnwindSafe(async {
                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let actual = pool.retiring.lock().first().cloned();
                        if actual.is_some_and(|actual| actual.try_lock().is_err()) { break; }
                        tokio::task::yield_now().await;
                    }
                }).await.unwrap();
                pool.signal_shutdown();
                drop(resources.take());
                let completion = tokio::time::timeout(Duration::from_secs(5), worker.as_mut().unwrap()).await.unwrap();
                drop(worker.take());
                let result = completion.unwrap();
                assert!(matches!(result, Err(BoxError::PoolError(_))), "queued shutdown must refuse before actual provider entry");
                assert!(pool.retiring.lock().is_empty());
                assert!(matches!(entered_rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)), "provider must never be entered");
                assert!(matches!(returned_rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)));
                assert!(pool.ensure_admission_open().is_err());
                assert_eq!(pool.stats().await.total_created, 0);
                assert!(store.list().unwrap().is_empty());
                assert!(!caller.join("writes").exists());
                serde_json::json!({"status":"PASS","prelaunchRefusalOnly":true,"actualVmLaunches":0,"providerEntered":false,"queuedShutdownRefused":true,"admissionClosed":true,"preparedOwnerFinalized":true,"noWorkloadWrites":true,"callerIdentity":identity,"runtimeRssMeasured":false})
            }).catch_unwind().await;
            drop(resources.take());
            // Any unexpected provider entry stays BEFORE real start; dropping this gate refuses it.
            drop(proceed.take());
            if let Some(worker) = worker.take() { let _ = worker.await; }
            let cleanup = pool.drain_idle().await;
            let remaining = store.list().unwrap();
            let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-pool-admission","actualVmLaunches":0})).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    cleanup.unwrap(); assert!(remaining.is_empty() && dirs.is_empty());
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let metadata = caller.join("canary").metadata().unwrap();
                    assert_eq!((metadata.dev(), metadata.ino()), identity);
                    assert_eq!(std::fs::read(caller.join("canary")).unwrap(), b"caller-owned-pool-admission");
                    assert!(!caller.join("writes").exists());
                    proof["noLateWorkloadWrites"] = true.into();
                    std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                }
                Err(error) => std::panic::resume_unwind(error),
            }
        });
    }).unwrap().join().unwrap();
}
