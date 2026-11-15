use std::sync::Arc;

use kbox_contract::contract::{RuntimeApplyRequest, RuntimeUnitSpec};
use kbox_contract::{RuntimeAdmissionGuard, RuntimeError};
use tokio::sync::Mutex;

use super::*;

#[path = "process_tests.rs"]
mod process_tests;

#[test]
fn engine_endpoint_is_stable_short_and_scoped_to_its_home() {
    use std::os::unix::ffi::OsStrExt;
    let home = std::path::PathBuf::from("/tmp").join("很长的 private home ".repeat(30));
    let endpoint = default_engine_endpoint(&home).unwrap();
    assert_eq!(default_engine_endpoint(&home).unwrap(), endpoint);
    assert!(endpoint.as_os_str().as_bytes().len() < 104);
    assert_ne!(
        default_engine_endpoint(&home.join("other")).unwrap(),
        endpoint
    );
    assert_eq!(
        default_engine_endpoint(std::path::Path::new("relative-home")).unwrap(),
        default_engine_endpoint(&std::env::current_dir().unwrap().join("relative-home")).unwrap()
    );
}

#[test]
fn engine_errors_retain_sources_without_exposing_sensitive_input() {
    let secret = "synthetic-secret-must-stay-private";
    let error = EngineError::from(std::io::Error::other(secret));
    assert!(std::error::Error::source(&error).is_some());
    for rendered in [
        format!("{error}"),
        format!("{error:?}"),
        serde_json::to_string(&error).unwrap(),
    ] {
        assert!(!rendered.contains(secret));
    }
}

fn template() -> EngineServiceTemplate {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../runtime/tests/golden/unit-spec-v4.json"
    )))
    .unwrap();
    let object = value.as_object_mut().unwrap();
    for name in [
        "schema",
        "unit_id",
        "generation",
        "class",
        "outputs",
        "semantics_profile_digest",
        "identity_attachment_digest",
    ] {
        object.remove(name);
    }
    value["resources"]["execution_timeout_ms"] = serde_json::Value::Null;
    value["isolation"] = "sandbox".into();
    value["secrets"] = serde_json::json!([]);
    serde_json::from_value(value).unwrap()
}

pub(super) fn intent(engine: &EngineKey, revision: u64, state: DesiredState) -> EngineIntent {
    EngineIntent {
        schema: "kbox.engine.intent.v2".into(),
        engine_id: engine.clone(),
        instance_id: EngineKey::parse("instance").unwrap(),
        request_id: EngineKey::parse(format!("request-{revision}")).unwrap(),
        revision,
        desired_state: state,
        plan_digest: format!("sha256:{}", "1".repeat(64)),
        service: (state == DesiredState::Up).then(template),
        directories: Vec::new(),
        prewarm_bindings_digest: None,
    }
}

#[test]
fn rejects_unsupported_schema_and_unsafe_identifiers() {
    let key = EngineKey::parse("engine").unwrap();
    for value in ["", "../escape", "/absolute", "with space"] {
        assert!(EngineKey::parse(value).is_err());
    }
    let mut request = intent(&key, 1, DesiredState::Up);
    request.schema = "kbox.engine.intent.v99".into();
    assert_eq!(
        request.validate().unwrap_err().code,
        EngineErrorCode::UnsupportedVersion
    );
    request.schema = "kbox.engine.intent.v2".into();
    request.revision = 0;
    assert_eq!(
        request.validate().unwrap_err().code,
        EngineErrorCode::InvalidRequest
    );
}

#[test]
fn engine_preserves_explicit_service_policy_without_workload_classification() {
    let mut request = intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    let ordinary = request.bind_service("owned-unit".into(), 7).unwrap();
    assert_eq!(ordinary.unit_id, "owned-unit");
    assert_eq!(ordinary.generation, 7);
    assert!(ordinary.semantics_profile_digest.is_none());
    assert_eq!(
        ordinary.class,
        kbox_contract::contract::RuntimeUnitClass::Service
    );
    request.service.as_mut().unwrap().isolation = kbox_contract::contract::IsolationLevel::Process;
    request.validate().unwrap();
    assert_eq!(
        request
            .bind_service("owned-unit".into(), 7)
            .unwrap()
            .isolation,
        kbox_contract::contract::IsolationLevel::Process
    );
    let service = request.service.as_mut().unwrap();
    service.isolation = kbox_contract::contract::IsolationLevel::Sandbox;
    service.semantics_profile_digest =
        Some(crate::kbox_contract_driver::NATIVE_GUEST_PROFILE_DIGEST.into());
    let explicit = request.bind_service("owned-unit".into(), 7).unwrap();
    assert_eq!(
        explicit.semantics_profile_digest,
        request.service.as_ref().unwrap().semantics_profile_digest
    );
    assert_eq!(explicit.process, ordinary.process);
    assert_eq!(explicit.resources, ordinary.resources);
    assert_eq!(explicit.mounts, ordinary.mounts);
}

#[test]
fn engine_refuses_unknown_product_fields_and_legacy_intents() {
    let request = intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    let encoded = serde_json::to_value(&request).unwrap();
    for field in ["workload", "assembly"] {
        let mut unknown = encoded.clone();
        unknown[field] = serde_json::json!({"extension": "client-owned"});
        assert!(serde_json::from_value::<EngineIntent>(unknown).is_err());
    }
    let mut legacy = request;
    legacy.schema = "kbox.engine.intent.v1".into();
    assert_eq!(
        legacy.validate().unwrap_err().code,
        EngineErrorCode::UnsupportedVersion
    );
}

#[test]
fn generic_service_does_not_reserve_client_installation_paths() {
    use kbox_contract::contract::{RuntimeMount, RuntimeMountSource};
    let mut request = intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    request.service.as_mut().unwrap().mounts = vec![RuntimeMount {
        name: "client-files".into(),
        source: RuntimeMountSource::Tmpfs { size_bytes: 1024 },
        target: "/opt".into(),
        read_only: false,
    }];
    request.validate().unwrap();
}

#[test]
fn journal_replays_exact_request_after_restart_and_keeps_engine_identity() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let key = journal.engine_id().clone();
    let request = intent(&key, 1, DesiredState::Up);
    let accepted = journal.accept(request.clone()).unwrap();
    drop(journal);
    let mut journal = EngineJournal::open(path).unwrap();
    assert_eq!(journal.engine_id(), &key);
    assert_eq!(journal.accept(request.clone()).unwrap(), accepted);
    let mut conflict = request;
    conflict.plan_digest = format!("sha256:{}", "2".repeat(64));
    assert_eq!(
        journal.accept(conflict).unwrap_err().code,
        EngineErrorCode::RequestConflict
    );
}

#[test]
fn newer_stop_can_arrive_before_old_up_and_old_up_cannot_be_accepted() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = EngineJournal::open(root.path().join("journal.json")).unwrap();
    let key = journal.engine_id().clone();
    journal.accept(intent(&key, 2, DesiredState::Down)).unwrap();
    assert_eq!(
        journal
            .accept(intent(&key, 1, DesiredState::Up))
            .unwrap_err()
            .code,
        EngineErrorCode::StaleIntent
    );
    assert_eq!(
        journal
            .current(&EngineKey::parse("instance").unwrap())
            .unwrap()
            .intent
            .desired_state,
        DesiredState::Down
    );
}

#[test]
fn unknown_engine_and_reused_revision_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = EngineJournal::open(root.path().join("journal.json")).unwrap();
    let key = journal.engine_id().clone();
    assert_eq!(
        journal
            .accept(intent(
                &EngineKey::parse("different").unwrap(),
                1,
                DesiredState::Up
            ))
            .unwrap_err()
            .code,
        EngineErrorCode::Forbidden
    );
    journal.accept(intent(&key, 1, DesiredState::Up)).unwrap();
    let mut request = intent(&key, 1, DesiredState::Up);
    request.request_id = EngineKey::parse("different-request").unwrap();
    assert_eq!(
        journal.accept(request).unwrap_err().code,
        EngineErrorCode::RequestConflict
    );
}

#[tokio::test]
async fn journal_revision_guard_fences_pending_backend_apply_after_stop_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let key = journal.engine_id().clone();
    let request = intent(&key, 1, DesiredState::Up);
    journal.accept(request.clone()).unwrap();
    let record = journal.request(&request.request_id).unwrap().clone();
    let backend_request = RuntimeApplyRequest {
        schema: RuntimeApplyRequest::SCHEMA.into(),
        request_id: record.runtime_request_id,
        spec: request
            .bind_service(journal.unit_id(&request.instance_id), record.generation)
            .unwrap(),
        deadline_at_ms: None,
        restore_from_snapshot_operation_id: None,
    };
    let shared = Arc::new(Mutex::new(journal));
    let guard = EngineRevisionGuard::new(shared.clone());
    guard.check_apply(&backend_request).await.unwrap();
    let mut changed_spec = backend_request.clone();
    changed_spec.spec.process.args.push("changed".into());
    assert!(matches!(
        guard.check_apply(&changed_spec).await,
        Err(RuntimeError::RequestConflict { .. })
    ));
    shared
        .lock()
        .await
        .accept(intent(&key, 2, DesiredState::Down))
        .unwrap();
    assert!(matches!(
        guard.check_apply(&backend_request).await,
        Err(RuntimeError::RequestConflict { .. })
    ));
    drop(guard);
    drop(shared);
    let restarted =
        EngineRevisionGuard::new(Arc::new(Mutex::new(EngineJournal::open(path).unwrap())));
    assert!(matches!(
        restarted.check_apply(&backend_request).await,
        Err(RuntimeError::RequestConflict { .. })
    ));
}

#[test]
fn journal_has_one_owner_until_drop() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("private/journal.json");
    let journal = EngineJournal::open(path.clone()).unwrap();
    let engine_id = journal.engine_id().clone();
    assert_eq!(
        EngineJournal::open(path.clone()).unwrap_err().code,
        EngineErrorCode::RequestConflict
    );
    drop(journal);
    assert_eq!(EngineJournal::open(path).unwrap().engine_id(), &engine_id);
}

#[test]
fn persistence_failure_requires_reopen_before_retry_or_backend_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let backup = root.path().join("original.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let key = journal.engine_id().clone();
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let request = intent(&key, 1, DesiredState::Up);
    assert!(journal.accept(request.clone()).is_err());
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(&backup, &path).unwrap();
    assert_eq!(
        journal.accept(request.clone()).unwrap_err().code,
        EngineErrorCode::Unknown
    );
    assert_eq!(
        journal.current_requests().unwrap_err().code,
        EngineErrorCode::Unknown
    );
    drop(journal);
    EngineJournal::open(path).unwrap().accept(request).unwrap();
}

#[test]
fn corrupt_journal_is_retained_and_does_not_reset_engine_identity() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    std::fs::write(&path, b"broken").unwrap();
    assert!(EngineJournal::open(path.clone()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"broken");
}

#[test]
fn journal_rejects_a_stale_instance_index_instead_of_reviving_old_up() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let key = journal.engine_id().clone();
    journal.accept(intent(&key, 1, DesiredState::Up)).unwrap();
    journal.accept(intent(&key, 2, DesiredState::Down)).unwrap();
    drop(journal);
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    stored["instances"]["instance"] = "request-1".into();
    let bytes = serde_json::to_vec(&stored).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(
        EngineJournal::open(path.clone()).unwrap_err().code,
        EngineErrorCode::InvalidRequest
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn service_template_remains_compatible_with_current_runtime_schema() {
    let spec: RuntimeUnitSpec = template().bind("owned-unit".into(), 1).unwrap();
    spec.validate().unwrap();
}

#[test]
fn unsupported_journal_is_retained_without_replacing_identity_or_records() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let key = journal.engine_id().clone();
    journal.accept(intent(&key, 1, DesiredState::Up)).unwrap();
    drop(journal);
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    stored["schema"] = "kbox.engine.journal.v1".into();
    let bytes = serde_json::to_vec(&stored).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(
        EngineJournal::open(path.clone()).unwrap_err().code,
        EngineErrorCode::UnsupportedVersion
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
