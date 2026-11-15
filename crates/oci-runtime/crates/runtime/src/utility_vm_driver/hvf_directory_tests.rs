#![cfg(test)]

use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};

use koci_agent_protocol::{AgentVmAttachmentManifest, GuestPath};
use koci_sdk::{
    oci_spec::runtime::Spec, ContainerId, ContainerTarget, CreateAttachments, Generation,
    OciBundle, ProcessIo, StorageAccessMode, StorageAttachmentId, StorageCleanup, StorageOwnership,
};
use serde_json::json;

use super::*;

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    share: PathBuf,
    bundle: OciBundle,
    attachments: CreateAttachments,
    target: ContainerTarget,
    guest_bundle: GuestPath,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let share = root.join("share");
        std::fs::create_dir_all(share.join("bundle/rootfs")).unwrap();
        let mut config = serde_json::to_value(Spec::default()).unwrap();
        let mut mounts = Vec::new();
        for (index, access) in ["ro", "rw"].into_iter().enumerate() {
            let source = root.join(format!("工作区-{index}"));
            std::fs::create_dir(&source).unwrap();
            std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755)).unwrap();
            mounts.push(
                json!({"destination":format!("/workspace-{index}"), "type":"none",
                "source":source, "options":["rbind",access]}),
            );
        }
        config["root"] = json!({"path":"rootfs", "readonly":true});
        config["mounts"] = json!(mounts);
        let bundle = OciBundle::from_json(share.join("bundle"), config.to_string()).unwrap();
        let attachments = Self::grants(&bundle);
        Self {
            _temporary: temporary,
            root,
            share,
            bundle,
            attachments,
            target: ContainerTarget::exact(ContainerId::new("directories").unwrap(), Generation(1)),
            guest_bundle: GuestPath::new("/run/koci-runtime/bundle").unwrap(),
        }
    }

    fn grants(bundle: &OciBundle) -> CreateAttachments {
        let mut attachments = CreateAttachments::from_bundle(bundle, ProcessIo::default()).unwrap();
        for (index, access) in [StorageAccessMode::ReadOnly, StorageAccessMode::ReadWrite]
            .into_iter()
            .enumerate()
        {
            attachments = attachments
                .attach_storage_mount(
                    bundle,
                    index,
                    StorageAttachmentId::new(format!("caller-{index}")).unwrap(),
                    access,
                    StorageOwnership::Caller,
                    StorageCleanup::DetachOnly,
                )
                .unwrap();
        }
        attachments
    }

    fn request(&self) -> UtilityVmLaunchRequest<'_> {
        UtilityVmLaunchRequest {
            target: &self.target,
            runtime_share: &self.share,
            bundle: &self.bundle,
            guest_bundle: &self.guest_bundle,
            attachment_contract: &self.attachments,
        }
    }
}

#[tokio::test]
async fn transport_retains_exact_identity_access_and_caller_permissions() {
    let fixture = Fixture::new();
    let digest = prepare(&fixture.request()).await.unwrap().unwrap();
    let bytes = std::fs::read(fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)).unwrap();
    let manifest = AgentVmAttachmentManifest::from_bytes(&bytes).unwrap();
    assert_eq!(manifest.digest().unwrap(), digest);
    assert_eq!(manifest.directories().len(), 2);
    for directory in manifest.directories() {
        let metadata = std::fs::metadata(directory.host_source()).unwrap();
        assert_eq!(directory.source_identity().device(), metadata.dev());
        assert_eq!(directory.source_identity().inode(), metadata.ino());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
        assert!(fixture
            .attachments
            .storage()
            .contains(directory.attachment()));
    }
}

#[tokio::test]
async fn raw_ext4_storage_keeps_existing_disk_identity_access_and_detach_contract() {
    let mut fixture = Fixture::new();
    let mut config: serde_json::Value = serde_json::from_str(fixture.bundle.config_json()).unwrap();
    for (index, mount) in config["mounts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let source = fixture.root.join(format!("卷-{index}.ext4"));
        std::fs::File::create(&source)
            .unwrap()
            .set_len(16 * 1024 * 1024)
            .unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600)).unwrap();
        mount["source"] = json!(source);
        mount["type"] = json!("ext4");
        mount["options"] = json!([if index == 0 { "ro" } else { "rw" }, "nosuid", "nodev"]);
    }
    fixture.bundle = OciBundle::from_json(fixture.bundle.directory(), config.to_string()).unwrap();
    fixture.attachments = Fixture::grants(&fixture.bundle);
    assert_eq!(
        prepare(&fixture.request()).await.unwrap_err().code,
        ErrorCode::Unsupported
    );
    assert!(!fixture
        .share
        .join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)
        .exists());
    config["annotations"] =
        json!({koci_sdk::RAW_STORAGE_IMAGES_EXTENSION:koci_sdk::RAW_STORAGE_IMAGES_VALUE});
    fixture.bundle = OciBundle::from_json(fixture.bundle.directory(), config.to_string()).unwrap();
    fixture.attachments = Fixture::grants(&fixture.bundle);
    let digest = prepare(&fixture.request()).await.unwrap().unwrap();
    let manifest = AgentVmAttachmentManifest::from_bytes(
        &std::fs::read(fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.digest().unwrap(), digest);
    assert!(manifest.directories().is_empty());
    assert_eq!(manifest.storage().len(), 2);
    for grant in manifest.storage() {
        let metadata = std::fs::metadata(grant.host_source()).unwrap();
        assert_eq!(grant.source_identity().device(), metadata.dev());
        assert_eq!(grant.source_identity().inode(), metadata.ino());
        assert_eq!(grant.source_identity().size(), metadata.len());
        assert_eq!(grant.ownership(), StorageOwnership::Caller);
        assert_eq!(grant.cleanup(), StorageCleanup::DetachOnly);
    }
}

#[tokio::test]
async fn restricted_directory_requires_a_declared_extension_and_enforcement_digest() {
    let mut fixture = Fixture::new();
    let restrictions =
        koci_sdk::StoragePathRestrictions::new(vec![koci_sdk::StoragePathRestriction::new(
            StorageAttachmentId::new("caller-1").unwrap(),
            vec![".env".into()],
            vec![".kbox".into()],
            vec![".kbox/loops".into()],
        )
        .unwrap()])
        .unwrap();
    let mut config: serde_json::Value = serde_json::from_str(fixture.bundle.config_json()).unwrap();
    config["annotations"] = json!({koci_sdk::STORAGE_PATH_RESTRICTIONS_EXTENSION: restrictions.to_annotation_value().unwrap()});
    fixture.bundle = OciBundle::from_json(fixture.bundle.directory(), config.to_string()).unwrap();
    fixture.attachments = Fixture::grants(&fixture.bundle);
    assert_eq!(
        prepare(&fixture.request()).await.unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    assert!(!fixture
        .share
        .join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)
        .exists());
    fixture.attachments = fixture
        .attachments
        .with_storage_path_restrictions(&fixture.bundle)
        .unwrap();
    let digest = prepare(&fixture.request()).await.unwrap().unwrap();
    let manifest = AgentVmAttachmentManifest::from_bytes(
        &std::fs::read(fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.digest().unwrap(), digest);
    assert_eq!(
        manifest.storage_path_restrictions_digest(),
        Some(restrictions.digest().unwrap().as_str())
    );
    manifest.validate_bundle(&fixture.bundle).unwrap();
}

#[tokio::test]
async fn replay_cannot_silently_export_a_replaced_directory() {
    let fixture = Fixture::new();
    prepare(&fixture.request()).await.unwrap();
    let manifest_path = fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    let retained = std::fs::read(&manifest_path).unwrap();
    let source = fixture.root.join("工作区-0");
    std::fs::rename(&source, fixture.root.join("original")).unwrap();
    std::fs::create_dir(&source).unwrap();
    let error = prepare(&fixture.request()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(std::fs::read(manifest_path).unwrap(), retained);
    assert!(fixture.root.join("original").is_dir());
}

#[tokio::test]
async fn aliases_and_overlapping_grants_fail_before_manifest_publication() {
    for alias in [true, false] {
        let mut fixture = Fixture::new();
        let source = fixture.root.join("工作区-0");
        if alias {
            std::fs::rename(&source, fixture.root.join("original")).unwrap();
            symlink(fixture.root.join("original"), &source).unwrap();
        } else {
            let mut config: serde_json::Value =
                serde_json::from_str(fixture.bundle.config_json()).unwrap();
            let nested = source.join("child");
            std::fs::create_dir(&nested).unwrap();
            config["mounts"][1]["source"] = json!(nested);
            fixture.bundle =
                OciBundle::from_json(fixture.bundle.directory(), config.to_string()).unwrap();
            fixture.attachments = Fixture::grants(&fixture.bundle);
        }
        let error = prepare(&fixture.request()).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::FailedPrecondition);
        assert!(!fixture
            .share
            .join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)
            .exists());
    }
}

#[tokio::test]
async fn a_create_without_directory_grants_refuses_stale_manifest_evidence() {
    let mut fixture = Fixture::new();
    prepare(&fixture.request()).await.unwrap();
    fixture.attachments =
        CreateAttachments::from_bundle(&fixture.bundle, ProcessIo::default()).unwrap();
    assert_eq!(
        prepare(&fixture.request()).await.unwrap_err().code,
        ErrorCode::Conflict
    );
}

impl Fixture {
    fn with_workload(mut self) -> Self {
        use sha2::{Digest, Sha256};
        let input = self
            .share
            .join("bundle")
            .join(koci_sdk::ROOTFS_IMAGE_FILE_NAME);
        let contents = vec![0_u8; 16 * 1024 * 1024];
        std::fs::write(&input, &contents).unwrap();
        std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o600)).unwrap();
        let descriptor = koci_sdk::RootfsImage::new(
            format!("sha256:{:x}", Sha256::digest(&contents)),
            contents.len() as u64,
            format!("sha256:{}", "b".repeat(64)),
        )
        .unwrap();
        let mut config: serde_json::Value =
            serde_json::from_str(self.bundle.config_json()).unwrap();
        config["root"]["readonly"] = json!(false);
        config["annotations"] =
            json!({koci_sdk::ROOTFS_IMAGE_EXTENSION: descriptor.annotation_value().unwrap()});
        self.bundle = OciBundle::from_json(self.bundle.directory(), config.to_string()).unwrap();
        std::fs::write(
            self.bundle.directory().join("config.json"),
            self.bundle.config_json(),
        )
        .unwrap();
        self.attachments = Self::grants(&self.bundle)
            .with_rootfs_image(&self.bundle)
            .unwrap();
        self
    }
}

#[tokio::test]
async fn workload_disk_is_private_bound_and_separate_for_two_executions() {
    let first = Fixture::new().with_workload();
    let second = Fixture::new().with_workload();
    for fixture in [&first, &second] {
        let digest = prepare(&fixture.request()).await.unwrap().unwrap();
        let bytes =
            std::fs::read(fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)).unwrap();
        let manifest = AgentVmAttachmentManifest::from_bytes(&bytes).unwrap();
        assert_eq!(manifest.digest().unwrap(), digest);
        assert_eq!(manifest.target(), &fixture.target);
        let disk = koci_agent_protocol::workload_rootfs_disk_path(&fixture.share);
        assert!(!disk.starts_with(&fixture.share));
        assert!(!fixture
            .bundle
            .directory()
            .join(koci_sdk::ROOTFS_IMAGE_FILE_NAME)
            .exists());
        let metadata = std::fs::metadata(&disk).unwrap();
        assert_eq!(
            manifest.rootfs_image().unwrap().source_identity().inode(),
            metadata.ino()
        );
        assert_eq!(metadata.nlink(), 1);
        assert_eq!(prepare(&fixture.request()).await.unwrap().unwrap(), digest);
    }
    let a = koci_agent_protocol::workload_rootfs_disk_path(&first.share);
    let b = koci_agent_protocol::workload_rootfs_disk_path(&second.share);
    assert_ne!(
        std::fs::metadata(&a).unwrap().ino(),
        std::fs::metadata(&b).unwrap().ino()
    );
    use std::os::unix::fs::FileExt;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&a)
        .unwrap()
        .write_all_at(b"mutated", 4096)
        .unwrap();
    assert!(prepare(&first.request()).await.is_err());
    assert!(prepare(&second.request()).await.is_ok());
    super::super::rootfs_image::cleanup(&first.share, &first.target)
        .await
        .unwrap();
    assert!(!a.exists());
    assert!(b.exists());
    super::super::rootfs_image::cleanup(&second.share, &second.target)
        .await
        .unwrap();
    assert!(!b.exists());
}

#[tokio::test]
async fn workload_disk_rejects_corruption_source_alias_and_stale_generation() {
    use std::os::unix::fs::FileExt;
    let corrupt = Fixture::new().with_workload();
    let input = corrupt
        .bundle
        .directory()
        .join(koci_sdk::ROOTFS_IMAGE_FILE_NAME);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&input)
        .unwrap()
        .write_all_at(b"bad", 0)
        .unwrap();
    assert!(prepare(&corrupt.request()).await.is_err());
    assert!(input.exists());
    assert!(!koci_agent_protocol::workload_rootfs_disk_path(&corrupt.share).exists());
    let alias = Fixture::new().with_workload();
    let input = alias
        .bundle
        .directory()
        .join(koci_sdk::ROOTFS_IMAGE_FILE_NAME);
    std::fs::hard_link(&input, alias.root.join("alias.ext4")).unwrap();
    assert!(prepare(&alias.request()).await.is_err());
    let fixture = Fixture::new().with_workload();
    prepare(&fixture.request()).await.unwrap();
    let changed = ContainerTarget::exact(fixture.target.id.clone(), Generation(2));
    let mut request = fixture.request();
    request.target = &changed;
    assert!(prepare(&request).await.is_err());
    let disk = koci_agent_protocol::workload_rootfs_disk_path(&fixture.share);
    let original = disk.with_extension("original");
    std::fs::rename(&disk, &original).unwrap();
    std::fs::copy(&original, &disk).unwrap();
    assert!(prepare(&fixture.request()).await.is_err());
    assert!(
        super::super::rootfs_image::cleanup(&fixture.share, &fixture.target)
            .await
            .is_err()
    );
    assert!(disk.exists());
    assert!(original.exists());
}

#[tokio::test]
async fn workload_disk_recovers_pre_manifest_interruption_and_refuses_missing_capability() {
    let mut unsupported = Fixture::new().with_workload();
    let mut missing = serde_json::to_value(&unsupported.attachments).unwrap();
    missing["extensions"]
        .as_object_mut()
        .unwrap()
        .remove(koci_sdk::ROOTFS_IMAGE_EXTENSION);
    unsupported.attachments = serde_json::from_value(missing).unwrap();
    assert!(prepare(&unsupported.request()).await.is_err());
    assert!(unsupported
        .bundle
        .directory()
        .join(koci_sdk::ROOTFS_IMAGE_FILE_NAME)
        .exists());
    let fixture = Fixture::new().with_workload();
    let rootfs = super::super::rootfs_image::prepare(&fixture.request())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rootfs.source_identity().size(), 16 * 1024 * 1024);
    assert!(!fixture
        .share
        .join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME)
        .exists());
    assert!(prepare(&fixture.request()).await.unwrap().is_some());
    super::super::rootfs_image::cleanup(&fixture.share, &fixture.target)
        .await
        .unwrap();
}

#[tokio::test]
async fn workload_disk_cleanup_refuses_wrong_generation_and_unbounded_manifest() {
    let fixture = Fixture::new().with_workload();
    prepare(&fixture.request()).await.unwrap();
    let changed = ContainerTarget::exact(fixture.target.id.clone(), Generation(2));
    let disk = koci_agent_protocol::workload_rootfs_disk_path(&fixture.share);
    assert!(
        super::super::rootfs_image::cleanup(&fixture.share, &changed)
            .await
            .is_err()
    );
    assert!(disk.exists());
    let path = fixture.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    let original = std::fs::read(&path).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(koci_agent_protocol::AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES as u64 + 1)
        .unwrap();
    assert!(prepare(&fixture.request()).await.is_err());
    assert!(
        super::super::rootfs_image::cleanup(&fixture.share, &fixture.target)
            .await
            .is_err()
    );
    assert!(disk.exists());
    std::fs::write(&path, original).unwrap();
    super::super::rootfs_image::cleanup(&fixture.share, &fixture.target)
        .await
        .unwrap();
    assert!(!disk.exists());
}
