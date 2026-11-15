use super::*;
use crate::{AttachmentCapabilities, ProcessIo};
use serde_json::json;

const MEMORY: &str = "dev.kbox.memory-secret-files";

fn fixture() -> OciBundle {
    let files = FileGrants::new(vec![FileGrant::new(
        0,
        StorageAttachmentId::new("private-secret-file").unwrap(),
        FileSourceIdentity::new(4, 71).unwrap(),
        StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let mut config = serde_json::to_value(crate::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":false});
    config["mounts"] = json!([{"type":"bind","source":"/private/secret file","destination":"/run/secrets/file","options":["bind","ro"]}]);
    config["annotations"] = json!({FILE_GRANTS_EXTENSION:files.annotation_value().unwrap(),MEMORY:json!({"schemaVersion":1,"mountIndices":[0]}).to_string()});
    OciBundle::from_json(
        std::env::temp_dir().join("memory-secret-grants"),
        config.to_string(),
    )
    .unwrap()
}

#[test]
fn memory_secret_raw_storage_requires_v3_exact_intent_and_required_capability() {
    for environment in [false, true] {
        let original = if environment {
            environment_fixture(2)
        } else {
            fixture()
        };
        let mut value: Value = serde_json::from_str(original.config_json()).unwrap();
        value["mounts"].as_array_mut().unwrap().push(json!({
            "source":"/caller/anonymous.ext4","destination":"/data","type":"ext4",
            "options":["rw","nosuid","nodev"]
        }));
        value["annotations"][crate::RAW_STORAGE_IMAGES_EXTENSION] =
            json!(crate::RAW_STORAGE_IMAGES_VALUE);
        value["annotations"][MEMORY] =
            json!(json!({"schemaVersion":3,"mountIndices":[0]}).to_string());
        let bundle = OciBundle::from_json(original.directory(), value.to_string()).unwrap();
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
            .unwrap()
            .attach_storage_mount(
                &bundle,
                1,
                StorageAttachmentId::new("raw-secret-disk").unwrap(),
                StorageAccessMode::ReadWrite,
                StorageOwnership::Caller,
                StorageCleanup::DetachOnly,
            )
            .unwrap();
        attachments.validate(&bundle).unwrap();
        let memory = attachments.memory_secret_files(&bundle).unwrap().unwrap();
        assert_eq!(memory.version(), 3);
        assert_eq!(
            memory.environment_bindings(&bundle).unwrap().len(),
            usize::from(environment)
        );
        let old = AttachmentCapabilities::base_v2()
            .with_extension(FILE_GRANTS_EXTENSION, vec![1])
            .unwrap()
            .with_extension(crate::RAW_STORAGE_IMAGES_EXTENSION, vec![1])
            .unwrap()
            .with_extension(MEMORY, vec![1, 2])
            .unwrap();
        assert_eq!(
            old.require(&attachments).unwrap_err().code,
            crate::ErrorCode::Unsupported
        );
        AttachmentCapabilities::base_v2()
            .with_extension(FILE_GRANTS_EXTENSION, vec![1])
            .unwrap()
            .with_extension(crate::RAW_STORAGE_IMAGES_EXTENSION, vec![1])
            .unwrap()
            .with_extension(MEMORY, vec![1, 2, 3])
            .unwrap()
            .require(&attachments)
            .unwrap();
        for version in [1, 2] {
            let mut legacy = value.clone();
            legacy["annotations"][MEMORY] =
                json!(json!({"schemaVersion":version,"mountIndices":[0]}).to_string());
            let legacy = OciBundle::from_json(bundle.directory(), legacy.to_string()).unwrap();
            assert!(CreateAttachments::from_bundle(&legacy, ProcessIo::default()).is_err());
        }
        value["annotations"]
            .as_object_mut()
            .unwrap()
            .remove(crate::RAW_STORAGE_IMAGES_EXTENSION);
        value["mounts"].as_array_mut().unwrap().pop();
        let missing_raw = OciBundle::from_json(bundle.directory(), value.to_string()).unwrap();
        assert!(CreateAttachments::from_bundle(&missing_raw, ProcessIo::default()).is_err());
    }
}

#[test]
fn memory_secret_files_require_exact_extension_and_old_hosts_refuse() {
    let bundle = fixture();
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    assert!(attachments.extensions().contains_key(MEMORY));
    let old = AttachmentCapabilities::base_v2()
        .with_extension(FILE_GRANTS_EXTENSION, vec![1])
        .unwrap();
    assert_eq!(
        old.require(&attachments).unwrap_err().code,
        crate::ErrorCode::Unsupported
    );
    let value = serde_json::to_value(&attachments).unwrap();
    assert_eq!(
        value["secrets"],
        json!([{"kind":"runtime-extension","name":MEMORY}])
    );
}

#[test]
fn memory_secret_files_refuse_bad_or_unbound_indices() {
    for value in [
        json!({"schemaVersion":2,"mountIndices":[0]}),
        json!({"schemaVersion":1,"mountIndices":[]}),
        json!({"schemaVersion":1,"mountIndices":[0,0]}),
        json!({"schemaVersion":1,"mountIndices":[1]}),
        json!({"schemaVersion":1,"mountIndices":[0],"unknown":true}),
    ] {
        let bundle = fixture();
        let mut config: Value = serde_json::from_str(bundle.config_json()).unwrap();
        config["annotations"][MEMORY] = json!(value.to_string());
        let changed = OciBundle::from_json(bundle.directory(), config.to_string()).unwrap();
        assert!(
            CreateAttachments::from_bundle(&changed, ProcessIo::default()).is_err(),
            "{value}"
        );
    }
}

#[test]
fn memory_secret_files_cannot_drop_required_classification_or_write_access() {
    let bundle = fixture();
    let original = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    for change in ["required", "version", "classification", "storage"] {
        let mut value = serde_json::to_value(&original).unwrap();
        match change {
            "required" => value["extensions"][MEMORY]["required"] = json!(false),
            "version" => value["extensions"][MEMORY]["version"] = json!(2),
            "classification" => value["secrets"] = json!([]),
            "storage" => value["storage"] = json!([]),
            _ => unreachable!(),
        }
        let changed: CreateAttachments = serde_json::from_value(value).unwrap();
        assert!(changed.validate(&bundle).is_err(), "{change}");
        assert!(changed.memory_secret_files(&bundle).is_err(), "{change}");
    }
    let mut config: Value = serde_json::from_str(bundle.config_json()).unwrap();
    let mut files: Value = serde_json::from_str(
        config["annotations"][FILE_GRANTS_EXTENSION]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    files["entries"][0]["accessMode"] = json!("read-write");
    config["annotations"][FILE_GRANTS_EXTENSION] = json!(files.to_string());
    config["mounts"][0]["options"] = json!(["bind", "rw"]);
    let rw = OciBundle::from_json(bundle.directory(), config.to_string()).unwrap();
    assert!(CreateAttachments::from_bundle(&rw, ProcessIo::default()).is_err());
}

fn environment_fixture(version: u16) -> OciBundle {
    let original = fixture();
    let mut value: Value = serde_json::from_str(original.config_json()).unwrap();
    let path = format!("/.kbox-secrets/{}/000.secret", "a".repeat(64));
    value["mounts"][0]["destination"] = json!(path);
    value["process"]["env"] = json!([format!(
        "K_BOX_SECRET_ENV_V1={}",
        json!([{"variable":"SYNTHETIC_ENV","path":path}])
    )]);
    value["annotations"][MEMORY] =
        json!(json!({"schemaVersion":version,"mountIndices":[0]}).to_string());
    OciBundle::from_json(original.directory(), value.to_string()).unwrap()
}

#[test]
fn memory_secret_environment_requires_explicit_v2_and_old_capability_refusal() {
    let bundle = environment_fixture(2);
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    assert_eq!(attachments.extensions()[MEMORY].version(), 2);
    let old = AttachmentCapabilities::base_v2()
        .with_extension(FILE_GRANTS_EXTENSION, vec![1])
        .unwrap()
        .with_extension(MEMORY, vec![1])
        .unwrap();
    assert_eq!(
        old.require(&attachments).unwrap_err().code,
        crate::ErrorCode::Unsupported
    );
    let new = AttachmentCapabilities::base_v2()
        .with_extension(FILE_GRANTS_EXTENSION, vec![1])
        .unwrap()
        .with_extension(MEMORY, vec![1, 2])
        .unwrap();
    new.require(&attachments).unwrap();
}

#[test]
fn memory_secret_environment_cannot_reuse_files_only_v1_semantics() {
    let bundle = environment_fixture(1);
    assert!(
        CreateAttachments::from_bundle(&bundle, ProcessIo::default()).is_err(),
        "files-only version1 silently admitted environment projection"
    );
}

#[test]
fn memory_secret_environment_refuses_unbound_duplicate_and_malformed_metadata() {
    let original = environment_fixture(2);
    for change in [
        "path",
        "variable",
        "duplicate",
        "literal",
        "missing",
        "oversize",
        "unknown",
        "bare",
    ] {
        let mut value: Value = serde_json::from_str(original.config_json()).unwrap();
        let encoded = value["process"]["env"][0]
            .as_str()
            .unwrap()
            .split_once('=')
            .unwrap()
            .1;
        let mut bindings: Value = serde_json::from_str(encoded).unwrap();
        match change {
            "path" => {
                bindings[0]["path"] = json!(format!("/.kbox-secrets/{}/001.secret", "b".repeat(64)))
            }
            "variable" => bindings[0]["variable"] = json!("9INVALID"),
            "duplicate" => {
                let duplicate = bindings[0].clone();
                bindings.as_array_mut().unwrap().push(duplicate);
            }
            "unknown" => bindings[0]["payload"] = json!("never-echo-this-synthetic-material"),
            _ => {}
        }
        value["process"]["env"] = json!([format!("K_BOX_SECRET_ENV_V1={bindings}")]);
        match change {
            "literal" => value["process"]["env"]
                .as_array_mut()
                .unwrap()
                .push(json!("SYNTHETIC_ENV=untrusted")),
            "missing" => value["process"]["env"] = json!([]),
            "oversize" => {
                value["process"]["env"] =
                    json!([format!("K_BOX_SECRET_ENV_V1={}", " ".repeat(65537))])
            }
            "bare" => value["process"]["env"] = json!(["K_BOX_SECRET_ENV_V1"]),
            _ => {}
        }
        let error = match OciBundle::from_json(original.directory(), value.to_string()) {
            Ok(bundle) => {
                CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap_err()
            }
            Err(error) => error,
        };
        assert_eq!(error.code, crate::ErrorCode::InvalidArgument, "{change}");
        assert!(!format!("{error:?}").contains("never-echo-this-synthetic-material"));
    }
}

#[test]
fn memory_secret_environment_metadata_parser_refuses_bare_control_key() {
    assert!(
        MemorySecretEnvironmentBinding::from_environment(&["K_BOX_SECRET_ENV_V1".into()]).is_err()
    );
}
