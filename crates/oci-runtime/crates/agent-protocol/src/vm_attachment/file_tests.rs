use super::*;
use koci_sdk::{
    ContainerId, CreateAttachments, FileGrant, FileGrants, FileSourceIdentity, Generation,
    ProcessIo, StorageAccessMode, StorageAttachmentId,
};
use serde_json::json;

fn fixture() -> (OciBundle, AgentVmAttachmentManifest) {
    fixture_with_nproc(false)
}

fn fixture_with_nproc(nproc: bool) -> (OciBundle, AgentVmAttachmentManifest) {
    let grants = FileGrants::new(vec![FileGrant::new(
        0,
        StorageAttachmentId::new("one-file").unwrap(),
        FileSourceIdentity::new(4, 17).unwrap(),
        StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let mut c = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
    c["root"] = json!({"path":"rootfs","readonly":true});
    c["mounts"] = json!([{"type":"bind","source":"/caller/中文 file","destination":"/data/file","options":["bind","ro"]}]);
    c["annotations"] = json!({koci_sdk::FILE_GRANTS_EXTENSION:grants.annotation_value().unwrap()});
    if nproc {
        c["process"]["rlimits"] = json!([{"type":"RLIMIT_NPROC","soft":16,"hard":16}]);
    }
    let bundle =
        OciBundle::from_json(std::env::temp_dir().join("file-manifest"), c.to_string()).unwrap();
    let a = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let f = AgentVmFileAttachment::new(
        a.storage()[0].clone(),
        "/caller/中文 file",
        grants.entries()[0].source_identity(),
        AgentVmDirectorySourceIdentity::new(4, 18).unwrap(),
        bundle.config_digest(),
    )
    .unwrap();
    let m = AgentVmAttachmentManifest::new_with_file_grants(
        ContainerTarget::exact(ContainerId::new("one-file").unwrap(), Generation(3)),
        GuestPath::new("/run/koci-runtime/bundle").unwrap(),
        &bundle,
        a.digest().unwrap(),
        vec![],
        vec![f],
        None,
    )
    .unwrap();
    (bundle, m)
}
#[test]
fn files_are_explicit_v6_and_require_exact_bundle_incarnation() {
    let (bundle, m) = fixture();
    let bytes = m.to_bytes().unwrap();
    assert_eq!(AgentVmAttachmentManifest::from_bytes(&bytes).unwrap(), m);
    m.validate_bundle(&bundle).unwrap();
    assert_eq!(m.schema_version, AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION);
    assert!(m.files()[0].guest_file_relative_path().ends_with("/file"));
    for field in [
        "schemaVersion",
        "tag",
        "sourceIdentity",
        "ownership",
        "cleanup",
        "hostSource",
        "projectionIdentity",
    ] {
        let mut v = serde_json::to_value(&m).unwrap();
        match field {
            "schemaVersion" => v[field] = json!(AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION),
            "tag" => v["files"][0][field] = json!("kfile-000000000000000000000000"),
            "sourceIdentity" => v["files"][0][field]["inode"] = json!(999),
            "ownership" => v["files"][0]["attachment"][field] = json!("runtime"),
            "cleanup" => v["files"][0]["attachment"][field] = json!("delete"),
            "hostSource" => v["files"][0][field] = json!("/caller/replacement"),
            _ => v["files"][0][field]["inode"] = json!(0),
        }
        let result = serde_json::from_value::<AgentVmAttachmentManifest>(v)
            .map(|m| m.validate_bundle(&bundle).is_err());
        assert!(result.is_err() || result.unwrap(), "{field}");
    }
    let mut v = serde_json::to_value(&m).unwrap();
    let duplicate = v["files"][0].clone();
    v["files"].as_array_mut().unwrap().push(duplicate);
    let invalid: AgentVmAttachmentManifest = serde_json::from_value(v).unwrap();
    assert!(invalid.validate_bundle(&bundle).is_err());
}

#[test]
fn memory_secret_files_require_a_new_manifest_and_refuse_downgrade() {
    let (ordinary, old) = fixture();
    let mut config: Value = serde_json::from_str(ordinary.config_json()).unwrap();
    config["annotations"][koci_sdk::MEMORY_SECRET_FILES_EXTENSION] =
        json!(koci_sdk::MemorySecretFiles::new(vec![0])
            .unwrap()
            .annotation_value()
            .unwrap());
    let bundle = OciBundle::from_json(ordinary.directory(), config.to_string()).unwrap();
    let a = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let file = AgentVmFileAttachment::new(
        a.storage()[0].clone(),
        old.files()[0].host_source(),
        old.files()[0].source_identity(),
        old.files()[0].projection_identity(),
        bundle.config_digest(),
    )
    .unwrap();
    let manifest = AgentVmAttachmentManifest::new_with_file_grants(
        old.target().clone(),
        old.guest_bundle().clone(),
        &bundle,
        a.digest().unwrap(),
        vec![],
        vec![file],
        None,
    )
    .unwrap();
    assert_eq!(manifest.schema_version, "koci.agent-vm-attachments.v8");
    let mut downgraded = serde_json::to_value(&manifest).unwrap();
    downgraded["schemaVersion"] = json!(AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION);
    let downgraded: AgentVmAttachmentManifest = serde_json::from_value(downgraded).unwrap();
    assert!(downgraded.validate_bundle(&bundle).is_err());
    let mut forged = serde_json::to_value(&old).unwrap();
    forged["schemaVersion"] = json!("koci.agent-vm-attachments.v8");
    let forged: AgentVmAttachmentManifest = serde_json::from_value(forged).unwrap();
    assert!(forged.validate_bundle(&ordinary).is_err());
}

#[test]
fn memory_secret_environment_requires_manifest_v9_and_refuses_files_only_downgrade() {
    let (ordinary, old) = fixture();
    let mut config: Value = serde_json::from_str(ordinary.config_json()).unwrap();
    let path = format!("/.kbox-secrets/{}/000.secret", "a".repeat(64));
    config["mounts"][0]["destination"] = json!(path);
    config["process"]["env"] = json!([format!(
        "K_BOX_SECRET_ENV_V1={}",
        json!([{"variable":"SYNTHETIC_ENV","path":path}])
    )]);
    config["annotations"][koci_sdk::MEMORY_SECRET_FILES_EXTENSION] =
        json!(koci_sdk::MemorySecretFiles::new_for_environment(vec![0])
            .unwrap()
            .annotation_value()
            .unwrap());
    let bundle = OciBundle::from_json(ordinary.directory(), config.to_string()).unwrap();
    let a = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let file = AgentVmFileAttachment::new(
        a.storage()[0].clone(),
        old.files()[0].host_source(),
        old.files()[0].source_identity(),
        old.files()[0].projection_identity(),
        bundle.config_digest(),
    )
    .unwrap();
    let manifest = AgentVmAttachmentManifest::new_with_file_grants(
        old.target().clone(),
        old.guest_bundle().clone(),
        &bundle,
        a.digest().unwrap(),
        vec![],
        vec![file],
        None,
    )
    .unwrap();
    assert_eq!(manifest.schema_version, "koci.agent-vm-attachments.v9");
    for version in [
        AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION,
        AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION,
    ] {
        let mut downgraded = serde_json::to_value(&manifest).unwrap();
        downgraded["schemaVersion"] = json!(version);
        let downgraded: AgentVmAttachmentManifest = serde_json::from_value(downgraded).unwrap();
        assert!(downgraded.validate_bundle(&bundle).is_err(), "{version}");
    }
    let mut forged = serde_json::to_value(&old).unwrap();
    forged["schemaVersion"] = json!("koci.agent-vm-attachments.v9");
    let forged: AgentVmAttachmentManifest = serde_json::from_value(forged).unwrap();
    assert!(forged.validate_bundle(&ordinary).is_err());
}

#[test]
fn nproc_requires_explicit_v10_without_reinterpreting_prior_transports() {
    let (legacy_bundle, legacy) = fixture();
    let legacy_bytes = legacy.to_bytes().unwrap();
    assert!(!String::from_utf8_lossy(&legacy_bytes).contains("nprocTransportSchema"));
    assert_eq!(
        legacy
            .clone()
            .with_nproc_admission(&legacy_bundle)
            .unwrap()
            .to_bytes()
            .unwrap(),
        legacy_bytes
    );
    let (bundle, guarded) = fixture_with_nproc(true);
    assert_eq!(
        guarded.schema_version,
        AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        guarded.transport_schema(),
        AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        AgentVmAttachmentManifest::from_bytes(&guarded.to_bytes().unwrap()).unwrap(),
        guarded
    );
    guarded.validate_bundle(&bundle).unwrap();
    let mut downgraded = guarded.clone();
    downgraded.schema_version = downgraded.nproc_transport_schema.take().unwrap();
    assert!(downgraded.validate_bundle(&bundle).is_err());
    for base in [
        None,
        Some(AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION.into()),
        Some("unknown".into()),
    ] {
        let mut invalid = guarded.clone();
        invalid.nproc_transport_schema = base;
        assert!(invalid.validate().is_err());
    }
    let mut invalid = legacy;
    invalid.nproc_transport_schema = Some(AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION.into());
    assert!(invalid.validate().is_err());
}
