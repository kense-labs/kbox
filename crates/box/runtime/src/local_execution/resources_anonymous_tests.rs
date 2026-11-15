use super::*;
use crate::{BoxStateStore, ManagedRuntimeRoute};
use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionGeneration, ExecutionId, OperationId};

fn fixture(auto_remove: bool) -> (tempfile::TempDir, PathBuf, BoxRecord, VolumeStore) {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().canonicalize().unwrap();
    let mut request = CreateExecutionRequest {
        external_sandbox_id: "ephemeral-volume".into(),
        config: BoxConfig {
            image: "fixture:local".into(),
            network: NetworkMode::None,
            ..Default::default()
        },
        labels: Default::default(),
        policy: Default::default(),
        rootfs_snapshot_id: None,
    };
    request.policy.auto_remove = auto_remove;
    let mut record = super::super::record::build_managed_record(
        &home,
        &ExecutionId::new("00000000-0000-4000-8000-000000000012").unwrap(),
        OperationId::new("ephemeral-volume").unwrap(),
        request,
        chrono::Utc::now(),
    )
    .unwrap();
    record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::OciSdk;
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let (volume, _) = store.claim_anonymous("anon_ephemeral", &record.id).unwrap();
    std::fs::write(
        Path::new(&volume.mount_point).join("data"),
        b"retain-until-authorized",
    )
    .unwrap();
    record.anonymous_volumes = vec![volume.name];
    BoxStateStore::from_records(home.join("boxes.json"), vec![record.clone()])
        .save()
        .unwrap();
    (directory, home, record, store)
}

#[tokio::test]
async fn ephemeral_cleanup_uses_existing_authority_and_keeps_other_allocations() {
    let (_temporary, home, record, store) = fixture(true);
    store.claim_anonymous("anon_other", "other-owner").unwrap();
    store
        .create(kbox_core::volume::VolumeConfig::new("named-caller", ""))
        .unwrap();
    cleanup_ephemeral_oci_anonymous(&home, &record)
        .await
        .unwrap();
    assert!(store.get("anon_ephemeral").unwrap().is_none());
    assert!(store.get("anon_other").unwrap().is_some());
    assert!(store.get("named-caller").unwrap().is_some());
    assert!(BoxStateStore::load_readonly(home.join("boxes.json"))
        .unwrap()
        .find_by_id(&record.id)
        .is_some());
}

#[tokio::test]
async fn ephemeral_cleanup_cannot_delete_after_state_or_generation_ownership_moves() {
    for advance_generation in [false, true] {
        let (_temporary, home, old, store) = fixture(true);
        let mut current = old.clone();
        current.status = "stopped".into();
        if advance_generation {
            current.managed_execution.as_mut().unwrap().generation =
                ExecutionGeneration::new(2).unwrap();
        }
        BoxStateStore::from_records(home.join("boxes.json"), vec![current.clone()])
            .save()
            .unwrap();
        let catalog = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            cleanup_ephemeral_oci_anonymous(&home, &old).await,
            Err(ExecutionManagerError::Conflict { .. })
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), catalog);
        let volume = store.get("anon_ephemeral").unwrap().unwrap();
        assert_eq!(
            std::fs::read(Path::new(&volume.mount_point).join("data")).unwrap(),
            b"retain-until-authorized"
        );
    }
}

#[tokio::test]
async fn retained_and_explicit_restart_cleanup_preserve_anonymous_data() {
    let (_temporary, home, record, store) = fixture(false);
    cleanup_ephemeral_oci_anonymous(&home, &record)
        .await
        .unwrap();
    assert!(store.get("anon_ephemeral").unwrap().is_some());
    let (_temporary, home, mut record, store) = fixture(true);
    record.status = crate::ManagedExecutionState::RestartStopping
        .as_status()
        .into();
    record.managed_execution.as_mut().unwrap().pending_operation =
        Some(crate::ManagedExecutionOperation::Restart {
            operation_id: OperationId::new("retain-restart").unwrap(),
            source_generation: ExecutionGeneration::INITIAL,
            source_state: crate::ManagedExecutionState::Stopped,
            stop_timeout_secs: None,
        });
    cleanup_ephemeral_oci_anonymous(&home, &record)
        .await
        .unwrap();
    assert!(store.get("anon_ephemeral").unwrap().is_some());
}
