use super::*;

#[test]
fn volume_names_cannot_create_or_reclaim_a_caller_directory() {
    for name in [
        "../caller",
        "nested/name",
        "nested\\name",
        "name/",
        "name/.",
        ".",
        "..",
        "",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir(root.join("caller")).unwrap();
        std::fs::write(root.join("caller/canary"), b"caller-data").unwrap();
        let store = VolumeStore::new(root.join("volumes.json"), root.join("volumes"));
        assert!(
            store.create(VolumeConfig::new(name, "")).is_err(),
            "{name:?} was admitted"
        );
        assert!(!root.join("volumes.json").exists());
        assert!(!root.join("volumes").exists());
        assert_eq!(
            std::fs::read(root.join("caller/canary")).unwrap(),
            b"caller-data"
        );
    }
}

#[test]
fn poisoned_catalog_cannot_delete_a_caller_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("volumes")).unwrap();
    std::fs::create_dir(root.join("caller")).unwrap();
    std::fs::write(root.join("caller/canary"), b"caller-data").unwrap();
    let mut entries = HashMap::new();
    entries.insert("../caller".into(), VolumeConfig::new("../caller", ""));
    let bytes = serde_json::to_vec(&VolumesFile { volumes: entries }).unwrap();
    let path = root.join("volumes.json");
    std::fs::write(&path, &bytes).unwrap();
    let store = VolumeStore::new(path.clone(), root.join("volumes"));
    assert!(store.remove("../caller", false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::read(root.join("caller/canary")).unwrap(),
        b"caller-data"
    );
}

#[test]
fn named_volume_mutations_refuse_a_corrupt_catalog_without_moving_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("volumes.json");
    std::fs::write(&path, b"invalid-catalog").unwrap();
    let store = VolumeStore::new(path.clone(), directory.path().join("volumes"));
    assert!(store.create(VolumeConfig::new("dataset-a", "")).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"invalid-catalog");
    assert!(!directory.path().join("volumes").exists());
}

#[test]
fn normal_volume_reads_and_prune_do_not_quarantine_a_corrupt_catalog() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("volumes.json");
    std::fs::write(&path, b"invalid-catalog").unwrap();
    let store = VolumeStore::new(path.clone(), directory.path().join("volumes"));
    assert!(store.get("dataset-a").is_err());
    assert!(store.list().is_err());
    assert!(store.prune().is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"invalid-catalog");
}

#[test]
fn alternate_case_name_cannot_claim_an_existing_allocation() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let store = VolumeStore::new(root.join("volumes.json"), root.join("volumes"));
    let first = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let source = PathBuf::from(&first.mount_point);
    std::fs::write(source.join("canary"), b"caller-data").unwrap();
    let alias_exists = store.volume_dir("DATASET-A").exists();
    let result = store.create(VolumeConfig::new("DATASET-A", ""));
    if alias_exists {
        assert!(
            result.is_err(),
            "two names claimed one filesystem allocation"
        );
        assert_eq!(store.list().unwrap().len(), 1);
    } else {
        assert!(result.is_ok());
        assert_eq!(store.list().unwrap().len(), 2);
    }
    assert_eq!(
        std::fs::read(source.join("canary")).unwrap(),
        b"caller-data"
    );
}

#[test]
fn allocation_identity_is_store_issued_preserved_and_not_reused_from_copied_config() {
    let directory = tempfile::tempdir().unwrap();
    let store = VolumeStore::new(
        directory.path().join("volumes.json"),
        directory.path().join("volumes"),
    );
    let first = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let id = first.labels[ALLOCATION_ID_LABEL].clone();
    uuid::Uuid::parse_str(&id).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    assert!(store
        .modify("dataset-a", |volume| {
            volume.labels.remove(ALLOCATION_ID_LABEL);
        })
        .is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_eq!(
        store
            .get_or_create(VolumeConfig::new("dataset-a", ""))
            .unwrap()
            .labels[ALLOCATION_ID_LABEL],
        id
    );
    let previous = store.remove("dataset-a", false).unwrap();
    assert_ne!(
        store.create(previous).unwrap().labels[ALLOCATION_ID_LABEL],
        id
    );
}

#[test]
fn legacy_catalog_without_allocation_label_is_read_without_rewriting() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("volumes.json");
    let store = VolumeStore::new(path.clone(), directory.path().join("volumes"));
    let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    let mut entries = store.load().unwrap();
    entries
        .get_mut("dataset-a")
        .unwrap()
        .labels
        .remove(ALLOCATION_ID_LABEL);
    let data = serde_json::to_vec(&VolumesFile { volumes: entries }).unwrap();
    std::fs::write(&path, &data).unwrap();
    assert_eq!(
        store.get("dataset-a").unwrap().unwrap().mount_point,
        volume.mount_point
    );
    assert_eq!(std::fs::read(&path).unwrap(), data);
}

#[test]
fn invalid_allocation_label_and_catalog_symlink_refuse_without_changing_targets() {
    for value in ["invalid", "00000000-0000-0000-0000-000000000000"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("volumes.json");
        let store = VolumeStore::new(path.clone(), directory.path().join("volumes"));
        let mut volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
        volume
            .labels
            .insert(ALLOCATION_ID_LABEL.into(), value.into());
        let bytes = serde_json::to_vec(&VolumesFile {
            volumes: HashMap::from([("dataset-a".into(), volume)]),
        })
        .unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(store.get("dataset-a").is_err());
        assert!(store.prune().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    #[cfg(unix)]
    {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("not-created");
        let path = directory.path().join("volumes.json");
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let store = VolumeStore::new(path.clone(), directory.path().join("volumes"));
        assert!(store.create(VolumeConfig::new("dataset-a", "")).is_err());
        assert!(!target.exists());
        assert!(std::fs::symlink_metadata(path)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}

#[test]
fn two_volumes_cannot_publish_one_allocation_identity() {
    let directory = tempfile::tempdir().unwrap();
    let store = VolumeStore::new(
        directory.path().join("volumes.json"),
        directory.path().join("volumes"),
    );
    let first = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
    store.create(VolumeConfig::new("dataset-b", "")).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let mut entries = store.load().unwrap();
    entries.get_mut("dataset-b").unwrap().labels.insert(
        ALLOCATION_ID_LABEL.into(),
        first.labels[ALLOCATION_ID_LABEL].clone(),
    );
    assert!(store.save(&entries).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_eq!(store.list().unwrap().len(), 2);
}
