use super::*;
use kbox_core::{
    volume::VolumeConfig, BoxConfig, CreateExecutionRequest, ExecutionId, OperationId,
};

pub(super) fn fixture() -> (tempfile::TempDir, PathBuf, crate::VolumeStore, BoxRecord) {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = crate::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let mut request = CreateExecutionRequest {
        external_sandbox_id: "named-admission".into(),
        config: BoxConfig {
            image: "alpine:latest".into(),
            network: NetworkMode::None,
            resources: ResourceConfig {
                vcpus: 1,
                memory_mb: 512,
                ..Default::default()
            },
            volumes: vec![format!("{}:/data:rw", volume.mount_point)],
            ..Default::default()
        },
        labels: Default::default(),
        policy: Default::default(),
        rootfs_snapshot_id: None,
    };
    request.policy.volume_names = vec![volume.name];
    let record = super::super::record::build_managed_record(
        &home,
        &ExecutionId::new("00000000-0000-4000-8000-000000000011").unwrap(),
        OperationId::new("named-admission").unwrap(),
        request,
        chrono::Utc::now(),
    )
    .unwrap();
    (temporary, home, store, record)
}

#[test]
fn hvf_named_volume_profile_admits_existing_plain_store_mount() {
    let (_temporary, home, store, record) = fixture();
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    super::super::oci_storage_attachments::validate_box_named_volume_sources(&home, &record)
        .unwrap();
    assert!(store
        .get("dataset-a")
        .unwrap()
        .unwrap()
        .in_use_by
        .is_empty());
    assert!(!home.join("boxes").exists());
}

#[test]
fn named_preflight_rejects_missing_duplicate_and_drifted_bindings_without_mutation() {
    for case in ["missing", "duplicate", "declaration-drift"] {
        let (_temporary, home, store, mut record) = fixture();
        let before = std::fs::read(store.path()).unwrap();
        match case {
            "missing" => record
                .managed_execution
                .as_mut()
                .unwrap()
                .request
                .config
                .volumes
                .clear(),
            "duplicate" => {
                let source = store.get("dataset-a").unwrap().unwrap().mount_point;
                record
                    .managed_execution
                    .as_mut()
                    .unwrap()
                    .request
                    .config
                    .volumes
                    .push(format!("{source}:/other:ro"));
            }
            "declaration-drift" => record.volume_names.clear(),
            _ => unreachable!(),
        }
        assert!(
            super::super::oci_storage_attachments::validate_box_named_volume_sources(
                &home, &record
            )
            .is_err(),
            "{case}"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}
