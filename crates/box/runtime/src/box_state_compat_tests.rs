use super::tests::record;
use super::*;

fn managed_record(id: &str, operation: &str) -> BoxRecord {
    super::tests::managed_record(id, kbox_core::OperationId::new(operation).unwrap())
}

fn file_record() -> BoxRecord {
    let mut record = managed_record("file", "file-create");
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    metadata.caller_files.insert(
        PathBuf::from("/caller/文件 input"),
        koci_sdk::FileSourceIdentity::new(1, 2).unwrap(),
    );
    record
}

#[test]
fn legacy_file_store_migrates_atomically_without_changing_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("有空格 state.json");
    let records = vec![
        record("legacy"),
        managed_record("no-file", "old-create"),
        file_record(),
    ];
    let original = serde_json::to_vec_pretty(&records).unwrap();
    std::fs::write(&path, &original).unwrap();
    let loaded = BoxStateStore::load_readonly(&path).unwrap();
    assert!(!path.is_dir(), "readonly load must not migrate");
    loaded.save().unwrap();
    assert!(
        path.is_dir(),
        "file authority requires an old-reader I/O fence"
    );
    assert!(std::fs::read_to_string(&path).is_err());
    assert_eq!(
        std::fs::read(path.join("records.v1.json")).unwrap(),
        original
    );
    let loaded = BoxStateStore::load(&path).unwrap();
    assert_eq!(
        serde_json::to_vec_pretty(loaded.records()).unwrap(),
        original
    );
    assert_eq!(loaded.path(), path);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2); // store directory, lock
}

#[test]
fn no_file_store_retains_legacy_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![managed_record("plain", "plain-create")])
        .save()
        .unwrap();
    assert!(path.is_file());
    assert_eq!(
        BoxStateStore::load_readonly(&path).unwrap().records().len(),
        1
    );
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn oci_anonymous_store_fences_unaware_readers_without_changing_ownership_or_route() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    let mut value = managed_record("raw-volume", "raw-create");
    value.managed_execution.as_mut().unwrap().runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    value.anonymous_volumes = vec!["anon_raw_plan".into()];
    let expected = serde_json::to_vec_pretty(&vec![value]).unwrap();
    std::fs::write(&path, &expected).unwrap();
    let snapshot = BoxStateStore::load_readonly(&path).unwrap();
    assert!(path.is_file());
    assert_eq!(
        serde_json::to_vec_pretty(snapshot.records()).unwrap(),
        expected
    );
    let loaded = BoxStateStore::load(&path).unwrap();
    assert!(
        path.is_dir(),
        "opaque anonymous data needs the existing unaware-reader fence"
    );
    assert_eq!(
        serde_json::to_vec_pretty(loaded.records()).unwrap(),
        expected
    );
    assert!(std::fs::read_to_string(&path).is_err());
    assert_eq!(
        std::fs::read(path.join("records.v1.json")).unwrap(),
        expected
    );
}

#[test]
fn file_store_empty_after_cleanup_keeps_old_reader_fence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![file_record()])
        .save()
        .unwrap();
    BoxStateStore::modify(&path, |store| {
        store.records_mut().clear();
        Ok(())
    })
    .unwrap();
    assert!(path.is_dir());
    assert!(BoxStateStore::load_or_quarantine(&path)
        .unwrap()
        .records()
        .is_empty());
    assert!(std::fs::read_to_string(path).is_err());
}

#[test]
fn file_store_corruption_never_quarantines_authority_to_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![file_record()])
        .save()
        .unwrap();
    let data = path.join("records.v1.json");
    std::fs::write(&data, b"invalid json").unwrap();
    assert_eq!(
        BoxStateStore::load_or_quarantine(&path).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert!(BoxStateStore::modify_or_quarantine(&path, |store| {
        store.records_mut().clear();
        Ok::<(), std::io::Error>(())
    })
    .is_err());
    assert_eq!(std::fs::read(data).unwrap(), b"invalid json");
    assert!(path.is_dir());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn file_store_missing_records_and_future_layout_refuse_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![file_record()])
        .save()
        .unwrap();
    std::fs::remove_file(path.join("records.v1.json")).unwrap();
    assert!(BoxStateStore::load_or_quarantine(&path).is_err());
    assert!(BoxStateStore::from_records(&path, vec![]).save().is_err());
    std::fs::write(path.join("records.v2.json"), b"[]").unwrap();
    assert!(BoxStateStore::load_or_quarantine(&path).is_err());
    assert!(BoxStateStore::from_records(&path, vec![]).save().is_err());
    assert_eq!(std::fs::read(path.join("records.v2.json")).unwrap(), b"[]");
}

#[test]
fn file_store_concurrent_migration_keeps_all_records_and_one_container() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    std::fs::write(&path, serde_json::to_vec(&vec![file_record()]).unwrap()).unwrap();
    let threads: Vec<_> = (0..8)
        .map(|index| {
            let path = path.clone();
            std::thread::spawn(move || {
                BoxStateStore::modify(&path, |store| {
                    store.records_mut().push(record(&format!("legacy-{index}")));
                    Ok(())
                })
                .unwrap()
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(BoxStateStore::load(&path).unwrap().records().len(), 9);
    assert!(path.is_dir());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn mutable_load_adopts_old_file_records_and_stale_flat_writer_cannot_replace_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    let original = serde_json::to_vec_pretty(&vec![file_record()]).unwrap();
    std::fs::write(&path, &original).unwrap();
    let loaded = BoxStateStore::load_or_quarantine(&path).unwrap();
    assert_eq!(loaded.records().len(), 1);
    assert!(path.is_dir());
    let stale = path.with_extension("json.tmp");
    std::fs::write(&stale, b"[]").unwrap();
    assert!(std::fs::rename(&stale, &path).is_err());
    assert_eq!(
        std::fs::read(path.join("records.v1.json")).unwrap(),
        original
    );
    assert_eq!(std::fs::read(stale).unwrap(), b"[]");
}

#[test]
fn file_store_symlinked_records_refuse_without_following_or_quarantine() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![file_record()])
        .save()
        .unwrap();
    let outside = dir.path().join("canary");
    std::fs::write(&outside, b"[]").unwrap();
    std::fs::remove_file(path.join("records.v1.json")).unwrap();
    std::os::unix::fs::symlink(&outside, path.join("records.v1.json")).unwrap();
    assert!(BoxStateStore::load_or_quarantine(&path).is_err());
    assert!(BoxStateStore::from_records(&path, vec![]).save().is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"[]");
    assert!(path.is_dir());
}

#[test]
fn file_store_concurrent_record_loss_never_reports_an_empty_collection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.json");
    BoxStateStore::from_records(&path, vec![file_record()])
        .save()
        .unwrap();
    let data = path.join("records.v1.json");
    let bytes = std::fs::read(&data).unwrap();
    let writer = std::thread::spawn(move || {
        for _ in 0..4000 {
            std::fs::remove_file(&data).unwrap();
            std::fs::write(&data, &bytes).unwrap();
        }
    });
    let mut empty = false;
    for _ in 0..4000 {
        if let Ok(store) = BoxStateStore::load_readonly(&path) {
            empty |= store.records().is_empty();
        }
    }
    writer.join().unwrap();
    assert!(
        !empty,
        "lost directory records must be an I/O error, never an empty authoritative store"
    );
    assert_eq!(
        BoxStateStore::load_readonly(&path).unwrap().records().len(),
        1
    );
}

#[test]
fn file_authority_cannot_change_in_managed_transition() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::ManagedExecutionStore::new(dir.path().join("boxes.json"));
    let record = file_record();
    let id = kbox_core::ExecutionId::new(record.id.clone()).unwrap();
    store.reserve(record).unwrap();
    let before = serde_json::to_vec(&store.list().unwrap()).unwrap();
    let result = store.transition_with(
        &id,
        kbox_core::ExecutionGeneration::INITIAL,
        crate::ManagedExecutionState::Created,
        crate::ManagedExecutionState::Starting,
        |record| {
            record
                .managed_execution
                .as_mut()
                .unwrap()
                .caller_files
                .clear();
        },
    );
    assert!(matches!(
        result,
        Err(crate::ManagedExecutionStoreError::InvalidRecord(_))
    ));
    assert_eq!(serde_json::to_vec(&store.list().unwrap()).unwrap(), before);
}
