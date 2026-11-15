use super::*;
use sha2::{Digest, Sha256};

fn empty_rootfs(parent: &Path) -> OciWorkloadRootfs {
    let image = parent.join("image");
    std::fs::create_dir_all(image.join("blobs/sha256")).unwrap();
    let blob = |bytes: &[u8]| {
        let digest = format!("sha256:{:x}", Sha256::digest(bytes));
        std::fs::write(
            image
                .join("blobs/sha256")
                .join(digest.trim_start_matches("sha256:")),
            bytes,
        )
        .unwrap();
        digest
    };
    let config = serde_json::to_vec(&serde_json::json!({"architecture":"arm64","os":"linux","config":{},"rootfs":{"type":"layers","diff_ids":[]}})).unwrap();
    let manifest = serde_json::to_vec(&serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":blob(&config),"size":config.len()},"layers":[]})).unwrap();
    std::fs::write(
        image.join("oci-layout"),
        br#"{"imageLayoutVersion":"1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(image.join("index.json"), serde_json::to_vec(&serde_json::json!({"schemaVersion":2,"manifests":[{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":blob(&manifest),"size":manifest.len(),"platform":{"os":"linux","architecture":"arm64"}}]})).unwrap()).unwrap();
    OciWorkloadRootfs::load(&crate::oci::OciImage::from_path(&image).unwrap(), parent).unwrap()
}

fn allocation(home: &Path) -> (VolumeStore, VolumeConfig, PlannedAnonymousVolume) {
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let (volume, _) = store.claim_anonymous("anon_fixture", "owner").unwrap();
    let plan = PlannedAnonymousVolume {
        name: volume.name.clone(),
        legacy_name: "anon_legacy".into(),
        guest_path: "/seed".into(),
    };
    (store, volume, plan)
}

#[test]
fn anonymous_disk_publication_is_atomic_private_and_reuse_never_reseeds_data() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let (store, volume, plan) = allocation(&home);
    let plans = vec![(plan, volume.clone())];
    let mut root = empty_rootfs(&home);
    let staging = home.join("staging");
    std::fs::create_dir(&staging).unwrap();
    let catalog = std::fs::read(store.path()).unwrap();
    let staged = stage(&mut root, &plans, 16, &staging).unwrap();
    assert_eq!(staged.len(), 1);
    assert!(!disk_path(&volume).exists());
    publish(&home, "owner", staged).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), catalog);
    let disk = disk_path(&volume);
    let bytes = std::fs::read(&disk).unwrap();
    assert_eq!(
        std::fs::metadata(&disk).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let artifact = crate::rootfs::resume_volume_ext4_artifact(disk.parent().unwrap()).unwrap();
    assert_eq!(
        artifact.manifest.fs_uuid,
        hex::encode(allocation_uuid(&volume).unwrap().into_bytes())
    );
    assert!(stage(&mut root, &plans, 16, &staging).unwrap().is_empty());
    assert_eq!(std::fs::read(&disk).unwrap(), bytes);
    assert!(stage(&mut root, &plans, 32, &staging).is_err());
    assert_eq!(std::fs::read(&disk).unwrap(), bytes);
}

#[test]
fn anonymous_disk_publication_refuses_other_owners_reallocated_names_and_busy_catalogs() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let (store, volume, plan) = allocation(&home);
    let plans = vec![(plan, volume.clone())];
    let mut root = empty_rootfs(&home);
    let staging = home.join("staging");
    std::fs::create_dir(&staging).unwrap();
    let staged = stage(&mut root, &plans, 16, &staging).unwrap();
    let retained = staged[0].1.directory.clone();
    assert!(publish(&home, "other", staged.clone()).is_err());
    assert!(retained.exists());
    let lock = crate::file_lock::FileLock::acquire(store.path()).unwrap();
    assert!(publish(&home, "owner", staged.clone()).is_err());
    drop(lock);
    store.remove_anonymous(&volume.name, "owner").unwrap();
    let (replacement, _) = store.claim_anonymous(&volume.name, "owner").unwrap();
    assert_ne!(
        allocation_uuid(&replacement).unwrap(),
        allocation_uuid(&volume).unwrap()
    );
    assert!(publish(&home, "owner", staged).is_err());
    assert!(retained.exists());
    assert!(!disk_path(&replacement).exists());
    assert!(std::fs::read_dir(&replacement.mount_point)
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn anonymous_disk_stage_preserves_legacy_data_and_require_rejects_path_aliases() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let (store, volume, plan) = allocation(&home);
    let plans = vec![(plan, volume.clone())];
    let mut root = empty_rootfs(&home);
    let staging = home.join("staging");
    std::fs::create_dir(&staging).unwrap();
    let canary = Path::new(&volume.mount_point).join("caller-data");
    std::fs::write(&canary, b"retain").unwrap();
    let catalog = std::fs::read(store.path()).unwrap();
    assert!(stage(&mut root, &plans, 16, &staging).is_err());
    assert_eq!(std::fs::read(&canary).unwrap(), b"retain");
    assert_eq!(std::fs::read(store.path()).unwrap(), catalog);
    let original = home.join("original");
    std::fs::rename(&volume.mount_point, &original).unwrap();
    std::os::unix::fs::symlink(&original, &volume.mount_point).unwrap();
    assert!(store.require_anonymous(&volume.name, "owner").is_err());
    assert_eq!(
        std::fs::read(original.join("caller-data")).unwrap(),
        b"retain"
    );
}
