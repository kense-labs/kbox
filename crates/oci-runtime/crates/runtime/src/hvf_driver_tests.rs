use super::*;
use koci_core::DriverKind;
use koci_sdk::{CreateAttachments, OciBundle, ProcessIo};
use serde_json::json;

fn capability() -> DriverCapability {
    DriverCapability {
        driver: DriverKind::LibkHvf,
        status: CapabilityStatus::Available,
        readiness: DriverReadiness::Experimental,
        isolation_classes: vec![IsolationClass::DedicatedVm],
        reason: None,
        evidence: Default::default(),
    }
}

fn ordinary_bundle() -> OciBundle {
    let mut config = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    OciBundle::from_json(
        std::env::temp_dir().join("hvf-probe-test"),
        config.to_string(),
    )
    .unwrap()
}

#[test]
fn optional_memory_probe_failure_preserves_ordinary_admission_and_refuses_secrets() {
    let mut driver = capability();
    let error = Error::new(
        ErrorCode::FailedPrecondition,
        "synthetic disk service unavailable",
    )
    .for_operation("verify-macos-memory-backing");
    let capabilities = memory_secret_capabilities(&mut driver, Err(error)).unwrap();
    assert!(driver.can_launch());
    assert_eq!(driver.readiness, DriverReadiness::Experimental);
    assert_eq!(driver.isolation_classes, vec![IsolationClass::DedicatedVm]);
    assert_eq!(
        driver.evidence["memory_secret_files"],
        "unavailable-memory-backing-not-verified"
    );
    assert!(driver.evidence["memory_secret_files_probe_error"]
        .contains("synthetic disk service unavailable"));
    for version in [1, 2, 3] {
        assert!(!capabilities.supports_extension(koci_sdk::MEMORY_SECRET_FILES_EXTENSION, version));
    }

    let bundle = ordinary_bundle();
    let ordinary = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    capabilities.require(&ordinary).unwrap();
    let files = koci_sdk::FileGrants::new(vec![koci_sdk::FileGrant::new(
        0,
        koci_sdk::StorageAttachmentId::new("probe-test-secret").unwrap(),
        koci_sdk::FileSourceIdentity::new(4, 71).unwrap(),
        koci_sdk::StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let mut config: serde_json::Value = serde_json::from_str(bundle.config_json()).unwrap();
    config["mounts"] = json!([{"type":"bind","source":"/caller/secret","destination":"/run/secrets/file","options":["bind","ro"]}]);
    config["annotations"] = json!({
        koci_sdk::FILE_GRANTS_EXTENSION: files.annotation_value().unwrap(),
        koci_sdk::MEMORY_SECRET_FILES_EXTENSION: json!({"schemaVersion":1,"mountIndices":[0]}).to_string()
    });
    let secret_bundle = OciBundle::from_json(bundle.directory(), config.to_string()).unwrap();
    let secret = CreateAttachments::from_bundle(&secret_bundle, ProcessIo::default()).unwrap();
    let refusal = capabilities
        .with_extension(koci_sdk::FILE_GRANTS_EXTENSION, vec![1])
        .unwrap()
        .require(&secret)
        .unwrap_err();
    assert_eq!(refusal.code, ErrorCode::Unsupported);
    assert_eq!(refusal.operation.as_deref(), Some("create"));
    assert!(refusal
        .message
        .contains(koci_sdk::MEMORY_SECRET_FILES_EXTENSION));
}

#[test]
fn optional_memory_capability_requires_positive_backing_proof() {
    for proven in [false, true] {
        let mut driver = capability();
        let capabilities = memory_secret_capabilities(&mut driver, Ok(proven)).unwrap();
        for version in [1, 2, 3] {
            assert_eq!(
                capabilities.supports_extension(koci_sdk::MEMORY_SECRET_FILES_EXTENSION, version),
                proven
            );
        }
        assert!(!driver
            .evidence
            .contains_key("memory_secret_files_probe_error"));
    }
}

#[test]
fn unexpected_memory_probe_errors_are_not_converted_to_capability_absence() {
    let mut driver = capability();
    let error = memory_secret_capabilities(
        &mut driver,
        Err(Error::new(ErrorCode::Internal, "synthetic worker failure")
            .for_operation("open-hvf-runtime-driver")),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Internal);
    assert_eq!(error.operation.as_deref(), Some("open-hvf-runtime-driver"));
    assert_eq!(error.message, "synthetic worker failure");
}
