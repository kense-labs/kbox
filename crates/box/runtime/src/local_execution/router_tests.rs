use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kbox_core::{
    BoxConfig, CreateExecutionRequest, ExecutionBoundary, ExecutionId, ExecutionManager,
    ExecutionManagerError, ExecutionManagerResult, ExecutionState, KillOutcome, OperationId,
};

use super::record::build_managed_record;
use super::{
    LocalExecutionBackend, LocalExecutionBackendRouter, LocalExecutionHandle,
    LocalExecutionManager, LocalExecutionObservation, LocalExecutionResourcePlan,
    OciMigrationPolicy,
};
use crate::{BoxRecord, ManagedRuntimeRoute};

#[derive(Default)]
struct RoutedProbeBackend {
    fail_isolation_preflight: AtomicBool,
    fail_preflight: AtomicBool,
    isolation_preflights: AtomicUsize,
    preflights: AtomicUsize,
    kills: AtomicUsize,
    observed_isolations: Mutex<Vec<ExecutionBoundary>>,
    observed_routes: Mutex<Vec<ManagedRuntimeRoute>>,
    planned_anonymous_volumes: Mutex<Vec<String>>,
}

impl RoutedProbeBackend {
    fn reject_isolation_preflight(&self) {
        self.fail_isolation_preflight.store(true, Ordering::Relaxed);
    }

    fn reject_preflight(&self) {
        self.fail_preflight.store(true, Ordering::Relaxed);
    }

    fn preflights(&self) -> usize {
        self.preflights.load(Ordering::Relaxed)
    }

    fn isolation_preflights(&self) -> usize {
        self.isolation_preflights.load(Ordering::Relaxed)
    }

    fn kills(&self) -> usize {
        self.kills.load(Ordering::Relaxed)
    }

    fn observed_routes(&self) -> Vec<ManagedRuntimeRoute> {
        self.observed_routes.lock().unwrap().clone()
    }

    fn observed_isolations(&self) -> Vec<ExecutionBoundary> {
        self.observed_isolations.lock().unwrap().clone()
    }

    fn plan_anonymous_volumes(&self, names: &[&str]) {
        *self.planned_anonymous_volumes.lock().unwrap() =
            names.iter().map(|name| (*name).to_string()).collect();
    }
}

#[async_trait]
impl LocalExecutionBackend for RoutedProbeBackend {
    async fn preflight_launch(&self) -> ExecutionManagerResult<()> {
        self.isolation_preflights.fetch_add(1, Ordering::Relaxed);
        self.observed_isolations
            .lock()
            .unwrap()
            .push(ExecutionBoundary::Microvm);
        if self.fail_isolation_preflight.load(Ordering::Relaxed) {
            Err(ExecutionManagerError::Unavailable(
                "selected isolation is unavailable".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn preflight(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.preflights.fetch_add(1, Ordering::Relaxed);
        self.observed_routes.lock().unwrap().push(
            record
                .managed_execution
                .as_ref()
                .expect("managed metadata")
                .runtime_route,
        );
        if self.fail_preflight.load(Ordering::Relaxed) {
            Err(ExecutionManagerError::Unavailable(
                "selected backend is unavailable".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn plan_create_resources(
        &self,
        _record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionResourcePlan> {
        Ok(LocalExecutionResourcePlan {
            anonymous_volumes: self.planned_anonymous_volumes.lock().unwrap().clone(),
            ..Default::default()
        })
    }

    async fn start(&self, _record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        Err(ExecutionManagerError::Unavailable(
            "probe backend does not launch".to_string(),
        ))
    }

    async fn inspect(
        &self,
        _record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionObservation> {
        Ok(LocalExecutionObservation {
            state: ExecutionState::Created,
            handle: None,
            exit_code: None,
        })
    }

    async fn pause(
        &self,
        _record: &BoxRecord,
        _keep_memory: bool,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        Err(ExecutionManagerError::Unavailable(
            "probe backend does not pause".to_string(),
        ))
    }

    async fn resume(&self, _record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        Err(ExecutionManagerError::Unavailable(
            "probe backend does not resume".to_string(),
        ))
    }

    async fn kill(&self, _record: &BoxRecord) -> ExecutionManagerResult<KillOutcome> {
        self.kills.fetch_add(1, Ordering::Relaxed);
        Ok(KillOutcome::Killed)
    }
}

#[test]
fn migration_policies_stay_the_existing_set() {
    let policies = [
        OciMigrationPolicy::LegacyOnly,
        OciMigrationPolicy::MicrovmViaOci,
        OciMigrationPolicy::AllViaOci,
    ];
    let names: Vec<_> = policies.into_iter().map(policy_name).collect();
    assert_eq!(names, ["legacy-only", "microvm-via-oci", "all-via-oci",]);
}

fn policy_name(policy: OciMigrationPolicy) -> &'static str {
    match policy {
        OciMigrationPolicy::LegacyOnly => "legacy-only",
        OciMigrationPolicy::MicrovmViaOci => "microvm-via-oci",
        OciMigrationPolicy::AllViaOci => "all-via-oci",
    }
}

#[tokio::test]
async fn oci_preflight_retry_never_retargets_to_legacy() {
    let temporary = tempfile::tempdir().unwrap();
    let state_path = temporary.path().join("boxes.json");
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    oci.reject_isolation_preflight();
    let manager = LocalExecutionManager::new(
        &state_path,
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::MicrovmViaOci,
        )),
    );

    let error = manager
        .preflight_launch()
        .await
        .expect_err("sandbox preflight must fail on the selected backend");
    assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
    assert_eq!(legacy.isolation_preflights(), 0);
    assert_eq!(oci.observed_isolations(), vec![ExecutionBoundary::Microvm]);

    assert!(matches!(
        manager.preflight_launch().await,
        Err(ExecutionManagerError::Unavailable(_))
    ));
    assert_eq!(legacy.isolation_preflights(), 0);
    assert_eq!(
        oci.observed_isolations(),
        vec![ExecutionBoundary::Microvm; 2]
    );
    assert!(!state_path.exists());
    assert!(!temporary.path().join("boxes").exists());
}

#[tokio::test]
async fn microvm_preflight_error_does_not_retarget_to_sandbox() {
    let temporary = tempfile::tempdir().unwrap();
    let state_path = temporary.path().join("boxes.json");
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    legacy.reject_isolation_preflight();
    let manager = LocalExecutionManager::new(
        &state_path,
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::LegacyOnly,
        )),
    );

    let error = manager
        .preflight_launch()
        .await
        .expect_err("microvm preflight must fail on the selected backend");
    assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
    assert_eq!(oci.isolation_preflights(), 0);
    assert_eq!(
        legacy.observed_isolations(),
        vec![ExecutionBoundary::Microvm]
    );

    let sandbox_error = manager
        .preflight_launch()
        .await
        .expect_err("sandbox stays on the legacy backend after the microvm error");
    assert!(matches!(
        sandbox_error,
        ExecutionManagerError::Unavailable(_)
    ));
    assert_eq!(oci.isolation_preflights(), 0);
    assert_eq!(
        legacy.observed_isolations(),
        vec![ExecutionBoundary::Microvm, ExecutionBoundary::Microvm]
    );
    assert!(!state_path.exists());
    assert!(!temporary.path().join("boxes").exists());
}

#[tokio::test]
async fn selected_oci_isolation_preflight_fails_closed_without_fallback_or_state() {
    let temporary = tempfile::tempdir().unwrap();
    let state_path = temporary.path().join("boxes.json");
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    oci.reject_isolation_preflight();
    let manager = LocalExecutionManager::new(
        &state_path,
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::AllViaOci,
        )),
    );

    let error = manager
        .preflight_launch()
        .await
        .expect_err("selected OCI isolation must fail closed");

    assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
    assert_eq!(legacy.isolation_preflights(), 0);
    assert_eq!(oci.isolation_preflights(), 1);
    assert_eq!(oci.observed_isolations(), vec![ExecutionBoundary::Microvm]);
    assert!(!state_path.exists());
}

#[tokio::test]
async fn sandbox_policy_stamps_oci_route_before_preflight_and_reservation() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    oci.plan_anonymous_volumes(&["anon_planned_one", "anon_planned_two"]);
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::MicrovmViaOci,
    );
    let manager = LocalExecutionManager::new(
        temporary.path().join("boxes.json"),
        temporary.path(),
        Arc::new(router),
    );

    let reservation = manager
        .create(request(), &OperationId::new("route-sandbox").unwrap())
        .await
        .unwrap();
    let record = manager
        .get(&reservation.execution_id)
        .await
        .unwrap()
        .expect("reserved record");

    assert_eq!(
        record.managed_execution.as_ref().unwrap().runtime_route,
        ManagedRuntimeRoute::OciSdk
    );
    assert_eq!(legacy.preflights(), 0);
    assert_eq!(oci.preflights(), 1);
    assert_eq!(oci.observed_routes(), vec![ManagedRuntimeRoute::OciSdk]);
    assert_eq!(
        record.anonymous_volumes,
        vec!["anon_planned_one", "anon_planned_two"]
    );
}

#[tokio::test]
async fn new_policy_preserves_old_legacy_records_and_routes_new_microvms_to_oci() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let manager = LocalExecutionManager::new(
        temporary.path().join("boxes.json"),
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::LegacyOnly,
        )),
    );

    let existing = manager
        .create(request(), &OperationId::new("route-legacy-kept").unwrap())
        .await
        .unwrap();
    let manager = LocalExecutionManager::new(
        temporary.path().join("boxes.json"),
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::MicrovmViaOci,
        )),
    );
    let microvm = manager
        .create(request(), &OperationId::new("route-microvm-oci").unwrap())
        .await
        .unwrap();

    let existing_record = manager
        .get(&existing.execution_id)
        .await
        .unwrap()
        .expect("existing record");
    let microvm_record = manager
        .get(&microvm.execution_id)
        .await
        .unwrap()
        .expect("microvm record");

    assert_eq!(
        existing_record
            .managed_execution
            .as_ref()
            .unwrap()
            .runtime_route,
        ManagedRuntimeRoute::BoxVm
    );
    assert_eq!(
        microvm_record
            .managed_execution
            .as_ref()
            .unwrap()
            .runtime_route,
        ManagedRuntimeRoute::OciSdk
    );
    assert_eq!(legacy.preflights(), 1);
    assert_eq!(oci.preflights(), 1);
}

#[tokio::test]
async fn invalid_backend_resource_plan_is_never_reserved() {
    let temporary = tempfile::tempdir().unwrap();
    let state_path = temporary.path().join("boxes.json");
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    oci.plan_anonymous_volumes(&["../escaped"]);
    let manager = LocalExecutionManager::new(
        &state_path,
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy,
            oci,
            OciMigrationPolicy::AllViaOci,
        )),
    );

    let error = manager
        .create(
            request(),
            &OperationId::new("invalid-resource-plan").unwrap(),
        )
        .await
        .expect_err("unsafe planned identities must fail before reservation");

    assert!(matches!(error, ExecutionManagerError::Internal(_)));
    assert!(!state_path.exists());
}

#[tokio::test]
async fn explicit_legacy_policy_keeps_microvm_on_the_persisted_legacy_route() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let manager = LocalExecutionManager::new(
        temporary.path().join("boxes.json"),
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::LegacyOnly,
        )),
    );

    let reservation = manager
        .create(request(), &OperationId::new("route-microvm").unwrap())
        .await
        .unwrap();
    let record = manager
        .get(&reservation.execution_id)
        .await
        .unwrap()
        .expect("reserved record");

    assert_eq!(
        record.managed_execution.unwrap().runtime_route,
        ManagedRuntimeRoute::BoxVm
    );
    assert_eq!(legacy.preflights(), 1);
    assert_eq!(oci.preflights(), 0);
}

#[tokio::test]
async fn selected_oci_preflight_failure_never_falls_back_or_reserves() {
    let temporary = tempfile::tempdir().unwrap();
    let state_path = temporary.path().join("boxes.json");
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    oci.reject_preflight();
    let manager = LocalExecutionManager::new(
        &state_path,
        temporary.path(),
        Arc::new(LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::AllViaOci,
        )),
    );

    let error = manager
        .create(request(), &OperationId::new("route-no-fallback").unwrap())
        .await
        .unwrap_err();

    assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
    assert_eq!(legacy.preflights(), 0);
    assert_eq!(oci.preflights(), 1);
    assert!(!state_path.exists());
}

#[tokio::test]
async fn persisted_routes_ignore_a_later_policy_change() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let all_oci = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::AllViaOci,
    );
    let legacy_only = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::LegacyOnly,
    );
    let mut legacy_record = record(temporary.path(), "legacy");
    legacy_record
        .managed_execution
        .as_mut()
        .unwrap()
        .runtime_route = ManagedRuntimeRoute::BoxVm;
    let mut oci_record = record(temporary.path(), "oci");
    oci_record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::OciSdk;

    all_oci.kill(&legacy_record).await.unwrap();
    legacy_only.kill(&oci_record).await.unwrap();

    assert_eq!(legacy.kills(), 1);
    assert_eq!(oci.kills(), 1);
}

#[tokio::test]
async fn pre_routing_records_use_durable_runtime_evidence_not_current_policy() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::AllViaOci,
    );
    let legacy_record = record(temporary.path(), "old-legacy");
    let mut stopped_oci_record = record(temporary.path(), "old-oci");
    stopped_oci_record.exec_socket_path.clear();

    router.kill(&legacy_record).await.unwrap();
    router.kill(&stopped_oci_record).await.unwrap();

    assert_eq!(legacy.kills(), 1);
    assert_eq!(oci.kills(), 1);
}

#[tokio::test]
async fn sandbox_route_refuses_until_backend_is_wired() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::LegacyOnly,
    );
    let mut sandbox_record = record(temporary.path(), "sandbox");
    sandbox_record
        .managed_execution
        .as_mut()
        .unwrap()
        .runtime_route = ManagedRuntimeRoute::Sandbox;

    let error = router.kill(&sandbox_record).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("sandbox runtime route is not wired"),
        "unexpected error: {error}"
    );
    assert_eq!(legacy.kills(), 0);
    assert_eq!(oci.kills(), 0);
}

#[tokio::test]
async fn sandbox_execution_boundary_stamps_sandbox_route_on_create() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let sandbox = Arc::new(RoutedProbeBackend::default());
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::LegacyOnly,
    )
    .with_sandbox(sandbox.clone());
    let mut record = record(temporary.path(), "bound");
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .execution_boundary = kbox_core::ExecutionBoundary::Sandbox;

    assert_eq!(
        router.route_for_create(&record).unwrap(),
        ManagedRuntimeRoute::Sandbox
    );
}

#[tokio::test]
async fn sandbox_route_dispatches_only_to_attached_sandbox_backend() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let sandbox = Arc::new(RoutedProbeBackend::default());
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::LegacyOnly,
    )
    .with_sandbox(sandbox.clone());
    let mut sandbox_record = record(temporary.path(), "sandbox2");
    sandbox_record
        .managed_execution
        .as_mut()
        .unwrap()
        .runtime_route = ManagedRuntimeRoute::Sandbox;

    router.kill(&sandbox_record).await.unwrap();
    assert_eq!(sandbox.kills(), 1);
    assert_eq!(legacy.kills(), 0);
    assert_eq!(oci.kills(), 0);
}

#[tokio::test]
async fn sandbox_execution_boundary_pins_sandbox_route_without_microvm_downgrade() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let sandbox = Arc::new(RoutedProbeBackend::default());
    // Migration policy would otherwise select OCI; Sandbox boundary must win.
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::AllViaOci,
    )
    .with_sandbox(sandbox.clone());
    let mut sandbox_record = record(temporary.path(), "boundary-sandbox");
    sandbox_record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .execution_boundary = ExecutionBoundary::Sandbox;

    let route = router.route_for_create(&sandbox_record).unwrap();
    assert_eq!(route, ManagedRuntimeRoute::Sandbox);
    // Mirror prepare_creation_record: pin the route before record preflight.
    sandbox_record
        .managed_execution
        .as_mut()
        .unwrap()
        .runtime_route = route;
    router.preflight(&sandbox_record).await.unwrap();
    assert_eq!(sandbox.preflights(), 1);
    assert_eq!(legacy.preflights(), 0);
    assert_eq!(oci.preflights(), 0);
    assert_eq!(
        sandbox.observed_routes(),
        vec![ManagedRuntimeRoute::Sandbox]
    );
}

#[tokio::test]
async fn microvm_execution_boundary_still_follows_migration_policy() {
    let temporary = tempfile::tempdir().unwrap();
    let legacy = Arc::new(RoutedProbeBackend::default());
    let oci = Arc::new(RoutedProbeBackend::default());
    let sandbox = Arc::new(RoutedProbeBackend::default());
    let router = LocalExecutionBackendRouter::new(
        legacy.clone(),
        oci.clone(),
        OciMigrationPolicy::AllViaOci,
    )
    .with_sandbox(sandbox.clone());
    let microvm_record = record(temporary.path(), "boundary-microvm");
    assert_eq!(
        microvm_record
            .managed_execution
            .as_ref()
            .unwrap()
            .request
            .config
            .execution_boundary,
        ExecutionBoundary::Microvm
    );
    assert_eq!(
        router.route_for_create(&microvm_record).unwrap(),
        ManagedRuntimeRoute::OciSdk
    );
}

fn request() -> CreateExecutionRequest {
    CreateExecutionRequest {
        external_sandbox_id: "router".to_string(),
        config: BoxConfig {
            image: "alpine:latest".to_string(),
            ..Default::default()
        },
        labels: Default::default(),
        policy: Default::default(),
        rootfs_snapshot_id: None,
    }
}

fn record(home_dir: &Path, operation_suffix: &str) -> BoxRecord {
    build_managed_record(
        home_dir,
        &ExecutionId::new(format!("00000000-0000-4000-8000-{operation_suffix:0>12}")).unwrap(),
        OperationId::new(format!("route-{operation_suffix}")).unwrap(),
        request(),
        chrono::Utc::now(),
    )
    .unwrap()
}
