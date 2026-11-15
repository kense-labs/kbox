use super::*;

#[test]
fn hvf_memory_secret_disk_root_and_environment_refuse_before_preparation() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.request.config.volumes.clear();
    metadata.request.policy.volume_names.clear();
    record.volume_names.clear();
    metadata.request.config.extra_env.push((
        kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.into(),
        "[]".into(),
    ));
    let error =
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap_err();
    assert!(matches!(error, ExecutionManagerError::InvalidRequest(_)));
    assert!(error.to_string().contains("environment projection"));
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.request.config.extra_env.clear();
    let private = home.join("private-secret-root");
    std::fs::create_dir(&private).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
    metadata.request.policy.managed_secret_root = Some(private);
    let error =
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap_err();
    assert!(matches!(error, ExecutionManagerError::InvalidRequest(_)));
    assert!(
        error.to_string().contains("memory-backed ram://"),
        "{error}"
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_memory_secret_omission_preserves_spec_bytes_and_disk_refuses() {
    let mut spec = oci_spec::runtime::Spec::default();
    let before = serde_json::to_vec(&spec).unwrap();
    crate::vm::oci_hvf_spec::complete_memory_secret_files(&mut spec, None).unwrap();
    assert_eq!(serde_json::to_vec(&spec).unwrap(), before);
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    assert!(crate::vm::oci_hvf_spec::complete_memory_secret_files(&mut spec, Some(&root)).is_err());
    assert_eq!(serde_json::to_vec(&spec).unwrap(), before);
}
