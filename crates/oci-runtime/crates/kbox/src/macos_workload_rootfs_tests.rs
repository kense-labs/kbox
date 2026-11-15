use super::*;
use std::os::unix::fs::{FileExt, PermissionsExt};

use koci_agent_protocol::{
    AgentVmDirectorySourceIdentity, AgentVmRootfsImage, AgentVmStorageSourceIdentity, GuestPath,
};
use koci_sdk::{
    ContainerId, ContainerTarget, CreateAttachments, Generation, ProcessIo, RootfsImage,
    StorageAccessMode, StorageAttachmentId, StorageCleanup, StorageOwnership,
};
use serde_json::json;
use sha2::{Digest, Sha256};

struct Fixture {
    _root: tempfile::TempDir,
    share: MacosRuntimeShare,
    disk: PathBuf,
    digest: String,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().canonicalize().unwrap();
        let share = parent.join("1");
        let bundle = share.join("bundle");
        fs::create_dir_all(bundle.join("rootfs")).unwrap();
        fs::set_permissions(&share, fs::Permissions::from_mode(0o700)).unwrap();
        let caller = parent.join("caller");
        fs::create_dir(&caller).unwrap();
        let disk = koci_agent_protocol::workload_rootfs_disk_path(&share);
        let contents = vec![0_u8; 16 * 1024 * 1024];
        fs::write(&disk, &contents).unwrap();
        fs::set_permissions(&disk, fs::Permissions::from_mode(0o600)).unwrap();
        let image = RootfsImage::new(
            format!("sha256:{:x}", Sha256::digest(&contents)),
            contents.len() as u64,
            format!("sha256:{}", "b".repeat(64)),
        )
        .unwrap();
        let mut config =
            serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
        config["root"] = json!({"path":"rootfs","readonly":false});
        config["mounts"] = json!([{"destination":"/workspace","type":"bind","source":caller,"options":["rbind","rw"]}]);
        config["annotations"] =
            json!({koci_sdk::ROOTFS_IMAGE_EXTENSION:image.annotation_value().unwrap()});
        let bundle = OciBundle::from_json(&bundle, config.to_string()).unwrap();
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
            .unwrap()
            .attach_storage_mount(
                &bundle,
                0,
                StorageAttachmentId::new("workspace").unwrap(),
                StorageAccessMode::ReadWrite,
                StorageOwnership::Caller,
                StorageCleanup::DetachOnly,
            )
            .unwrap();
        let metadata = fs::metadata(&caller).unwrap();
        let directory = AgentVmDirectoryAttachment::new(
            attachments.storage()[0].clone(),
            caller.to_str().unwrap(),
            AgentVmDirectorySourceIdentity::new(metadata.dev(), metadata.ino()).unwrap(),
            &attachments.digest().unwrap(),
        )
        .unwrap();
        let metadata = fs::metadata(&disk).unwrap();
        let manifest = AgentVmAttachmentManifest::new_with_directories(
            ContainerTarget::exact(ContainerId::new("rootfs").unwrap(), Generation(1)),
            GuestPath::new("/run/koci-runtime/bundle").unwrap(),
            bundle.config_digest(),
            attachments.digest().unwrap(),
            vec![],
            vec![],
            vec![directory],
        )
        .unwrap()
        .with_rootfs_image(
            AgentVmRootfsImage::new(
                image,
                AgentVmStorageSourceIdentity::new(
                    metadata.dev(),
                    metadata.rdev(),
                    metadata.ino(),
                    metadata.len(),
                )
                .unwrap(),
            )
            .unwrap(),
            &bundle,
        )
        .unwrap();
        for (path, bytes) in [
            (
                bundle.directory().join("config.json"),
                bundle.config_json().as_bytes().to_vec(),
            ),
            (
                share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME),
                manifest.to_bytes().unwrap(),
            ),
        ] {
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        Self {
            _root: root,
            share: MacosRuntimeShare::open(&share).unwrap(),
            disk,
            digest: manifest.digest().unwrap(),
        }
    }
    fn open(&self) -> Result<Option<MacosVmAttachments>> {
        MacosVmAttachments::open(&self.share, Some(&self.digest))
    }
}

#[test]
fn workload_disk_pin_refuses_content_change_and_inode_replacement_at_vm_entry() {
    let fixture = Fixture::new();
    let pins = fixture.open().unwrap().unwrap();
    pins.reverify().unwrap();
    let disk = fs::OpenOptions::new()
        .write(true)
        .open(&fixture.disk)
        .unwrap();
    disk.write_all_at(b"changed", 4096).unwrap();
    assert!(pins.reverify().is_err());
    assert!(fixture.open().is_err());
    let fixture = Fixture::new();
    let pins = fixture.open().unwrap().unwrap();
    let old = fixture.disk.with_extension("old");
    fs::rename(&fixture.disk, &old).unwrap();
    fs::copy(&old, &fixture.disk).unwrap();
    assert!(pins.reverify().is_err());
    assert!(fixture.open().is_err());
}

#[test]
fn workload_disk_pin_refuses_aliases_and_insecure_permissions() {
    for alias in [true, false] {
        let fixture = Fixture::new();
        if alias {
            fs::hard_link(&fixture.disk, fixture.disk.with_extension("alias")).unwrap();
        } else {
            fs::set_permissions(&fixture.disk, fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert!(fixture.open().is_err());
    }
}
