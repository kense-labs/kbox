use super::*;
use futures::FutureExt;
use std::os::unix::fs::MetadataExt;

struct EscrowHandler {
    actual: Arc<parking_lot::Mutex<Box<dyn VmHandler>>>,
    stops: Arc<AtomicUsize>,
}

impl VmHandler for EscrowHandler {
    fn stop(&mut self, signal: i32, timeout: u64) -> Result<()> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        self.actual.lock().stop(signal, timeout)
    }
    fn metrics(&self) -> crate::vmm::VmMetrics {
        self.actual.lock().metrics()
    }
    fn is_running(&self) -> bool {
        self.actual.lock().is_running()
    }
    fn has_exited(&self) -> bool {
        self.actual.lock().has_exited()
    }
    fn pid(&self) -> u32 {
        self.actual.lock().pid()
    }
    fn exit_code(&self) -> Option<i32> {
        self.actual.lock().exit_code()
    }
    fn try_wait_exit(&mut self) -> Result<Option<i32>> {
        self.actual.lock().try_wait_exit()
    }
}

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_legacy_returned_vm_handler_survives_cancel_and_actual_cleanup() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-handler-publication-native")).unwrap(), "owned-handler-publication-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-handler-publication").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let mut vm = VmManager::new(BoxConfig {
                image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network: NetworkMode::None,
                cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()],
                volumes: vec![format!("{}:/owned:rw", caller.display())],
                ..BoxConfig::default()
            }, EventEmitter::new(16));
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let mut rescue = None;
            let stops = Arc::new(AtomicUsize::new(0));
            let outcome = std::panic::AssertUnwindSafe(async {
                vm.boot().await.unwrap();
                vm.wait_for_exec_available(Duration::from_secs(10)).await.unwrap();
                let id = vm.box_id().to_owned();
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                assert_eq!(vm.created_anonymous_volumes.len(), 1);
                let name = vm.created_anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, &id).unwrap();
                let saved = serde_json::to_value(&allocation).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino());
                let box_dir = home.join("boxes").join(&id);
                let before = serde_json::json!({"execution":id,"pid":pid,"allocation":saved,"volumeIdentity":identity});
                std::fs::write(root.join("before-cancel.json"), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !volume.join("writes").is_file() {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                // The fixture keeps rescue access to this exact handler; it is never an unrelated PID lookup.
                let actual = Arc::new(parking_lot::Mutex::new(vm.handler.write().await.take().unwrap()));
                rescue = Some(actual.clone());
                let calls = Arc::new(AtomicUsize::new(0));
                vm.provider = Some(Box::new(ReturnProvider { handler: StdMutex::new(Some(Box::new(EscrowHandler { actual, stops: stops.clone() }))), calls: calls.clone() }));
                let slot = vm.handler.clone();
                let locked = slot.write().await;
                {
                    let spec = InstanceSpec::default();
                    let installing = vm.start_legacy_handler(&spec);
                    tokio::pin!(installing);
                    poll_pending(installing.as_mut()).await;
                }
                assert_eq!(vm.pending_handler.as_ref().unwrap().pid(), pid);
                assert!(vm.pending_handler.as_ref().unwrap().is_running());
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                assert_eq!(stops.load(Ordering::SeqCst), 0);
                assert!(matches!(vm.start_legacy_handler(&InstanceSpec::default()).await, Err(BoxError::StateError(_))));
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                assert_eq!(serde_json::to_value(store.require_anonymous(&name, &id).unwrap()).unwrap(), saved);
                assert_eq!((volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino()), identity);
                assert!(box_dir.is_dir());
                let bytes = volume.join("writes").metadata().unwrap().len();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while volume.join("writes").metadata().unwrap().len() <= bytes {
                    assert!(tokio::time::Instant::now() < deadline, "same actual Guest must continue writing after installation cancellation");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                drop(locked);
                vm.destroy_with_timeout(2000).await.unwrap();
                assert!(stops.load(Ordering::SeqCst) > 0, "product finalizer must stop the actual handler before fixture rescue");
                assert!(!rescue.as_ref().unwrap().lock().is_running());
                assert!(vm.pending_handler.is_none() && vm.handler.read().await.is_none());
                assert!(!box_dir.exists() && store.get(&name).unwrap().is_none());
                serde_json::json!({"status":"PASS","before":before,"sameActualPidAndAllocationPreserved":true,"guestContinuedWriting":true,"pendingOwnerSurvivedCancellation":true,"repeatAcquisitionRefused":true,"productPhysicallyFinalized":true,"actualStopCalls":stops.load(Ordering::SeqCst),"runtimeRssMeasured":false})
            }).catch_unwind().await;
            let rescued = outcome.is_err() && rescue.is_some();
            if outcome.is_err() {
                if let Some(actual) = &rescue { actual.lock().stop(9, 0).unwrap(); }
            }
            let cleanup = vm.destroy_with_timeout(0).await;
            let remaining = store.list().unwrap();
            let dirs = std::fs::read_dir(home.join("boxes")).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).map(|entry| entry.path()).collect::<Vec<_>>()).unwrap_or_default();
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"fixtureRescueUsed":rescued,"remainingVolumes":remaining,"remainingBoxDirectories":dirs,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-handler-publication"})).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    cleanup.unwrap();
                    assert!(!rescued && remaining.is_empty() && dirs.is_empty());
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
