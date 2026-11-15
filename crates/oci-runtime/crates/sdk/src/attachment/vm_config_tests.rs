use super::*;
use crate::{AttachmentCapabilities, ProcessIo};
use serde_json::json;

fn bundle(value: Option<&str>) -> OciBundle {
    let mut config = serde_json::to_value(crate::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    if let Some(value) = value {
        config["annotations"] = json!({VM_RESOURCES_EXTENSION:value});
    }
    OciBundle::from_json(
        std::env::temp_dir().join("vm-sizing-sdk"),
        config.to_string(),
    )
    .unwrap()
}

#[test]
fn explicit_vm_resources_are_bound_as_required_and_old_hosts_refuse() {
    let requested = VmConfig::new(1, 768).unwrap();
    let bundle = bundle(Some(&requested.annotation_value().unwrap()));
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let extension = attachments
        .extensions()
        .get(VM_RESOURCES_EXTENSION)
        .expect("explicit VM resources must be declared to old Hosts");
    assert!(extension.required());
    assert_eq!(extension.version(), VM_RESOURCES_VERSION);
    assert_eq!(attachments.vm_config(&bundle).unwrap(), Some(requested));
    let error = AttachmentCapabilities::base_v1()
        .require(&attachments)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert_eq!(error.operation.as_deref(), Some("create"));
    assert!(error.message.contains(VM_RESOURCES_EXTENSION));
    AttachmentCapabilities::base_v1()
        .with_extension(VM_RESOURCES_EXTENSION, vec![VM_RESOURCES_VERSION])
        .unwrap()
        .require(&attachments)
        .unwrap();
}

#[test]
fn vm_annotation_cannot_bypass_admission_with_a_legacy_manifest() {
    let legacy = bundle(None);
    let attachments = CreateAttachments::from_bundle(&legacy, ProcessIo::default()).unwrap();
    let changed = bundle(Some(
        &VmConfig::new(1, 768).unwrap().annotation_value().unwrap(),
    ));
    assert_eq!(
        attachments.validate(&changed).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn vm_manifest_cannot_change_requirement_version_or_bound_configuration() {
    let original = bundle(Some(
        &VmConfig::new(1, 768).unwrap().annotation_value().unwrap(),
    ));
    let attachments = CreateAttachments::from_bundle(&original, ProcessIo::default()).unwrap();
    let changed = bundle(Some(
        &VmConfig::new(1, 1024).unwrap().annotation_value().unwrap(),
    ));
    assert_eq!(
        attachments.vm_config(&changed).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    for field in ["optional", "version", "name", "pointer", "digest"] {
        let mut value = serde_json::to_value(&attachments).unwrap();
        let extension = &mut value["extensions"][VM_RESOURCES_EXTENSION];
        match field {
            "optional" => extension["required"] = json!(false),
            "version" => extension["version"] = json!(2),
            "name" => extension["name"] = json!("dev.kbox.unrelated"),
            "pointer" => extension["configuration"]["jsonPointer"] = json!("/root"),
            "digest" => {
                extension["configuration"]["valueDigest"] =
                    json!(format!("sha256:{}", "0".repeat(64)))
            }
            _ => unreachable!(),
        }
        let altered: CreateAttachments = serde_json::from_value(value).unwrap();
        assert_eq!(
            altered.vm_config(&original).unwrap_err().code,
            ErrorCode::InvalidArgument,
            "{field}"
        );
        assert_eq!(
            altered.validate(&original).unwrap_err().code,
            ErrorCode::InvalidArgument,
            "{field}"
        );
    }
}

#[test]
fn legacy_vm_profile_has_no_new_extension_or_changed_default() {
    let bundle = bundle(None);
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    assert_eq!(VmConfig::from_bundle(&bundle).unwrap(), None);
    assert_eq!(attachments.vm_config(&bundle).unwrap(), None);
    assert!(!attachments
        .extensions()
        .contains_key(VM_RESOURCES_EXTENSION));
    AttachmentCapabilities::base_v1()
        .require(&attachments)
        .unwrap();
    assert_eq!(
        (
            VmConfig::default().vcpus(),
            VmConfig::default().memory_mib()
        ),
        (1, 512)
    );
}

#[test]
fn vm_resources_reuse_profile_bounds_and_preserve_windows_gate() {
    for cpus in [1, 2, 4, 8] {
        for memory in [128, 256, 768, 8192] {
            let result = VmConfig::new(cpus, memory);
            if cfg!(all(target_os = "windows", target_arch = "x86_64")) && cpus != 1 {
                assert_eq!(result.unwrap_err().code, ErrorCode::InvalidArgument);
            } else {
                let requested = result.unwrap();
                let bundle = bundle(Some(&requested.annotation_value().unwrap()));
                assert_eq!(VmConfig::from_bundle(&bundle).unwrap(), Some(requested));
            }
        }
    }
    for cpus in [0, 3, 9, 255] {
        assert_eq!(
            VmConfig::new(cpus, 512).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
    }
    for memory in [0, 127, 129, 8193, u32::MAX] {
        assert_eq!(
            VmConfig::new(1, memory).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
    }
}

#[test]
fn malformed_vm_resource_descriptors_fail_closed() {
    for value in [
        "{}",
        "null",
        "[]",
        "true",
        "not-json",
        r#"{"schemaVersion":2,"vcpus":1,"memoryMib":512}"#,
        r#"{"schemaVersion":1,"vcpus":0,"memoryMib":512}"#,
        r#"{"schemaVersion":1,"vcpus":1,"memoryMib":129}"#,
        r#"{"schemaVersion":1,"vcpus":1,"memoryMib":512,"ignored":true}"#,
        r#"{"schemaVersion":1,"vcpus":1,"vcpus":2,"memoryMib":512}"#,
        r#"{"schemaVersion":1,"vcpus":1.0,"memoryMib":512}"#,
        r#"{"schemaVersion":1,"vcpus":256,"memoryMib":512}"#,
        r#"{"schemaVersion":1,"vcpus":1,"memoryMiB":512}"#,
    ] {
        let error = VmConfig::from_bundle(&bundle(Some(value))).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArgument, "{value}");
        assert_eq!(error.operation.as_deref(), Some("validate-attachments"));
    }
    assert_eq!(
        VmConfig::from_bundle(&bundle(Some(&" ".repeat(1025))))
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
}
