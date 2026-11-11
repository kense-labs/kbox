mod support;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kbox_contract::contract::RuntimeApplyRequest;
use kbox_contract::{
    FileRuntimeStateStore, ManagedRuntimeClient, RuntimeAdmissionGuard, RuntimeClient,
    RuntimeError, RuntimeResult, RuntimeStateStore,
};
use support::fault_driver::{DeterministicFaultDriver, FaultBoundary, FaultMode, FaultOperation};
use support::fixtures::apply_request;

struct Guard {
    allowed: AtomicBool,
    calls: AtomicUsize,
    reject_after: usize,
}

impl Guard {
    fn allowing() -> Self {
        Self {
            allowed: AtomicBool::new(true),
            calls: AtomicUsize::new(0),
            reject_after: usize::MAX,
        }
    }
}

#[async_trait]
impl RuntimeAdmissionGuard for Guard {
    async fn check_apply(&self, request: &RuntimeApplyRequest) -> RuntimeResult<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.allowed.load(Ordering::SeqCst) || call >= self.reject_after {
            return Err(RuntimeError::RequestConflict {
                request_id: request.request_id.clone(),
            });
        }
        Ok(())
    }
}

#[tokio::test]
async fn superseded_while_waiting_for_unit_lease_never_dispatches() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(FileRuntimeStateStore::new(root.path()));
    let driver = Arc::new(DeterministicFaultDriver::new());
    let guard = Arc::new(Guard::allowing());
    let client = Arc::new(
        ManagedRuntimeClient::new(store.clone(), driver.clone())
            .with_admission_guard(guard.clone()),
    );
    let request = apply_request("old-up", "guard-wait");
    let held = store
        .acquire_operation_lease(&request.spec.unit_id)
        .await
        .unwrap();
    let task = tokio::spawn({
        let client = client.clone();
        let request = request.clone();
        async move { client.apply(&request).await }
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !driver
            .trace()
            .iter()
            .any(|event| event.operation == FaultOperation::Capabilities)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(guard.calls.load(Ordering::SeqCst), 0);
    guard.allowed.store(false, Ordering::SeqCst);
    drop(held);
    assert!(matches!(
        task.await.unwrap(),
        Err(RuntimeError::RequestConflict { .. })
    ));
    assert_eq!(driver.effect_count(FaultOperation::Apply), 0);
    assert_eq!(driver.resource_count(&request.spec.unit_id), 0);
    assert!(matches!(
        store.load(&request.spec.unit_id).await,
        Err(RuntimeError::NotFound { .. })
    ));
}

#[tokio::test]
async fn rechecks_fence_after_durable_reservation_and_before_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(FileRuntimeStateStore::new(root.path()));
    let driver = Arc::new(DeterministicFaultDriver::new());
    let guard = Arc::new(Guard {
        reject_after: 1,
        ..Guard::allowing()
    });
    let client =
        ManagedRuntimeClient::new(store.clone(), driver.clone()).with_admission_guard(guard);
    let request = apply_request("old-up", "guard-reserved");
    assert!(matches!(
        client.apply(&request).await,
        Err(RuntimeError::RequestConflict { .. })
    ));
    assert!(store
        .load_request(&request.spec.unit_id, &request.request_id)
        .await
        .is_ok());
    assert_eq!(driver.effect_count(FaultOperation::Apply), 0);
    assert_eq!(driver.resource_count(&request.spec.unit_id), 0);
}

#[tokio::test]
async fn completed_receipt_replays_after_fence_changes_without_new_effect() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(FileRuntimeStateStore::new(root.path()));
    let driver = Arc::new(DeterministicFaultDriver::new());
    let guard = Arc::new(Guard::allowing());
    let client =
        ManagedRuntimeClient::new(store, driver.clone()).with_admission_guard(guard.clone());
    let request = apply_request("original-up", "guard-replay");
    let first = client.apply(&request).await.unwrap();
    guard.allowed.store(false, Ordering::SeqCst);
    assert_eq!(client.apply(&request).await.unwrap(), first);
    assert_eq!(driver.effect_count(FaultOperation::Apply), 1);
    assert_eq!(guard.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn restarted_client_cannot_resume_a_superseded_pending_apply() {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(FileRuntimeStateStore::new(root.path()));
    let driver = Arc::new(DeterministicFaultDriver::new());
    let guard = Arc::new(Guard::allowing());
    let request = apply_request("old-up", "guard-restart");
    driver.arm(
        FaultOperation::Apply,
        FaultBoundary::BeforeEffect,
        FaultMode::Transport,
    );
    let client = ManagedRuntimeClient::new(store.clone(), driver.clone())
        .with_admission_guard(guard.clone());
    assert!(matches!(
        client.apply(&request).await,
        Err(RuntimeError::Transport(_))
    ));
    drop(client);
    guard.allowed.store(false, Ordering::SeqCst);
    let restarted = ManagedRuntimeClient::new(store, driver.clone()).with_admission_guard(guard);
    assert!(matches!(
        restarted.apply(&request).await,
        Err(RuntimeError::RequestConflict { .. })
    ));
    assert_eq!(driver.effect_count(FaultOperation::Apply), 0);
    assert_eq!(driver.resource_count(&request.spec.unit_id), 0);
}
