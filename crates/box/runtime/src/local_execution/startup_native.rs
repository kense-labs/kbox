//! Real managed VM startup retained across a caller control timeout.
use super::*;
use crate::ManagedRuntimeRoute;
use async_trait::async_trait;
use futures::FutureExt;
use kbox_core::{
    BoxConfig, CreateExecutionRequest, ExecutionManager, ExecutionState, KillExecutionOptions,
    NetworkMode, OperationId,
};
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Semaphore;

struct PendingVmStart {
    actual: VmLocalExecutionBackend,
    entered: Semaphore,
    release: Semaphore,
    starts: AtomicUsize,
    returned: Mutex<Option<LocalExecutionHandle>>,
}

#[async_trait]
impl LocalExecutionBackend for PendingVmStart {
    fn route_for_create(&self, record: &BoxRecord) -> ExecutionManagerResult<ManagedRuntimeRoute> {
        self.actual.route_for_create(record)
    }
    async fn preflight_launch(&self) -> ExecutionManagerResult<()> {
        self.actual.preflight_launch().await
    }
    async fn preflight(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.actual.preflight(record).await
    }
    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionResourcePlan> {
        self.actual.plan_create_resources(record).await
    }
    async fn start(&self, record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let handle = self.actual.start(record).await?;
        *self.returned.lock().unwrap() = Some(handle.clone());
        self.entered.add_permits(1);
        self.release.acquire().await.unwrap().forget();
        Ok(handle)
    }
    async fn inspect(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionObservation> {
        self.actual.inspect(record).await
    }
    async fn pause(
        &self,
        record: &BoxRecord,
        memory: bool,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        self.actual.pause(record, memory).await
    }
    async fn resume(&self, record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        self.actual.resume(record).await
    }
    async fn kill(&self, record: &BoxRecord) -> ExecutionManagerResult<kbox_core::KillOutcome> {
        self.actual.kill(record).await
    }
    async fn kill_with_status(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionTermination> {
        self.actual.kill_with_status(record).await
    }
}

#[test]
#[ignore = "requires owned sentinel home, VOLUME image and installed companions"]
fn native_timed_out_start_retains_original_vm_and_durable_completion() {
    let _subscriber = tracing_subscriber::fmt()
        .with_env_filter("kbox_runtime=info")
        .with_writer(std::io::stderr)
        .try_init();
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-execution-start-native")).unwrap(), "owned-execution-start-native-test\n");
        let root = home.parent().unwrap();
        let caller = root.join("caller");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned-execution-start").unwrap();
        let caller_identity = (caller.metadata().unwrap().dev(), caller.metadata().unwrap().ino());
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let backend = Arc::new(PendingVmStart { actual: VmLocalExecutionBackend::new(&home), entered: Semaphore::new(0), release: Semaphore::new(0), starts: AtomicUsize::new(0), returned: Mutex::new(None) });
            let manager = LocalExecutionManager::new(home.join("boxes.json"), &home, backend.clone());
            let reservation = manager.create(CreateExecutionRequest {
                external_sandbox_id: "native-start-timeout".into(),
                config: BoxConfig { image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(), network: NetworkMode::None, persistent: true, cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do printf x >> /owned/writes; printf y >> /scratch/writes; /bin/sleep 0.1; done".into()], volumes: vec![format!("{}:/owned:rw", caller.display())], ..Default::default() },
                labels: BTreeMap::new(), policy: Default::default(), rootfs_snapshot_id: None,
            }, &OperationId::new("owned-native-start-timeout").unwrap()).await.unwrap();
            let id = reservation.execution_id;
            let generation = reservation.generation;
            let mut first = Some({ let owner = manager.clone(); let id = id.clone(); tokio::spawn(async move { tokio::time::timeout(Duration::from_secs(1), owner.start(&id, generation)).await }) });
            let store = crate::volume::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
            let outcome = std::panic::AssertUnwindSafe(async {
                tokio::time::timeout(Duration::from_secs(180), backend.entered.acquire()).await.unwrap().unwrap().forget();
                assert!(first.take().unwrap().await.unwrap().is_err(), "caller control timeout must actually expire");
                let before = manager.managed_record(&id).await.unwrap().unwrap();
                assert_eq!(before.managed_state().unwrap(), Some(ManagedExecutionState::Starting));
                assert!(before.pid.is_none() && before.pid_start_time.is_none());
                let handle = backend.returned.lock().unwrap().clone().unwrap();
                let pid = handle.pid.unwrap();
                let token = handle.pid_start_time.unwrap();
                assert!(crate::process::is_process_alive_with_identity(pid, Some(token)));
                std::fs::write(root.join("starting-record.json"), serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                assert!(before.anonymous_volumes.is_empty());
                assert_eq!(handle.anonymous_volumes.len(), 1);
                let name = handle.anonymous_volumes[0].clone();
                let allocation = store.require_anonymous(&name, id.as_str()).unwrap();
                let saved = serde_json::to_value(&allocation).unwrap();
                let volume = PathBuf::from(&allocation.mount_point);
                let identity = (volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino());
                let proof = serde_json::json!({"execution":id,"generation":generation.get(),"pid":pid,"pidStartTime":token,"allocation":saved,"volumeIdentity":identity,"callerIdentity":caller_identity,"durableState":"starting","durablePidAbsentBeforeCompletion":true});
                std::fs::write(root.join("before-completion.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while !volume.join("writes").is_file() || !caller.join("writes").is_file() { assert!(tokio::time::Instant::now() < deadline); tokio::time::sleep(Duration::from_millis(50)).await; }
                let bytes = volume.join("writes").metadata().unwrap().len();
                while volume.join("writes").metadata().unwrap().len() <= bytes { assert!(tokio::time::Instant::now() < deadline); tokio::time::sleep(Duration::from_millis(50)).await; }
                let rebuilt = LocalExecutionManager::new(home.join("boxes.json"), &home, backend.clone());
                let mut replay = { let id = id.clone(); tokio::spawn(async move { rebuilt.start(&id, generation).await }) };
                let early = tokio::time::timeout(Duration::from_millis(100), &mut replay).await;
                backend.release.add_permits(1);
                let waited = early.is_err();
                let lease = match early { Ok(r) => r.unwrap().unwrap(), Err(_) => tokio::time::timeout(Duration::from_secs(30), replay).await.unwrap().unwrap().unwrap() };
                assert!(waited, "rebuilt caller bypassed original pending startup");
                assert_eq!(lease.execution_id, id);
                assert_eq!(lease.generation, generation);
                let recorded = manager.managed_record(&id).await.unwrap().unwrap();
                std::fs::write(root.join("durable-running-record.json"), serde_json::to_vec_pretty(&recorded).unwrap()).unwrap();
                assert_eq!(recorded.anonymous_volumes, handle.anonymous_volumes);
                assert_eq!(recorded.managed_execution.as_ref().unwrap().generation, generation);
                assert_eq!(recorded.managed_state().unwrap(), Some(ManagedExecutionState::Running));
                assert_eq!(recorded.pid, Some(pid)); assert_eq!(recorded.pid_start_time, Some(token));
                assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
                assert_eq!(serde_json::to_value(store.require_anonymous(&name, id.as_str()).unwrap()).unwrap(), saved);
                assert_eq!((volume.metadata().unwrap().dev(), volume.metadata().unwrap().ino()), identity);
                assert_eq!(manager.inspect(&id).await.unwrap().state, ExecutionState::Running);
                serde_json::json!({"status":"PASS","before":proof,"callerTimeoutActuallyExpired":true,"replayWaitedForOriginal":true,"sameActualPidAndAllocationPreserved":true,"guestContinuedWriting":true,"durableOriginalIdentityRecorded":true,"actualBackendStarts":1,"runtimeRssMeasured":false})
            }).catch_unwind().await;
            backend.release.add_permits(1);
            if let Some(first) = first.take() { let _ = first.await; }
            let killed = manager.kill_with_options(&id, generation, KillExecutionOptions { signal: Some(15), timeout_secs: Some(2) }).await;
            let removed = if killed.is_ok() { manager.remove_execution(&id, generation).await } else { Ok(false) };
            let remaining = manager.managed_records().await.unwrap();
            let volumes = store.list().unwrap();
            let handle = backend.returned.lock().unwrap().clone();
            let alive = handle.as_ref().is_some_and(|h| h.pid.is_some_and(|pid| crate::process::is_process_alive_with_identity(pid, h.pid_start_time)));
            let cleanup = serde_json::json!({"killOk":killed.is_ok(),"removeOk":removed.as_ref().is_ok_and(|v|*v),"remainingExecutions":remaining,"remainingVolumes":volumes,"samePidAlive":alive,"callerCanaryRetained":std::fs::read(caller.join("canary")).unwrap()==b"caller-owned-execution-start"});
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&cleanup).unwrap()).unwrap();
            match outcome {
                Ok(mut proof) => {
                    killed.unwrap(); assert!(removed.unwrap()); assert!(remaining.is_empty() && volumes.is_empty() && !alive);
                    assert!(!home.join("boxes").join(id.as_str()).exists());
                    assert_eq!((caller.metadata().unwrap().dev(), caller.metadata().unwrap().ino()), caller_identity);
                    let writes = std::fs::read(caller.join("writes")).unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    assert_eq!(std::fs::read(caller.join("writes")).unwrap(), writes);
                    proof["productPhysicallyFinalized"] = true.into(); proof["noLateWritesAfterOwnedCleanup"] = true.into(); proof["callerBytesRetained"] = writes.len().into();
                    std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                }
                Err(error) => std::panic::resume_unwind(error),
            }
        });
    }).unwrap().join().unwrap();
}
