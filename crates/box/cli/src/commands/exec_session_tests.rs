use super::{configured_process_limit_user, uses_managed_session};
use crate::state::BoxRecord;
use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionGeneration, OperationId};
use kbox_runtime::{ManagedExecutionMetadata, ManagedRuntimeRoute};

#[test]
fn per_uid_cli_defaults_use_managed_identity_and_keep_other_defaults() {
    let mut record =
        crate::test_helpers::fixtures::make_record("nproc-cli-id", "nproc-cli", "running", None);
    record.user = Some("0".into());
    let mut config = BoxConfig {
        image: record.image.clone(),
        user: Some("1001:2002".into()),
        cap_drop: vec!["ALL".into()],
        ..Default::default()
    };
    config.resource_limits.ulimits = vec!["NPROC=16:16".into()];
    let mut metadata = ManagedExecutionMetadata::new(
        OperationId::new("nproc-cli-create").unwrap(),
        ExecutionGeneration::new(7).unwrap(),
        CreateExecutionRequest {
            external_sandbox_id: "nproc-cli-external".into(),
            config,
            labels: Default::default(),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        },
    )
    .unwrap();
    metadata.runtime_route = ManagedRuntimeRoute::OciSdk;
    record.managed_execution = Some(metadata);
    for limit in ["NPROC=16:16", "SIGPENDING=16:16", "MSGQUEUE=2048:2048"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![limit.into()];
        assert_eq!(
            configured_process_limit_user(&record).as_deref(),
            Some("1001:2002")
        );
    }
    record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::BoxVm;
    assert_eq!(configured_process_limit_user(&record), None);
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.runtime_route = ManagedRuntimeRoute::OciSdk;
    metadata.request.config.resource_limits.ulimits.clear();
    assert_eq!(configured_process_limit_user(&record), None);
    record.managed_execution = None;
    assert_eq!(configured_process_limit_user(&record), None);
}

#[test]
fn persisted_box_and_oci_sessions_use_the_execution_manager() {
    let mut record: BoxRecord = crate::test_helpers::fixtures::make_record(
        "managed-session-id",
        "managed-session",
        "running",
        None,
    );
    assert!(!uses_managed_session(&record));
    let metadata = ManagedExecutionMetadata::new(
        OperationId::new("managed-session-create").unwrap(),
        ExecutionGeneration::new(7).unwrap(),
        CreateExecutionRequest {
            external_sandbox_id: "managed-session-external".into(),
            config: BoxConfig {
                image: record.image.clone(),
                ..Default::default()
            },
            labels: Default::default(),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        },
    )
    .unwrap();
    for route in [ManagedRuntimeRoute::BoxVm, ManagedRuntimeRoute::OciSdk] {
        let mut routed = metadata.clone();
        routed.runtime_route = route;
        record.managed_execution = Some(routed);
        assert!(
            uses_managed_session(&record),
            "{route:?} bypasses stored security"
        );
    }
}
