use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use kbox_core::{
    BoxConfig, CreateExecutionRequest, ExecutionBoundary, ExecutionGeneration, ExecutionId,
    ExecutionManager, ExecutionManagerError, ExecutionManagerResult, ExecutionState, KillOutcome,
    NetworkMode, OperationId,
};
use tokio::sync::{oneshot, Semaphore};

use super::{
    LocalExecutionBackend, LocalExecutionHandle, LocalExecutionManager, LocalExecutionObservation,
};
use crate::{ManagedExecutionState, ManagedExecutionStore};

#[derive(Clone)]
struct FakeExecution {
    state: ExecutionState,
    handle: LocalExecutionHandle,
    exit_code: Option<i32>,
}

struct InspectionControl {
    claimed: AtomicBool,
    completed: AtomicBool,
    started: Semaphore,
    release: Semaphore,
}

impl InspectionControl {
    fn new() -> Self {
        Self {
            claimed: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            started: Semaphore::new(0),
            release: Semaphore::new(0),
        }
    }

    async fn wait_until_started(&self) {
        self.started
            .acquire()
            .await
            .expect("inspection start semaphore must remain open")
            .forget();
    }

    fn release(&self) {
        self.release.add_permits(1);
    }
}

struct CancellationBackend {
    execution: Arc<Mutex<Option<FakeExecution>>>,
    inspection: Arc<InspectionControl>,
    startup: Option<Arc<InspectionControl>>,
    starts: AtomicUsize,
}

impl CancellationBackend {
    fn new() -> Self {
        Self {
            execution: Arc::new(Mutex::new(None)),
            inspection: Arc::new(InspectionControl::new()),
            startup: None,
            starts: AtomicUsize::new(0),
        }
    }

    fn with_pending_start() -> Self {
        let backend = Self::new();
        backend.inspection.claimed.store(true, Ordering::SeqCst);
        backend.inspection.completed.store(true, Ordering::SeqCst);
        Self {
            startup: Some(Arc::new(InspectionControl::new())),
            ..backend
        }
    }

    fn execution_id(record: &crate::BoxRecord) -> ExecutionId {
        ExecutionId::new(record.id.clone()).unwrap()
    }

    fn handle(record: &crate::BoxRecord) -> LocalExecutionHandle {
        LocalExecutionHandle {
            started_at: chrono::Utc::now(),
            pid: Some(std::process::id()),
            pid_start_time: crate::process::pid_start_time(std::process::id()),
            exec_socket_path: record.box_dir.join("sockets/exec.sock"),
            console_log: record.box_dir.join("logs/console.log"),
            anonymous_volumes: Vec::new(),
            oci_runtime: None,
        }
    }
}

#[async_trait]
impl LocalExecutionBackend for CancellationBackend {
    async fn start(
        &self,
        record: &crate::BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let handle = Self::handle(record);
        *self.execution.lock().unwrap() = Some(FakeExecution {
            state: ExecutionState::Running,
            handle: handle.clone(),
            exit_code: None,
        });
        if let Some(startup) = &self.startup {
            startup.started.add_permits(1);
            startup.release.acquire().await.unwrap().forget();
            startup.completed.store(true, Ordering::SeqCst);
        }
        Ok(handle)
    }

    async fn inspect(
        &self,
        record: &crate::BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionObservation> {
        if self
            .inspection
            .claimed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let execution = Arc::clone(&self.execution);
            let inspection = Arc::clone(&self.inspection);
            let (completed_tx, completed_rx) = oneshot::channel();
            tokio::spawn(async move {
                inspection.started.add_permits(1);
                inspection
                    .release
                    .acquire()
                    .await
                    .expect("inspection release semaphore must remain open")
                    .forget();
                if let Some(execution) = execution.lock().unwrap().as_mut() {
                    execution.state = ExecutionState::Stopped;
                    execution.exit_code = Some(0);
                }
                inspection.completed.store(true, Ordering::SeqCst);
                let _ = completed_tx.send(());
            });
            completed_rx.await.map_err(|error| {
                ExecutionManagerError::Internal(format!("detached inspection task failed: {error}"))
            })?;
        } else if !self.inspection.completed.load(Ordering::SeqCst) {
            return Err(ExecutionManagerError::NotFound(Self::execution_id(record)));
        }

        let execution = self.execution.lock().unwrap();
        let execution = execution
            .as_ref()
            .ok_or_else(|| ExecutionManagerError::NotFound(Self::execution_id(record)))?;
        Ok(LocalExecutionObservation {
            state: execution.state,
            handle: matches!(
                execution.state,
                ExecutionState::Running | ExecutionState::Paused
            )
            .then(|| execution.handle.clone()),
            exit_code: execution.exit_code,
        })
    }

    async fn pause(
        &self,
        record: &crate::BoxRecord,
        _keep_memory: bool,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        Err(ExecutionManagerError::Unavailable(format!(
            "fake pause is unavailable for {}",
            record.id
        )))
    }

    async fn resume(
        &self,
        record: &crate::BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        Err(ExecutionManagerError::Unavailable(format!(
            "fake resume is unavailable for {}",
            record.id
        )))
    }

    async fn kill(&self, record: &crate::BoxRecord) -> ExecutionManagerResult<KillOutcome> {
        let mut execution = self.execution.lock().unwrap();
        let execution = execution
            .as_mut()
            .ok_or_else(|| ExecutionManagerError::NotFound(Self::execution_id(record)))?;
        if execution.state == ExecutionState::Stopped {
            Ok(KillOutcome::AlreadyStopped)
        } else {
            execution.state = ExecutionState::Stopped;
            Ok(KillOutcome::Killed)
        }
    }
}

fn request() -> CreateExecutionRequest {
    CreateExecutionRequest {
        external_sandbox_id: "cancelled-inspection".to_string(),
        config: BoxConfig {
            image: "alpine:3.20".to_string(),
            network: NetworkMode::None,
            ..Default::default()
        },
        labels: BTreeMap::new(),
        policy: Default::default(),
        rootfs_snapshot_id: None,
    }
}

#[tokio::test]
async fn cancelled_inspection_keeps_its_lifecycle_lock_until_projection_finishes() {
    let directory = tempfile::tempdir().unwrap();
    let home_dir = directory.path().join("home");
    let state_path = directory.path().join("boxes.json");
    let backend = Arc::new(CancellationBackend::new());
    let manager = LocalExecutionManager::new(&state_path, &home_dir, backend.clone());
    let lease = manager
        .create_and_start(
            request(),
            &OperationId::new("cancelled-inspection-operation").unwrap(),
        )
        .await
        .unwrap();
    let execution_id = lease.execution_id;

    let first = {
        let manager = manager.clone();
        let execution_id = execution_id.clone();
        tokio::spawn(async move { manager.inspect(&execution_id).await })
    };
    backend.inspection.wait_until_started().await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());

    let restarted = LocalExecutionManager::new(&state_path, &home_dir, backend.clone());
    let mut replay = {
        let restarted = restarted.clone();
        let execution_id = execution_id.clone();
        tokio::spawn(async move { restarted.inspect(&execution_id).await })
    };
    let early = tokio::time::timeout(Duration::from_millis(50), &mut replay).await;
    backend.inspection.release();
    let (waited_for_original_inspection, status) = match early {
        Ok(result) => (false, result.unwrap().unwrap()),
        Err(_) => (
            true,
            tokio::time::timeout(Duration::from_secs(2), replay)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        ),
    };

    assert!(
        waited_for_original_inspection,
        "replay bypassed the cancelled inspection's lifecycle lock"
    );
    assert_eq!(status.state, ExecutionState::Stopped);
    let record = ManagedExecutionStore::new(state_path)
        .get(&execution_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        record.managed_state().unwrap(),
        Some(ManagedExecutionState::Stopped)
    );
    assert_eq!(record.exit_code, Some(0));
}

#[tokio::test]
async fn cancelled_start_keeps_its_original_operation_until_durable_completion() {
    let directory = tempfile::tempdir().unwrap();
    let home_dir = directory.path().join("home");
    let state_path = directory.path().join("boxes.json");
    let backend = Arc::new(CancellationBackend::with_pending_start());
    let manager = LocalExecutionManager::new(&state_path, &home_dir, backend.clone());
    let reservation = manager
        .create(request(), &OperationId::new("cancelled-start").unwrap())
        .await
        .unwrap();
    let execution_id = reservation.execution_id;
    let generation = reservation.generation;
    let first = {
        let manager = manager.clone();
        let execution_id = execution_id.clone();
        tokio::spawn(async move { manager.start(&execution_id, generation).await })
    };
    let startup = backend.startup.as_ref().unwrap();
    startup.wait_until_started().await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    drop(manager);

    let restarted = LocalExecutionManager::new(&state_path, &home_dir, backend.clone());
    {
        let waiter = restarted.start(&execution_id, generation);
        tokio::pin!(waiter);
        assert!(futures::poll!(waiter.as_mut()).is_pending());
    }
    let mut replay = {
        let restarted = restarted.clone();
        let execution_id = execution_id.clone();
        tokio::spawn(async move { restarted.start(&execution_id, generation).await })
    };
    let early = tokio::time::timeout(Duration::from_millis(50), &mut replay).await;
    startup.release();
    let waited = early.is_err();
    let lease = match early {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => tokio::time::timeout(Duration::from_secs(2), replay)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    };
    let before_stop = ManagedExecutionStore::new(&state_path)
        .get(&execution_id)
        .unwrap()
        .unwrap();
    // Finish the fixture before asserting the regression so a red run owns no work.
    restarted.kill(&execution_id, generation).await.unwrap();
    assert!(waited, "replay bypassed the original pending start");
    assert!(startup.completed.load(Ordering::SeqCst));
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    assert_eq!(lease.execution_id, execution_id);
    assert!(matches!(
        restarted
            .start(
                &execution_id,
                ExecutionGeneration::new(generation.get() + 1).unwrap()
            )
            .await,
        Err(ExecutionManagerError::Conflict { .. })
    ));
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    let record = ManagedExecutionStore::new(state_path)
        .get(&execution_id)
        .unwrap()
        .unwrap();
    assert_eq!(before_stop.pid, Some(std::process::id()));
    assert_eq!(
        before_stop.pid_start_time,
        crate::process::pid_start_time(std::process::id())
    );
    assert_eq!(
        before_stop.managed_state().unwrap(),
        Some(ManagedExecutionState::Running)
    );
    assert_eq!(
        record.managed_state().unwrap(),
        Some(ManagedExecutionState::Stopped)
    );
}

#[tokio::test]
async fn cancelled_start_cannot_publish_running_after_a_serialized_stop() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    let store = directory.path().join("boxes.json");
    let backend = Arc::new(CancellationBackend::with_pending_start());
    let manager = LocalExecutionManager::new(&store, &home, backend.clone());
    let reservation = manager
        .create(
            request(),
            &OperationId::new("cancelled-start-stop").unwrap(),
        )
        .await
        .unwrap();
    let id = reservation.execution_id;
    let generation = reservation.generation;
    let first = {
        let manager = manager.clone();
        let id = id.clone();
        tokio::spawn(async move { manager.start(&id, generation).await })
    };
    let startup = backend.startup.as_ref().unwrap();
    startup.wait_until_started().await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let mut stop = {
        let manager = manager.clone();
        let id = id.clone();
        tokio::spawn(async move { manager.kill(&id, generation).await })
    };
    let early = tokio::time::timeout(Duration::from_millis(50), &mut stop).await;
    startup.release();
    let waited = early.is_err();
    let result = match early {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => tokio::time::timeout(Duration::from_secs(2), stop)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    };
    assert!(waited, "stop bypassed the original pending start");
    assert_eq!(result, KillOutcome::Killed);
    assert!(startup.completed.load(Ordering::SeqCst));
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    let record = ManagedExecutionStore::new(store).get(&id).unwrap().unwrap();
    assert_eq!(
        record.managed_state().unwrap(),
        Some(ManagedExecutionState::Stopped)
    );
}

#[tokio::test]
async fn cancelled_start_before_admission_leaves_the_reservation_unstarted() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    let store = directory.path().join("boxes.json");
    let backend = Arc::new(CancellationBackend::new());
    let manager = LocalExecutionManager::new(&store, &home, backend.clone());
    let reservation = manager
        .create(
            request(),
            &OperationId::new("cancelled-before-start").unwrap(),
        )
        .await
        .unwrap();
    let id = reservation.execution_id;
    let lifecycle = super::lifecycle_lock::acquire(&home, id.as_str())
        .await
        .unwrap();
    {
        let start = manager.start(&id, reservation.generation);
        tokio::pin!(start);
        assert!(futures::poll!(start.as_mut()).is_pending());
    }
    drop(lifecycle);
    let status = manager.inspect(&id).await.unwrap();
    assert_eq!(status.state, ExecutionState::Created);
    assert_eq!(backend.starts.load(Ordering::SeqCst), 0);
    let record = ManagedExecutionStore::new(store).get(&id).unwrap().unwrap();
    assert!(record.pid.is_none());
    assert!(record.pid_start_time.is_none());
}
