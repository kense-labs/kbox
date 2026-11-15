use super::*;
use crate::{AttachmentCapabilities, ErrorCode, ProcessIo};
use serde_json::json;

fn bundle(value: &str, mount_type: &str) -> OciBundle {
    let mut config = serde_json::to_value(crate::oci_spec::runtime::Spec::default()).unwrap();
    config["annotations"] = json!({RAW_STORAGE_IMAGES_EXTENSION:value});
    config["mounts"] = json!([{"destination":"/data","type":mount_type,"source":"/caller/volume.ext4","options":["rw","nosuid","nodev"]}]);
    OciBundle::from_json(
        std::env::temp_dir().join("raw-storage-contract"),
        config.to_string(),
    )
    .unwrap()
}

#[test]
fn raw_storage_requires_an_exact_token_and_supported_required_capability() {
    let bundle = bundle(RAW_STORAGE_IMAGES_VALUE, "ext4");
    let contract = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    assert!(contract.raw_storage_images(&bundle).unwrap());
    assert!(
        contract.storage().is_empty(),
        "the feature token must not grant source authority"
    );
    assert_eq!(
        AttachmentCapabilities::base_v2()
            .require(&contract)
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    AttachmentCapabilities::base_v2()
        .with_extension(RAW_STORAGE_IMAGES_EXTENSION, vec![1])
        .unwrap()
        .require(&contract)
        .unwrap();
    for change in ["missing", "optional", "future"] {
        let mut encoded = serde_json::to_value(&contract).unwrap();
        match change {
            "missing" => {
                encoded["extensions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(RAW_STORAGE_IMAGES_EXTENSION);
            }
            "optional" => {
                encoded["extensions"][RAW_STORAGE_IMAGES_EXTENSION]["required"] = json!(false)
            }
            "future" => encoded["extensions"][RAW_STORAGE_IMAGES_EXTENSION]["version"] = json!(2),
            _ => unreachable!(),
        }
        let changed: CreateAttachments = serde_json::from_value(encoded).unwrap();
        assert!(changed.validate(&bundle).is_err(), "{change}");
    }
}

#[test]
fn raw_storage_rejects_unknown_or_unbound_annotation_values() {
    for (value, kind) in [
        ("", "ext4"),
        ("raw-ext4-v2", "ext4"),
        (RAW_STORAGE_IMAGES_VALUE, "bind"),
    ] {
        assert!(
            CreateAttachments::from_bundle(&bundle(value, kind), ProcessIo::default()).is_err()
        );
    }
}
