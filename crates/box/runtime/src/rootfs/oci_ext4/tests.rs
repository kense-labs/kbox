use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use tar::{Builder, EntryType, Header};

use super::publish_oci_layers_ext4;
use crate::oci::OciImage;
use crate::rootfs::Ext4ArtifactOptions;
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn write_blob(root: &Path, bytes: &[u8]) -> String {
    let digest = sha256(bytes);
    std::fs::create_dir_all(root.join("blobs/sha256")).unwrap();
    std::fs::write(
        root.join("blobs/sha256")
            .join(digest.strip_prefix("sha256:").unwrap()),
        bytes,
    )
    .unwrap();
    digest
}

fn image_from_layers(root: &Path, layers: &[&Path]) -> OciImage {
    let mut descriptors = Vec::with_capacity(layers.len());
    let mut diff_ids = Vec::with_capacity(layers.len());
    for layer in layers {
        let bytes = std::fs::read(layer).unwrap();
        descriptors.push(serde_json::json!({
            "mediaType": "application/vnd.oci.image.layer.v1.tar",
            "digest": write_blob(root, &bytes),
            "size": bytes.len()
        }));
        diff_ids.push(sha256(&bytes));
    }
    let config = serde_json::to_vec(&serde_json::json!({
        "architecture": "arm64",
        "os": "linux",
        "config": {"Cmd": ["/bin/true"]},
        "rootfs": {"type": "layers", "diff_ids": diff_ids}
    }))
    .unwrap();
    let config_digest = write_blob(root, &config);
    let manifest = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": config_digest,
            "size": config.len()
        },
        "layers": descriptors
    }))
    .unwrap();
    let manifest_digest = write_blob(root, &manifest);
    std::fs::write(
        root.join("oci-layout"),
        br#"{"imageLayoutVersion":"1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("index.json"),
        serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 2,
            "manifests": [{
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "digest": manifest_digest,
                "size": manifest.len(),
                "platform": {"os": "linux", "architecture": "arm64"}
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    OciImage::from_path(root).unwrap()
}

fn append_file(archive: &mut Builder<File>, path: &Path, content: &[u8], mode: u32) {
    let mut header = Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(mode);
    header.set_uid(123);
    header.set_gid(456);
    header.set_mtime(1_704_067_200);
    header.set_cksum();
    archive.append_data(&mut header, path, content).unwrap();
}

fn append_marker(archive: &mut Builder<File>, path: &str) {
    append_file(archive, Path::new(path), b"", 0o000);
}

fn append_dir(archive: &mut Builder<File>, path: &str, mode: u32) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Directory);
    header.set_size(0);
    header.set_mode(mode);
    header.set_uid(123);
    header.set_gid(456);
    header.set_mtime(1_704_067_200);
    header.set_cksum();
    archive
        .append_data(&mut header, path, std::io::empty())
        .unwrap();
}

fn append_symlink(archive: &mut Builder<File>, path: &str, target: &Path) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_uid(7);
    header.set_gid(8);
    header.set_mtime(1_704_067_201);
    header.set_cksum();
    archive.append_link(&mut header, path, target).unwrap();
}

#[test]
fn assembles_layers_without_a_host_namespace() {
    assemble_layers(false);
}

#[test]
fn volume_ext4_seeds_only_its_resolved_subtree_with_foreign_metadata_and_links() {
    let temporary = tempfile::tempdir().unwrap();
    let lower = temporary.path().join("lower.tar");
    let upper = temporary.path().join("upper.tar");
    let mut archive = Builder::new(File::create(&lower).unwrap());
    append_dir(&mut archive, "data", 0o750);
    archive
        .append_pax_extensions([("SCHILY.xattr.user.kbox", b"volume".as_slice())])
        .unwrap();
    append_file(&mut archive, Path::new("data/input"), b"seed", 0o640);
    append_file(&mut archive, Path::new("data/gone"), b"old", 0o600);
    append_file(&mut archive, Path::new("private-canary"), b"outside", 0o600);
    append_symlink(&mut archive, "volume-alias", Path::new("/data"));
    append_symlink(&mut archive, "bad-alias", Path::new("../../outside"));
    append_symlink(&mut archive, "data/link", Path::new("input"));
    let mut hardlink = Header::new_gnu();
    hardlink.set_entry_type(EntryType::Link);
    hardlink.set_size(0);
    hardlink.set_mode(0o640);
    hardlink.set_cksum();
    archive
        .append_link(&mut hardlink, "data/linked", Path::new("data/input"))
        .unwrap();
    archive.finish().unwrap();
    drop(archive);
    let mut archive = Builder::new(File::create(&upper).unwrap());
    append_marker(&mut archive, "data/.wh.gone");
    append_file(&mut archive, Path::new("data/new"), b"upper", 0o600);
    archive.finish().unwrap();
    drop(archive);
    let image = image_from_layers(&temporary.path().join("image"), &[&lower, &upper]);
    let mut root = super::OciWorkloadRootfs::load(&image, temporary.path()).unwrap();
    let options = Ext4ArtifactOptions::from_disk_mib(16, [0x71; 16]).unwrap();
    let artifact = root
        .publish_volume("/volume-alias", &temporary.path().join("volume"), options)
        .unwrap();
    let filesystem = mkext4::reader::Fs::open(File::open(artifact.disk).unwrap()).unwrap();
    assert!(filesystem.verify().unwrap().is_empty());
    let input = filesystem.resolve("/input").unwrap();
    assert_eq!(filesystem.read_file(input).unwrap(), b"seed");
    assert_eq!(filesystem.resolve("/linked").unwrap(), input);
    let inode = filesystem.inode(input).unwrap();
    assert_eq!(
        (inode.uid, inode.gid, inode.mode & 0o7777),
        (123, 456, 0o640)
    );
    let root_inode = filesystem.inode(filesystem.resolve("/").unwrap()).unwrap();
    assert_eq!(
        (root_inode.uid, root_inode.gid, root_inode.mode & 0o7777),
        (123, 456, 0o750)
    );
    assert!(filesystem
        .xattrs(input)
        .unwrap()
        .iter()
        .any(
            |attribute| attribute.full_name().as_deref() == Some(b"user.kbox")
                && attribute.value == b"volume"
        ));
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/new").unwrap())
            .unwrap(),
        b"upper"
    );
    assert!(filesystem.resolve("/gone").is_err());
    assert!(filesystem.resolve("/private-canary").is_err());
    assert!(filesystem.resolve("/data").is_err());
    for invalid in ["relative", "/", "/bad-alias", "/data/input"] {
        assert!(root
            .publish_volume(invalid, &temporary.path().join("invalid"), options)
            .is_err());
        assert!(!temporary.path().join("invalid").exists());
    }
    let empty = root
        .publish_volume("/missing", &temporary.path().join("empty"), options)
        .unwrap();
    let filesystem = mkext4::reader::Fs::open(File::open(empty.disk).unwrap()).unwrap();
    assert!(filesystem.resolve("/input").is_err());
    assert!(filesystem.verify().unwrap().is_empty());
}

#[test]
fn workload_ext4_preserves_oci_ownership_links_and_layers_without_boot_injection() {
    assemble_layers(true);
}

fn assemble_layers(workload: bool) {
    let temporary = tempfile::tempdir().unwrap();
    let lower = temporary.path().join("lower.tar");
    let upper = temporary.path().join("upper.tar");
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"guest-init").unwrap();

    let mut lower_tar = Builder::new(File::create(&lower).unwrap());
    lower_tar
        .append_pax_extensions([("SCHILY.xattr.user.kbox", b"preserved".as_slice())])
        .unwrap();
    append_file(&mut lower_tar, Path::new("README"), b"upper-case", 0o640);
    append_file(&mut lower_tar, Path::new("Readme"), b"mixed-case", 0o600);
    append_file(&mut lower_tar, Path::new("etc/removed"), b"lower", 0o644);
    append_file(&mut lower_tar, Path::new("opaque/old"), b"old", 0o644);
    let mut sparse = vec![0; 4 * 1024 * 1024];
    sparse.extend_from_slice(b"end");
    append_file(&mut lower_tar, Path::new("sparse"), &sparse, 0o644);
    let raw_name = std::ffi::OsStr::from_bytes(b"raw-\xff");
    append_file(&mut lower_tar, Path::new(raw_name), b"raw", 0o644);
    append_dir(&mut lower_tar, "usr", 0o755);
    append_dir(&mut lower_tar, "usr/sbin", 0o755);
    append_file(
        &mut lower_tar,
        Path::new("usr/sbin/init"),
        b"image-init",
        0o640,
    );
    append_symlink(&mut lower_tar, "sbin", Path::new("/usr/sbin"));
    lower_tar.finish().unwrap();
    drop(lower_tar);

    let mut upper_tar = Builder::new(File::create(&upper).unwrap());
    append_marker(&mut upper_tar, "etc/.wh.removed");
    append_file(&mut upper_tar, Path::new("opaque/new"), b"new", 0o644);
    append_marker(&mut upper_tar, "opaque/.wh..wh..opq");
    append_file(&mut upper_tar, Path::new("same-layer"), b"survives", 0o644);
    append_marker(&mut upper_tar, ".wh.same-layer");
    append_symlink(&mut upper_tar, "link", Path::new("opaque/new"));
    let mut hardlink = Header::new_gnu();
    hardlink.set_entry_type(EntryType::Link);
    hardlink.set_size(0);
    hardlink.set_mode(0o777);
    hardlink.set_uid(7);
    hardlink.set_gid(8);
    hardlink.set_mtime(1_704_067_201);
    hardlink.set_cksum();
    upper_tar
        .append_link(&mut hardlink, "link.alias", Path::new("link"))
        .unwrap();
    upper_tar.finish().unwrap();
    drop(upper_tar);
    let image = image_from_layers(&temporary.path().join("image"), &[&lower, &upper]);

    let options = Ext4ArtifactOptions::from_disk_mib(32, [0x31; 16]).unwrap();
    let destination = temporary.path().join("artifact");
    let artifact = if workload {
        let mut root = super::OciWorkloadRootfs::load(&image, temporary.path()).unwrap();
        use crate::process_path::ProcessRootfs;
        assert!(!root.executable_exists("/sbin/init").unwrap());
        root.ensure_directory("/workspace").unwrap();
        let box_dir = temporary.path().join("box");
        std::fs::create_dir(&box_dir).unwrap();
        crate::rootfs::install_baseline_bytes_if_absent(
            &box_dir,
            &root.diff_baseline_bytes().unwrap(),
        )
        .unwrap();
        assert!(!crate::rootfs::guest_diff_baseline_required(&box_dir).unwrap());
        let baseline_path = box_dir.join(crate::rootfs::DIFF_BASELINE_FILE);
        let before = std::fs::read(&baseline_path).unwrap();
        let baseline: std::collections::HashMap<String, crate::rootfs::RootfsFileInfo> =
            serde_json::from_slice(&before).unwrap();
        assert_eq!(
            (baseline["/README"].mode, baseline["/README"].size),
            (0o100640, 10)
        );
        assert_eq!(baseline["/usr"].mode, 0o040755);
        assert!(baseline["/usr"].is_dir);
        assert_eq!(baseline["/link"], baseline["/link.alias"]);
        assert_eq!(baseline["/link"].mode, 0o120777);
        assert!(!baseline.contains_key("/etc/removed"));
        assert!(!baseline.contains_key("/opaque/old"));
        assert!(baseline.contains_key("/opaque/new"));
        crate::rootfs::install_baseline_bytes_if_absent(
            &box_dir,
            &root.diff_baseline_bytes().unwrap(),
        )
        .unwrap();
        assert_eq!(std::fs::read(&baseline_path).unwrap(), before);
        root.publish(&destination, options).unwrap()
    } else {
        publish_oci_layers_ext4(
            &image,
            &guest_init,
            &sha256(b"guest-init"),
            &destination,
            options,
        )
        .unwrap()
    };

    let filesystem = mkext4::reader::Fs::open(File::open(artifact.disk).unwrap()).unwrap();
    assert!(filesystem.verify().unwrap().is_empty());
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/README").unwrap())
            .unwrap(),
        b"upper-case"
    );
    let readme = filesystem.resolve("/README").unwrap();
    let readme_inode = filesystem.inode(readme).unwrap();
    assert_eq!(
        (
            readme_inode.mode & 0o7777,
            readme_inode.uid,
            readme_inode.gid
        ),
        (0o640, 123, 456)
    );
    assert!(filesystem.xattrs(readme).unwrap().iter().any(|xattr| {
        xattr.full_name().as_deref() == Some(b"user.kbox") && xattr.value == b"preserved"
    }));
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/Readme").unwrap())
            .unwrap(),
        b"mixed-case"
    );
    assert!(filesystem.resolve("/etc/removed").is_err());
    assert!(filesystem.resolve("/opaque/old").is_err());
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/opaque/new").unwrap())
            .unwrap(),
        b"new"
    );
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/same-layer").unwrap())
            .unwrap(),
        b"survives"
    );
    let raw = filesystem
        .lookup(mkext4::spec::ROOT_INO, raw_name.as_bytes())
        .unwrap()
        .unwrap();
    assert_eq!(filesystem.read_file(raw).unwrap(), b"raw");
    let sparse = filesystem.resolve("/sparse").unwrap();
    let sparse_inode = filesystem.inode(sparse).unwrap();
    assert_eq!(sparse_inode.size, 4 * 1024 * 1024 + 3);
    assert!(
        sparse_inode.blocks * 512 < sparse_inode.size / 2,
        "direct assembly should preserve large zero runs as ext4 holes: blocks={}, size={}",
        sparse_inode.blocks,
        sparse_inode.size
    );
    assert_eq!(
        filesystem.resolve("/link").unwrap(),
        filesystem.resolve("/link.alias").unwrap()
    );
    assert_eq!(
        filesystem
            .symlink_target(filesystem.resolve("/link").unwrap())
            .unwrap(),
        b"opaque/new"
    );
    assert_eq!(
        filesystem
            .read_file(filesystem.resolve("/usr/sbin/init").unwrap())
            .unwrap(),
        if workload {
            b"image-init".as_slice()
        } else {
            b"guest-init".as_slice()
        }
    );
    let root = filesystem.inode(mkext4::spec::ROOT_INO).unwrap();
    assert_eq!((root.uid, root.gid), (0, 0));
    let directory = filesystem
        .inode(filesystem.resolve("/usr").unwrap())
        .unwrap();
    assert_eq!(
        (directory.uid, directory.gid, directory.mode & 0o7777),
        (123, 456, 0o755)
    );
    let init = filesystem.resolve("/usr/sbin/init").unwrap();
    let init_inode = filesystem.inode(init).unwrap();
    assert_eq!(
        (
            init_inode.mode & 0o7777,
            init_inode.uid,
            init_inode.gid,
            init_inode.mtime
        ),
        if workload {
            (0o640, 123, 456, 1_704_067_200)
        } else {
            (0o755, 0, 0, 0)
        }
    );
    assert!(filesystem.resolve("/.kimage_metadata_v1.json").is_err());
}

#[test]
fn failed_capacity_check_never_publishes_a_partial_generation() {
    failed_capacity(false);
}

#[test]
fn workload_capacity_failure_cleans_partial_disk_and_spool() {
    failed_capacity(true);
}

fn failed_capacity(workload: bool) {
    let temporary = tempfile::tempdir().unwrap();
    let layer = temporary.path().join("oversized.tar");
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"guest-init").unwrap();
    let payload = vec![0x5a; 20 * 1024 * 1024];
    let mut archive = Builder::new(File::create(&layer).unwrap());
    append_file(&mut archive, Path::new("payload"), &payload, 0o644);
    archive.finish().unwrap();
    drop(archive);
    let destination = temporary.path().join("artifact");
    let image = image_from_layers(&temporary.path().join("image"), &[&layer]);

    let options = Ext4ArtifactOptions::from_disk_mib(16, [0x32; 16]).unwrap();
    let other = temporary.path().join("other-owned-state");
    std::fs::write(&other, b"retain").unwrap();
    let error = if workload {
        super::OciWorkloadRootfs::load(&image, temporary.path())
            .unwrap()
            .publish(&destination, options)
    } else {
        publish_oci_layers_ext4(
            &image,
            &guest_init,
            &sha256(b"guest-init"),
            &destination,
            options,
        )
    }
    .unwrap_err()
    .to_string();
    assert_eq!(std::fs::read(&other).unwrap(), b"retain");

    assert!(error.contains("ext4"), "{error}");
    assert!(!destination.exists());
    assert!(std::fs::read_dir(temporary.path()).unwrap().all(|entry| {
        let name = entry.unwrap().file_name();
        let name = name.to_string_lossy();
        !name.starts_with(".krootfs-ext4-") && !name.starts_with(".koci-ext4-content-")
    }));
}

#[test]
fn rejects_internal_paths_and_symlink_parent_escapes() {
    for (name, build_layer) in [
        (
            "reserved",
            Box::new(|archive: &mut Builder<File>| {
                append_file(
                    archive,
                    Path::new(".krootfs_metadata_v1.json"),
                    b"forged",
                    0o644,
                );
            }) as Box<dyn Fn(&mut Builder<File>)>,
        ),
        (
            "escape",
            Box::new(|archive: &mut Builder<File>| {
                append_symlink(archive, "escape", Path::new("../../outside"));
                append_file(archive, Path::new("escape/payload"), b"forged", 0o644);
            }),
        ),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let layer = temporary.path().join(format!("{name}.tar"));
        let guest_init = temporary.path().join("guest-init");
        std::fs::write(&guest_init, b"guest-init").unwrap();
        let mut archive = Builder::new(File::create(&layer).unwrap());
        build_layer(&mut archive);
        archive.finish().unwrap();
        drop(archive);
        let destination = temporary.path().join("artifact");
        let image = image_from_layers(&temporary.path().join("image"), &[&layer]);

        let error = publish_oci_layers_ext4(
            &image,
            &guest_init,
            &sha256(b"guest-init"),
            &destination,
            Ext4ArtifactOptions::from_disk_mib(16, [0x33; 16]).unwrap(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("reserved internal path") || error.contains("escapes the guest root"),
            "{error}"
        );
        assert!(!destination.exists());
    }
}

#[test]
fn rejects_a_layer_changed_after_image_authentication() {
    let temporary = tempfile::tempdir().unwrap();
    let layer = temporary.path().join("layer.tar");
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"guest-init").unwrap();
    let mut archive = Builder::new(File::create(&layer).unwrap());
    append_file(&mut archive, Path::new("payload"), b"authenticated", 0o644);
    archive.finish().unwrap();
    drop(archive);
    let image = image_from_layers(&temporary.path().join("image"), &[&layer]);
    let blob = image.layer_paths()[0].clone();
    let mut changed = std::fs::read(&blob).unwrap();
    changed[0] ^= 0xff;
    std::fs::write(&blob, changed).unwrap();
    let destination = temporary.path().join("artifact");

    let error = publish_oci_layers_ext4(
        &image,
        &guest_init,
        &sha256(b"guest-init"),
        &destination,
        Ext4ArtifactOptions::from_disk_mib(16, [0x34; 16]).unwrap(),
    )
    .unwrap_err()
    .to_string();

    let workload_error = super::OciWorkloadRootfs::load(&image, temporary.path())
        .unwrap_err()
        .to_string();
    assert!(
        workload_error.contains("descriptor digest"),
        "{workload_error}"
    );
    assert!(error.contains("descriptor digest"), "{error}");
    assert!(!destination.exists());
}

#[test]
fn assembles_an_authenticated_gzip_layer() {
    let temporary = tempfile::tempdir().unwrap();
    let plain = temporary.path().join("layer.tar");
    let compressed = temporary.path().join("layer.tar.gz");
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"guest-init").unwrap();
    let mut archive = Builder::new(File::create(&plain).unwrap());
    append_file(&mut archive, Path::new("compressed"), b"verified", 0o644);
    archive.finish().unwrap();
    drop(archive);
    let mut encoder = flate2::write::GzEncoder::new(
        File::create(&compressed).unwrap(),
        flate2::Compression::default(),
    );
    std::io::copy(&mut File::open(&plain).unwrap(), &mut encoder).unwrap();
    encoder.finish().unwrap();
    let image = image_from_layers(&temporary.path().join("image"), &[&compressed]);

    let artifact = publish_oci_layers_ext4(
        &image,
        &guest_init,
        &sha256(b"guest-init"),
        &temporary.path().join("artifact"),
        Ext4ArtifactOptions::from_disk_mib(16, [0x35; 16]).unwrap(),
    )
    .unwrap();

    let filesystem = mkext4::reader::Fs::open(File::open(artifact.disk).unwrap()).unwrap();
    let file = filesystem.resolve("/compressed").unwrap();
    assert_eq!(filesystem.read_file(file).unwrap(), b"verified");
}

#[test]
fn rejects_guest_init_that_disagrees_with_its_cache_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let layer = temporary.path().join("layer.tar");
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"changed-init").unwrap();
    let mut archive = Builder::new(File::create(&layer).unwrap());
    append_file(&mut archive, Path::new("payload"), b"image", 0o644);
    archive.finish().unwrap();
    drop(archive);
    let image = image_from_layers(&temporary.path().join("image"), &[&layer]);
    let destination = temporary.path().join("artifact");

    let error = publish_oci_layers_ext4(
        &image,
        &guest_init,
        &sha256(b"original-init"),
        &destination,
        Ext4ArtifactOptions::from_disk_mib(16, [0x36; 16]).unwrap(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("cache identity"), "{error}");
    assert!(!destination.exists());
}

#[test]
fn rejects_an_empty_guest_init_before_artifact_publication() {
    let temporary = tempfile::tempdir().unwrap();
    let guest_init = temporary.path().join("guest-init");
    std::fs::write(&guest_init, b"").unwrap();
    let image = image_from_layers(&temporary.path().join("image"), &[]);
    let destination = temporary.path().join("artifact");

    let error = publish_oci_layers_ext4(
        &image,
        &guest_init,
        &sha256(b""),
        &destination,
        Ext4ArtifactOptions::from_disk_mib(16, [0x37; 16]).unwrap(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("boot contract"), "{error}");
    assert!(!destination.exists());
}
