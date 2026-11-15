use super::*;
use kbox_core::volume::VolumeConfig;
use koci_sdk::ProcessIo;
use serde_json::json;

fn bundle(home: &Path, source: &Path) -> OciBundle {
    let directory = home.join("bundle");
    std::fs::create_dir_all(directory.join("rootfs")).unwrap();
    OciBundle::from_json(&directory, serde_json::to_string(&json!({
        "ociVersion":"1.3.0", "root":{"path":"rootfs"},
        "process":{"cwd":"/","args":["/bin/true"],"user":{"uid":0,"gid":0}},
        "mounts":[{"destination":"/data","type":"bind","source":source,"options":["rbind","rw"]}]
    })).unwrap()).unwrap()
}

fn attach(home: &Path, source: &Path) -> ExecutionManagerResult<CreateAttachments> {
    let bundle = bundle(home, source);
    let mut record = super::tests::make_record(home, "11111111-1111-4111-8111-111111111111");
    record.volume_names = vec!["dataset-a".into()];
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    attach_box_owned_volume_storage(&bundle, attachments, home, &record, &[])
}

#[test]
fn named_storage_identity_is_stable_per_allocation_and_changes_after_recreation() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let first = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let source = PathBuf::from(&first.mount_point);
    let initial = attach(&home, &source).unwrap().storage()[0]
        .identity()
        .clone();
    assert_eq!(
        *attach(&home, &source).unwrap().storage()[0].identity(),
        initial
    );
    store.remove("dataset-a", false).unwrap();
    let mut next = VolumeConfig::new("dataset-a", "");
    next.created_at = "2026-11-15T00:00:00Z".into();
    assert_ne!(first.created_at, next.created_at);
    store.create(next).unwrap();
    assert_ne!(
        *attach(&home, &source).unwrap().storage()[0].identity(),
        initial,
        "a mutable volume name cannot stand in for an allocation incarnation"
    );
}

#[test]
fn recreating_a_volume_from_its_previous_config_gets_a_new_allocation_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let first = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let source = PathBuf::from(&first.mount_point);
    let initial = attach(&home, &source).unwrap().storage()[0]
        .identity()
        .clone();
    let previous_config = store.remove("dataset-a", false).unwrap();
    store.create(previous_config).unwrap();
    assert_ne!(
        *attach(&home, &source).unwrap().storage()[0].identity(),
        initial
    );
}

#[test]
fn legacy_named_storage_delegation_preserves_unlabelled_catalog_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let mut entries = store.load().unwrap();
    entries
        .get_mut("dataset-a")
        .unwrap()
        .labels
        .remove(crate::volume::ALLOCATION_ID_LABEL);
    let bytes = serde_json::to_vec(&json!({"volumes":entries})).unwrap();
    std::fs::write(store.path(), &bytes).unwrap();
    let source = PathBuf::from(&volume.mount_point);
    let first = attach(&home, &source).unwrap();
    let second = attach(&home, &source).unwrap();
    assert_eq!(
        first.storage()[0].identity(),
        second.storage()[0].identity()
    );
    assert_ne!(first.storage()[0].identity().as_str(), "dataset-a");
    assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
}

#[test]
fn named_storage_corrupt_catalog_refuses_without_quarantining_or_changing_data() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let source = PathBuf::from(&volume.mount_point);
    std::fs::write(source.join("canary"), b"caller-data").unwrap();
    std::fs::write(home.join("volumes.json"), b"invalid-catalog").unwrap();
    assert!(attach(&home, &source).is_err());
    assert_eq!(
        std::fs::read(home.join("volumes.json")).unwrap(),
        b"invalid-catalog"
    );
    assert_eq!(
        std::fs::read(source.join("canary")).unwrap(),
        b"caller-data"
    );
}

#[test]
fn named_storage_rejects_unenforced_driver_quota_and_catalog_path_escape() {
    for case in ["driver", "quota", "escape", "name"] {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().canonicalize().unwrap();
        let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
        let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
        let mut entries = store.load().unwrap();
        let entry = entries.get_mut("dataset-a").unwrap();
        match case {
            "driver" => entry.driver = "unimplemented".into(),
            "quota" => entry.size_limit = 1,
            "escape" => {
                entry.mount_point = home.join("caller-outside").to_string_lossy().into_owned();
                std::fs::create_dir(&entry.mount_point).unwrap();
            }
            "name" => entry.name = "different-allocation".into(),
            _ => unreachable!(),
        }
        let source = PathBuf::from(&entry.mount_point);
        let before = serde_json::to_vec(&json!({"volumes":entries})).unwrap();
        std::fs::write(home.join("volumes.json"), &before).unwrap();
        assert!(attach(&home, &source).is_err(), "{case} was admitted");
        assert_eq!(std::fs::read(home.join("volumes.json")).unwrap(), before);
        assert!(Path::new(&volume.mount_point).is_dir());
    }
}

#[test]
fn anonymous_raw_storage_uses_exact_allocation_identity_and_retains_box_cleanup_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().canonicalize().unwrap();
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let record = super::tests::make_record(&home, "11111111-1111-4111-8111-111111111111");
    let (volume, _) = store.claim_anonymous("anon_fixture", &record.id).unwrap();
    let disk = crate::volume::anonymous_ext4_disk_path(Path::new(&volume.mount_point));
    std::fs::create_dir_all(disk.parent().unwrap()).unwrap();
    std::fs::write(&disk, b"raw image").unwrap();
    let bundle = OciBundle::from_json(home.join("bundle"), json!({
        "ociVersion":"1.3.0", "root":{"path":"rootfs"},
        "process":{"cwd":"/","args":["/bin/true"],"user":{"uid":0,"gid":0}},
        "annotations":{koci_sdk::RAW_STORAGE_IMAGES_EXTENSION:koci_sdk::RAW_STORAGE_IMAGES_VALUE},
        "mounts":[{"type":"ext4","source":disk,"destination":"/seed","options":["rw"]}]
    }).to_string()).unwrap();
    let base = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let names = vec![volume.name.clone()];
    let attached =
        attach_box_owned_volume_storage(&bundle, base.clone(), &home, &record, &names).unwrap();
    let storage = &attached.storage()[0];
    assert_eq!(storage.ownership(), koci_sdk::StorageOwnership::Caller);
    assert_eq!(storage.cleanup(), koci_sdk::StorageCleanup::DetachOnly);
    assert_eq!(
        storage.access_mode(),
        koci_sdk::StorageAccessMode::ReadWrite
    );
    let allocation =
        uuid::Uuid::parse_str(&volume.labels[crate::volume::ALLOCATION_ID_LABEL]).unwrap();
    assert_eq!(
        storage.identity().as_str(),
        format!("kbox.volume.v1.{}", allocation.simple())
    );
    reject_unclassified_bind_mounts(&bundle, attached).unwrap();
    let mut wrong_owner = record.clone();
    wrong_owner.id = "22222222-2222-4222-8222-222222222222".into();
    assert!(
        attach_box_owned_volume_storage(&bundle, base.clone(), &home, &wrong_owner, &names)
            .is_err()
    );
    let original = std::fs::read(store.path()).unwrap();
    let mut catalog: serde_json::Value = serde_json::from_slice(&original).unwrap();
    catalog["volumes"][&volume.name]["labels"]
        .as_object_mut()
        .unwrap()
        .remove(crate::volume::ALLOCATION_ID_LABEL);
    std::fs::write(store.path(), serde_json::to_vec(&catalog).unwrap()).unwrap();
    assert!(
        attach_box_owned_volume_storage(&bundle, base.clone(), &home, &record, &names).is_err()
    );
    std::fs::write(store.path(), original).unwrap();
    std::fs::remove_file(&disk).unwrap();
    assert!(attach_box_owned_volume_storage(&bundle, base, &home, &record, &names).is_err());
    assert!(Path::new(&volume.mount_point).exists());
}
