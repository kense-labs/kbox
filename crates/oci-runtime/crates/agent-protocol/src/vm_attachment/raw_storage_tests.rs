use super::*;
use koci_sdk::{
    ContainerId, CreateAttachments, FileGrant, FileGrants, FileSourceIdentity, Generation,
    ProcessIo, RootfsImage, StorageAccessMode, StorageAttachmentId, StorageCleanup,
    StorageOwnership,
};
use serde_json::json;

fn fixture() -> (OciBundle, AgentVmAttachmentManifest) {
    fixture_with_nproc(false)
}

fn fixture_with_nproc(nproc: bool) -> (OciBundle, AgentVmAttachmentManifest) {
    fixture_with_memory(nproc, false)
}

fn fixture_with_memory(nproc: bool, memory: bool) -> (OciBundle, AgentVmAttachmentManifest) {
    let grants = FileGrants::new(vec![FileGrant::new(
        0,
        StorageAttachmentId::new("file").unwrap(),
        FileSourceIdentity::new(4, 17).unwrap(),
        StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let image = RootfsImage::new(
        format!("sha256:{}", "a".repeat(64)),
        16 * 1024 * 1024,
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let mut config = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    config["mounts"] = json!([
        {"source":"/caller/file","destination":"/file","type":"bind","options":["bind","ro"]},
        {"source":"/caller/卷.ext4","destination":"/volume","type":"ext4","options":["rw","nosuid","nodev"]},
        {"source":"/caller/directory","destination":"/directory","type":"bind","options":["rbind","ro"]}
    ]);
    config["annotations"] = json!({
        koci_sdk::RAW_STORAGE_IMAGES_EXTENSION:koci_sdk::RAW_STORAGE_IMAGES_VALUE,
        koci_sdk::FILE_GRANTS_EXTENSION:grants.annotation_value().unwrap(),
        koci_sdk::ROOTFS_IMAGE_EXTENSION:image.annotation_value().unwrap()
    });
    if memory {
        config["annotations"][koci_sdk::MEMORY_SECRET_FILES_EXTENSION] =
            json!(json!({"schemaVersion":3,"mountIndices":[0]}).to_string());
    }
    if nproc {
        config["process"]["rlimits"] = json!([{"type":"RLIMIT_NPROC","soft":16,"hard":16}]);
    }
    let bundle = OciBundle::from_json(
        std::env::temp_dir().join("raw-manifest"),
        config.to_string(),
    )
    .unwrap();
    let contract = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
        .unwrap()
        .attach_storage_mount(
            &bundle,
            1,
            StorageAttachmentId::new("raw").unwrap(),
            StorageAccessMode::ReadWrite,
            StorageOwnership::Caller,
            StorageCleanup::DetachOnly,
        )
        .unwrap()
        .attach_storage_mount(
            &bundle,
            2,
            StorageAttachmentId::new("directory").unwrap(),
            StorageAccessMode::ReadOnly,
            StorageOwnership::Caller,
            StorageCleanup::DetachOnly,
        )
        .unwrap();
    let find = |name: &str| {
        contract
            .storage()
            .iter()
            .find(|grant| grant.identity().as_str() == name)
            .unwrap()
            .clone()
    };
    let digest = contract.digest().unwrap();
    let raw = find("raw");
    let storage = AgentVmStorageAttachment::new(
        raw.identity().clone(),
        raw.mount().clone(),
        raw.access_mode(),
        raw.ownership(),
        raw.cleanup(),
        "/caller/卷.ext4",
        AgentVmStorageSourceIdentity::new(4, 0, 700, 16 * 1024 * 1024).unwrap(),
        &digest,
    )
    .unwrap();
    let directory = AgentVmDirectoryAttachment::new(
        find("directory"),
        "/caller/directory",
        AgentVmDirectorySourceIdentity::new(4, 99).unwrap(),
        &digest,
    )
    .unwrap();
    let file = AgentVmFileAttachment::new(
        find("file"),
        "/caller/file",
        FileSourceIdentity::new(4, 17).unwrap(),
        AgentVmDirectorySourceIdentity::new(4, 18).unwrap(),
        bundle.config_digest(),
    )
    .unwrap();
    let rootfs = AgentVmRootfsImage::new(
        image,
        AgentVmStorageSourceIdentity::new(4, 0, 800, 16 * 1024 * 1024).unwrap(),
    )
    .unwrap();
    let manifest = AgentVmAttachmentManifest::new_with_raw_storage(
        ContainerTarget::exact(ContainerId::new("raw-volume").unwrap(), Generation(2)),
        GuestPath::new("/run/koci-runtime/bundle").unwrap(),
        &bundle,
        digest,
        vec![storage],
        vec![directory],
        vec![file],
        Some(rootfs),
    )
    .unwrap();
    (bundle, manifest)
}

#[test]
fn raw_memory_files_require_v11_and_preserve_nproc_envelope_and_old_transports() {
    for nproc in [false, true] {
        let (bundle, manifest) = fixture_with_memory(nproc, true);
        assert_eq!(manifest.transport_schema(), "koci.agent-vm-attachments.v11");
        assert_eq!(
            manifest.schema_version,
            if nproc {
                AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION
            } else {
                "koci.agent-vm-attachments.v11"
            }
        );
        assert_eq!(
            AgentVmAttachmentManifest::from_bytes(&manifest.to_bytes().unwrap()).unwrap(),
            manifest
        );
        manifest.validate_bundle(&bundle).unwrap();
        for old in [
            AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION,
            AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION,
            AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION,
        ] {
            let mut changed = manifest.clone();
            if nproc {
                changed.nproc_transport_schema = Some(old.into());
            } else {
                changed.schema_version = old.into();
            }
            assert!(changed.validate_bundle(&bundle).is_err(), "{old}");
        }
        for kind in ["files", "storage", "rootfs"] {
            let mut changed = manifest.clone();
            match kind {
                "files" => changed.files.clear(),
                "storage" => changed.storage.clear(),
                "rootfs" => changed.rootfs_image = None,
                _ => unreachable!(),
            }
            assert!(changed.validate_bundle(&bundle).is_err(), "{kind}");
        }
    }
}

#[test]
fn combined_storage_is_explicit_v7_and_does_not_reinterpret_legacy_manifest_versions() {
    let (bundle, manifest) = fixture();
    assert_eq!(
        manifest.schema_version,
        AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        AgentVmAttachmentManifest::from_bytes(&manifest.to_bytes().unwrap()).unwrap(),
        manifest
    );
    manifest.validate_bundle(&bundle).unwrap();
    for version in [
        AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION,
        AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION,
        AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION,
    ] {
        let mut changed = manifest.clone();
        changed.schema_version = version.into();
        assert!(changed.validate_bundle(&bundle).is_err(), "{version}");
    }
    let mut changed = manifest;
    changed.storage.clear();
    assert!(changed.validate_bundle(&bundle).is_err());
}

#[test]
fn combined_storage_refuses_cross_kind_object_allocation_and_serial_aliases() {
    let (bundle, manifest) = fixture();
    let mut value = serde_json::to_value(&manifest).unwrap();
    value["storage"][0]["sourceIdentity"]["inode"] = json!(17);
    let changed: AgentVmAttachmentManifest = serde_json::from_value(value).unwrap();
    assert!(changed.validate_bundle(&bundle).is_err());
    let mut value = serde_json::to_value(&manifest).unwrap();
    for (key, inode) in [("rootfsImage", 1234567_u64), ("storage", 1234568_u64)] {
        let identity = if key == "storage" {
            &mut value[key][0]["sourceIdentity"]
        } else {
            &mut value[key]["sourceIdentity"]
        };
        identity["device"] = json!(1234567890123_u64);
        identity["inode"] = json!(inode);
    }
    let changed: AgentVmAttachmentManifest = serde_json::from_value(value).unwrap();
    assert!(changed.validate_bundle(&bundle).is_err());
    let mut changed = manifest.clone();
    let raw = &manifest.storage[0];
    changed.storage[0] = AgentVmStorageAttachment::new(
        manifest.files[0].attachment().identity().clone(),
        raw.mount().clone(),
        raw.access_mode(),
        raw.ownership(),
        raw.cleanup(),
        raw.host_source(),
        raw.source_identity(),
        &manifest.attachment_digest,
    )
    .unwrap();
    assert!(changed.validate_bundle(&bundle).is_err());
}

#[test]
fn nproc_guard_keeps_raw_storage_file_and_rootfs_transport_semantics() {
    let (bundle, manifest) = fixture_with_nproc(true);
    assert_eq!(
        manifest.schema_version,
        AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.transport_schema(),
        AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.storage().len(), 1);
    assert_eq!(manifest.files().len(), 1);
    assert!(manifest.rootfs_image().is_some());
    manifest.validate_bundle(&bundle).unwrap();
    assert_eq!(
        AgentVmAttachmentManifest::from_bytes(&manifest.to_bytes().unwrap()).unwrap(),
        manifest
    );
}
