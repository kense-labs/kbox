use super::*;
use kbox_core::vmm::FsMount;

#[test]
fn readonly_exports_are_pinned_and_overlapping_writable_grants_refuse() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("readonly");
    let writable = root.join("writable");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&writable).unwrap();
    let spec = InstanceSpec {
        rootfs: RootfsSource::directory(&writable),
        fs_mounts: vec![FsMount {
            tag: "vol0".into(),
            host_path: source.clone(),
            read_only: true,
        }],
        ..Default::default()
    };
    let (prepared, guard) = prepare(spec.clone()).unwrap();
    assert_eq!(
        prepared.fs_mounts[0].host_path,
        guard.pinned_path(0).unwrap()
    );
    assert!(prepared.fs_mounts[0].read_only);
    let mut overlap = spec.clone();
    overlap.rootfs = RootfsSource::directory(&root);
    assert!(prepare(overlap).is_err());
    let mut snapshot = spec;
    snapshot.restore_from = Some("restore".into());
    assert!(prepare(snapshot).is_err());
    std::fs::rename(&source, root.join("moved")).unwrap();
    std::fs::create_dir(&source).unwrap();
    assert!(guard.reverify().is_err());
}
