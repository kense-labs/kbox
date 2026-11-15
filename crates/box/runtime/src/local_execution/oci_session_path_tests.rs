use super::*;
use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionId, OperationId, SnapshotImageConfig};

fn fixture() -> (tempfile::TempDir, BoxRecord) {
    let temporary = tempfile::tempdir().unwrap();
    let record = super::super::record::build_managed_record(
        temporary.path(),
        &ExecutionId::new("00000000-0000-4000-8000-000000000024").unwrap(),
        OperationId::new("guest-exec-path").unwrap(),
        CreateExecutionRequest {
            external_sandbox_id: "guest-exec-path".into(),
            config: BoxConfig::default(),
            labels: Default::default(),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        },
        chrono::Utc::now(),
    )
    .unwrap();
    std::fs::create_dir_all(&record.box_dir).unwrap();
    crate::resolved_image::persist_snapshot_image_config(
        &record.box_dir,
        &SnapshotImageConfig {
            env: vec![("PATH".into(), "/image/bin:/bin".into())],
            ..Default::default()
        },
    )
    .unwrap();
    (temporary, record)
}

#[test]
fn hvf_guest_exec_path_uses_guest_lookup_and_effective_image_path() {
    let (_temporary, record) = fixture();
    let mut args = vec!["sh".into(), "-c".into(), "exit 0".into()];
    let mut env = vec!["CONTROL=one".into()];
    resolve_exec_executable(
        &record,
        koci_sdk::DriverKind::LibkHvf,
        &mut args,
        &mut env,
        None,
    )
    .unwrap();
    assert_eq!(args, ["sh", "-c", "exit 0"]);
    assert_eq!(env, ["CONTROL=one", "PATH=/image/bin:/bin"]);
    assert!(!record.box_dir.join("rootfs").exists());
}

#[test]
fn hvf_guest_exec_path_preserves_overrides_and_refuses_escape() {
    let (_temporary, record) = fixture();
    let mut env = vec!["PATH=/explicit/bin".into()];
    let before = env.clone();
    let mut args = vec!["sh".into()];
    resolve_exec_executable(
        &record,
        koci_sdk::DriverKind::LibkHvf,
        &mut args,
        &mut env,
        None,
    )
    .unwrap();
    assert_eq!(args, ["sh"]);
    assert_eq!(env, before);
    let mut relative = vec!["../bin/sh".into()];
    resolve_exec_executable(
        &record,
        koci_sdk::DriverKind::LibkHvf,
        &mut relative,
        &mut env,
        Some("/work"),
    )
    .unwrap();
    assert_eq!(relative, ["/bin/sh"]);
    let mut escape = vec!["../../bin/sh".into()];
    assert!(matches!(
        resolve_exec_executable(
            &record,
            koci_sdk::DriverKind::LibkHvf,
            &mut escape,
            &mut env,
            Some("/")
        ),
        Err(ExecutionManagerError::InvalidRequest(_))
    ));
    std::fs::remove_file(
        record
            .box_dir
            .join(crate::resolved_image::RESOLVED_IMAGE_CONFIG_FILE),
    )
    .unwrap();
    let mut default = vec![];
    let mut command = vec!["sh".into()];
    resolve_exec_executable(
        &record,
        koci_sdk::DriverKind::LibkHvf,
        &mut command,
        &mut default,
        None,
    )
    .unwrap();
    assert_eq!(default, [format!("PATH={DEFAULT_CONTAINER_PATH}")]);
}

#[test]
fn other_driver_exec_path_still_requires_a_host_prepared_executable() {
    let (_temporary, record) = fixture();
    for driver in [
        koci_sdk::DriverKind::NativeLinux,
        koci_sdk::DriverKind::LibkKvm,
        koci_sdk::DriverKind::LibkWhpx,
    ] {
        let mut command = vec!["sh".into()];
        let mut env = vec![];
        assert!(matches!(
            resolve_exec_executable(&record, driver, &mut command, &mut env, None),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        assert_eq!(command, ["sh"]);
        assert!(env.is_empty());
    }
}

#[test]
fn hvf_guest_exec_path_keeps_path_entry_escape_rejection_atomic() {
    let (_temporary, record) = fixture();
    for path in ["/../../bin", "../../bin", "/bin:/../../bad"] {
        let mut args = vec!["sh".into()];
        let mut env = vec![format!("PATH={path}")];
        let before = env.clone();
        assert!(matches!(
            resolve_exec_executable(
                &record,
                koci_sdk::DriverKind::LibkHvf,
                &mut args,
                &mut env,
                None
            ),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        assert_eq!(args, ["sh"]);
        assert_eq!(env, before);
    }
}

#[test]
fn hvf_legacy_materialized_exec_keeps_resolved_argv_and_environment_bytes() {
    let (_temporary, record) = fixture();
    let executable = record.box_dir.join("rootfs/bin/sh");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(&executable, b"legacy executable fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut args = vec!["sh".into(), "-c".into(), "exit 0".into()];
    let mut env = vec!["CONTROL=one".into()];
    let before = env.clone();
    resolve_exec_executable(
        &record,
        koci_sdk::DriverKind::LibkHvf,
        &mut args,
        &mut env,
        None,
    )
    .unwrap();
    assert_eq!(args, ["/bin/sh", "-c", "exit 0"]);
    assert_eq!(env, before);
}
