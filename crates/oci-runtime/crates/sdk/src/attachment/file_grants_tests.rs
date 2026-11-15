use super::*;
use crate::{AttachmentCapabilities, ErrorCode, ProcessIo};
use serde_json::json;

fn fixture() -> OciBundle {
    let grants = FileGrants::new(vec![FileGrant::new(
        0,
        StorageAttachmentId::new("caller-file").unwrap(),
        FileSourceIdentity::new(4, 71).unwrap(),
        StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let mut config = serde_json::to_value(crate::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    config["mounts"] = json!([{"type":"bind","source":"/caller/中文 file","destination":"/data/file","options":["bind","ro"]}]);
    config["annotations"] = json!({FILE_GRANTS_EXTENSION:grants.annotation_value().unwrap()});
    OciBundle::from_json(std::env::temp_dir().join("file-grants"), config.to_string()).unwrap()
}

#[test]
fn file_grants_are_required_exact_caller_storage_and_old_hosts_refuse() {
    let bundle = fixture();
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    assert_eq!(attachments.storage().len(), 1);
    let files = attachments.file_grants(&bundle).unwrap().unwrap();
    assert_eq!(
        files.entries()[0].source_identity(),
        FileSourceIdentity::new(4, 71).unwrap()
    );
    assert_eq!(
        attachments.storage()[0].ownership(),
        StorageOwnership::Caller
    );
    assert_eq!(
        attachments.storage()[0].cleanup(),
        StorageCleanup::DetachOnly
    );
    assert_eq!(
        AttachmentCapabilities::base_v2()
            .require(&attachments)
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    AttachmentCapabilities::base_v2()
        .with_extension(FILE_GRANTS_EXTENSION, vec![1])
        .unwrap()
        .require(&attachments)
        .unwrap();
    for field in ["version", "required"] {
        let mut value = serde_json::to_value(&attachments).unwrap();
        value["extensions"][FILE_GRANTS_EXTENSION][field] = if field == "version" {
            json!(2)
        } else {
            json!(false)
        };
        let changed: CreateAttachments = serde_json::from_value(value).unwrap();
        assert!(changed.validate(&bundle).is_err());
    }
}

#[test]
fn file_grants_refuse_aliases_drift_paths_and_unbounded_values() {
    let bundle = fixture();
    let files = FileGrants::from_bundle(&bundle).unwrap().unwrap();
    assert!(FileGrants::new(vec![files.entries()[0].clone(), files.entries()[0].clone()]).is_err());
    assert!(FileSourceIdentity::new(1, 0).is_err());
    for field in ["source", "destination", "options", "type"] {
        let mut config: Value = serde_json::from_str(bundle.config_json()).unwrap();
        config["mounts"][0][field] = match field {
            "source" => json!("/caller/../other"),
            "destination" => json!("/"),
            "options" => json!(["bind", "rw"]),
            _ => json!("ext4"),
        };
        assert!(CreateAttachments::from_bundle(
            &OciBundle::from_json(bundle.directory(), config.to_string()).unwrap(),
            ProcessIo::default()
        )
        .is_err());
    }
    let mut config: Value = serde_json::from_str(bundle.config_json()).unwrap();
    config["annotations"][FILE_GRANTS_EXTENSION] = json!("x".repeat(MAX_BYTES + 1));
    assert!(FileGrants::from_bundle(
        &OciBundle::from_json(bundle.directory(), config.to_string()).unwrap()
    )
    .is_err());
}

#[test]
fn file_descriptor_cannot_claim_runtime_ownership_or_cleanup() {
    let bundle = fixture();
    let grants = FileGrants::from_bundle(&bundle).unwrap().unwrap();
    for field in ["ownership", "cleanup"] {
        let mut value = serde_json::to_value(&grants).unwrap();
        value["entries"][0][field] = json!(if field == "ownership" {
            "runtime"
        } else {
            "delete"
        });
        let mut config: Value = serde_json::from_str(bundle.config_json()).unwrap();
        config["annotations"][FILE_GRANTS_EXTENSION] = json!(value.to_string());
        assert!(FileGrants::from_bundle(
            &OciBundle::from_json(bundle.directory(), config.to_string()).unwrap()
        )
        .is_err());
    }
}
