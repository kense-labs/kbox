use super::*;
use sha2::{Digest, Sha256};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[test]
fn portable_disk_publication_preserves_sparse_capacity_bytes_and_source() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("workload-file"), b"rootfs-data").unwrap();
    let disk = temporary.path().join("sparse.ext4");
    let length = 64 * 1024 * 1024;
    let mut file = File::create(&disk).unwrap();
    file.set_len(length).unwrap();
    file.write_all(b"source-head").unwrap();
    file.seek(SeekFrom::Start(length - 1024 * 1024)).unwrap();
    file.write_all(b"source-tail-before-hole").unwrap();
    file.sync_all().unwrap();
    drop(file);
    let before = std::fs::metadata(&disk).unwrap();
    let digest = |path: &Path| {
        let mut file = File::open(path).unwrap();
        let mut bytes = [0u8; 64 * 1024];
        let mut hash = Sha256::new();
        loop {
            let read = file.read(&mut bytes).unwrap();
            if read == 0 {
                break;
            }
            hash.update(&bytes[..read]);
        }
        hash.finalize()
    };
    let expected = digest(&disk);
    let bundle = temporary.path().join("handoffs/container/operation/bundle");
    publish_portable_bundle_with_disk(&source, &Spec::default(), &bundle, Some(&disk)).unwrap();
    let output = bundle.join(koci_sdk::ROOTFS_IMAGE_FILE_NAME);
    let metadata = std::fs::metadata(&output).unwrap();
    assert_eq!(metadata.len(), length);
    assert!(metadata.blocks() * 512 < length / 4);
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(digest(&output), expected);
    assert_eq!(
        std::fs::read(bundle.join("rootfs/workload-file")).unwrap(),
        b"rootfs-data"
    );
    let after = std::fs::metadata(&disk).unwrap();
    assert_eq!(
        (
            after.dev(),
            after.ino(),
            after.len(),
            after.mode(),
            after.blocks()
        ),
        (
            before.dev(),
            before.ino(),
            before.len(),
            before.mode(),
            before.blocks()
        )
    );
    assert_eq!(digest(&disk), expected);
    let first_inode = metadata.ino();
    assert!(
        publish_portable_bundle_with_disk(&source, &Spec::default(), &bundle, Some(&disk)).is_err()
    );
    assert_eq!(std::fs::metadata(&output).unwrap().ino(), first_inode);
    assert_eq!(digest(&output), expected);
}
