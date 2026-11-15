use super::*;
use crate::{AttachmentCapabilities, ErrorCode, ProcessIo};
use serde_json::json;

fn bundle(image: &RootfsImage) -> OciBundle {
    let mut config = serde_json::to_value(crate::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    config["annotations"] = json!({ROOTFS_IMAGE_EXTENSION: image.annotation_value().unwrap()});
    OciBundle::from_json(
        std::env::temp_dir().join("rootfs-contract"),
        config.to_string(),
    )
    .unwrap()
}

#[test]
fn workload_rootfs_requires_an_explicit_supported_capability() {
    let image = RootfsImage::new(
        format!("sha256:{}", "a".repeat(64)),
        16 * 1024 * 1024,
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let bundle = bundle(&image);
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
        .unwrap()
        .with_rootfs_image(&bundle)
        .unwrap();
    assert_eq!(attachments.rootfs_image(&bundle).unwrap(), Some(image));
    assert_eq!(
        AttachmentCapabilities::base_v2()
            .require(&attachments)
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    AttachmentCapabilities::base_v2()
        .with_extension(ROOTFS_IMAGE_EXTENSION, vec![1])
        .unwrap()
        .require(&attachments)
        .unwrap();
    let mut incompatible = serde_json::to_value(&attachments).unwrap();
    incompatible["extensions"][ROOTFS_IMAGE_EXTENSION]["version"] = json!(2);
    let incompatible: CreateAttachments = serde_json::from_value(incompatible).unwrap();
    assert!(incompatible.validate(&bundle).is_err());
    let mut missing = serde_json::to_value(&attachments).unwrap();
    missing["extensions"]
        .as_object_mut()
        .unwrap()
        .remove(ROOTFS_IMAGE_EXTENSION);
    let missing: CreateAttachments = serde_json::from_value(missing).unwrap();
    assert!(missing.validate(&bundle).is_err());
}

#[test]
fn workload_rootfs_rejects_invalid_identity_capacity_and_root_path() {
    for (digest, size) in [
        ("sha256:bad".into(), 16 * 1024 * 1024),
        (format!("sha256:{}", "A".repeat(64)), 16 * 1024 * 1024),
        (format!("sha256:{}", "a".repeat(64)), 4096),
    ] {
        assert!(RootfsImage::new(digest, size, format!("sha256:{}", "b".repeat(64))).is_err());
    }
    let image = RootfsImage::new(
        format!("sha256:{}", "a".repeat(64)),
        16 * 1024 * 1024,
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let source = bundle(&image);
    let mut config: serde_json::Value = serde_json::from_str(source.config_json()).unwrap();
    for root in [
        json!({"path":"../rootfs"}),
        json!({"path":"rootfs","readonly":true}),
    ] {
        config["root"] = root;
        let bundle = OciBundle::from_json(source.directory(), config.to_string()).unwrap();
        assert!(RootfsImage::from_bundle(&bundle).is_err());
    }
    config["root"] = json!({"path":"rootfs"});
    config["annotations"][ROOTFS_IMAGE_EXTENSION] = json!("x".repeat(1025));
    assert!(RootfsImage::from_bundle(
        &OciBundle::from_json(source.directory(), config.to_string()).unwrap()
    )
    .is_err());
}

#[test]
fn workload_readonly_root_requires_its_exact_version_and_old_hosts_refuse() {
    let writable = RootfsImage::new(
        format!("sha256:{}", "a".repeat(64)),
        16 * 1024 * 1024,
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let source = bundle(&writable);
    let mut descriptor = serde_json::to_value(&writable).unwrap();
    descriptor["schemaVersion"] = json!(2);
    let mut config: serde_json::Value = serde_json::from_str(source.config_json()).unwrap();
    config["root"]["readonly"] = json!(true);
    config["annotations"][ROOTFS_IMAGE_EXTENSION] = json!(descriptor.to_string());
    let readonly = OciBundle::from_json(source.directory(), config.to_string()).unwrap();
    let image = RootfsImage::from_bundle(&readonly).unwrap().unwrap();
    assert_eq!(serde_json::to_value(&image).unwrap(), descriptor);
    let attachments = CreateAttachments::from_bundle(&readonly, ProcessIo::default())
        .unwrap()
        .with_rootfs_image(&readonly)
        .unwrap();
    assert_eq!(attachments.rootfs_image(&readonly).unwrap(), Some(image));
    let old = AttachmentCapabilities::base_v2()
        .with_extension(ROOTFS_IMAGE_EXTENSION, vec![1])
        .unwrap();
    assert_eq!(
        old.require(&attachments).unwrap_err().code,
        ErrorCode::Unsupported
    );
    AttachmentCapabilities::base_v2()
        .with_extension(ROOTFS_IMAGE_EXTENSION, vec![1, 2])
        .unwrap()
        .require(&attachments)
        .unwrap();
    config["root"]["readonly"] = json!(false);
    assert!(RootfsImage::from_bundle(
        &OciBundle::from_json(source.directory(), config.to_string()).unwrap()
    )
    .is_err());
    let mut mismatch = serde_json::to_value(&attachments).unwrap();
    mismatch["extensions"][ROOTFS_IMAGE_EXTENSION]["version"] = json!(1);
    assert!(serde_json::from_value::<CreateAttachments>(mismatch)
        .unwrap()
        .validate(&readonly)
        .is_err());
}

#[test]
fn workload_root_versions_preserve_legacy_bytes_and_validate_before_upgrade() {
    let image = RootfsImage::new(
        format!("sha256:{}", "a".repeat(64)),
        16 * 1024 * 1024,
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    assert_eq!(image.version(), ROOTFS_IMAGE_VERSION);
    assert!(!image.readonly_root());
    assert_eq!(
        image.annotation_value().unwrap(),
        format!(
            r#"{{"schemaVersion":1,"sha256":"sha256:{}","size":16777216,"imageDigest":"sha256:{}"}}"#,
            "a".repeat(64),
            "b".repeat(64)
        )
    );
    let readonly = image.clone().with_readonly_root().unwrap();
    assert_eq!(readonly.version(), ROOTFS_IMAGE_READONLY_VERSION);
    assert!(readonly.readonly_root());
    assert_eq!(readonly.clone().with_readonly_root().unwrap(), readonly);
    let mut invalid = serde_json::to_value(image).unwrap();
    invalid["sha256"] = json!("sha256:bad");
    assert!(serde_json::from_value::<RootfsImage>(invalid)
        .unwrap()
        .with_readonly_root()
        .is_err());
}
