use super::*;

#[test]
fn test_apply_changes_cmd() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["CMD /bin/bash".to_string()]);
    assert_eq!(
        config["config"]["Cmd"],
        serde_json::json!(["/bin/sh", "-c", "/bin/bash"])
    );
}

#[test]
fn test_apply_changes_entrypoint() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["ENTRYPOINT /app/start".to_string()]);
    assert_eq!(
        config["config"]["Entrypoint"],
        serde_json::json!(["/bin/sh", "-c", "/app/start"])
    );
}

#[test]
fn test_apply_changes_env() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["ENV FOO=bar".to_string()]);
    let env = config["config"]["Env"].as_array().unwrap();
    assert_eq!(env.len(), 1);
    assert_eq!(env[0], "FOO=bar");
}

#[test]
fn test_apply_changes_workdir() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["WORKDIR /app".to_string()]);
    assert_eq!(config["config"]["WorkingDir"], "/app");
}

#[test]
fn test_apply_changes_user() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["USER nobody".to_string()]);
    assert_eq!(config["config"]["User"], "nobody");
}

#[test]
fn test_apply_changes_label() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["LABEL version=1.0".to_string()]);
    assert_eq!(config["config"]["Labels"]["version"], "1.0");
}

#[test]
fn test_apply_changes_expose() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &["EXPOSE 8080".to_string()]);
    assert!(config["config"]["ExposedPorts"]["8080/tcp"].is_object());
}

#[test]
fn test_apply_changes_multiple() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(
        &mut config,
        &[
            "CMD /start".to_string(),
            "ENV APP=test".to_string(),
            "WORKDIR /opt".to_string(),
        ],
    );
    assert!(config["config"]["Cmd"].is_array());
    assert!(config["config"]["Env"].is_array());
    assert_eq!(config["config"]["WorkingDir"], "/opt");
}

#[test]
fn test_apply_changes_empty() {
    let mut config = serde_json::json!({"config": {}});
    apply_changes(&mut config, &[]);
    assert_eq!(config["config"], serde_json::json!({}));
}

#[test]
fn test_compute_diff_id() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.txt"), "world").unwrap();
    let id = compute_diff_id(dir.path()).unwrap();
    assert!(!id.is_empty());
    assert_eq!(id.len(), 64); // sha256 hex
}

#[test]
fn test_guest_metadata_overrides_host_uid_gid_and_mode_in_tar() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };
    let rootfs = tempfile::TempDir::new().unwrap();
    let file = rootfs.path().join("probe");
    std::fs::write(&file, b"payload").unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(b"probe");
    let manifest = RootfsMetadataManifest::new(vec![RootfsMetadataEntry {
        path_base64: encoded,
        kind: RootfsEntryKind::Regular,
        mode: 0o100755,
        uid: 0,
        gid: 0,
        mtime: 123,
        size: 7,
        link_target_base64: None,
    }]);
    let output = rootfs.path().join("rootfs.tar");

    create_tar_from_guest_metadata(rootfs.path(), &manifest, &output).unwrap();

    let mut archive = tar::Archive::new(std::fs::File::open(output).unwrap());
    let entry = archive.entries().unwrap().next().unwrap().unwrap();
    assert_eq!(entry.path().unwrap(), Path::new("probe"));
    assert_eq!(entry.header().mode().unwrap() & 0o7777, 0o755);
    assert_eq!(entry.header().uid().unwrap(), 0);
    assert_eq!(entry.header().gid().unwrap(), 0);
    assert_eq!(entry.header().mtime().unwrap(), 123);
}

#[test]
fn test_guest_metadata_preserves_hardlinks_without_duplicate_payloads() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };
    let rootfs = tempfile::TempDir::new().unwrap();
    std::fs::write(rootfs.path().join("busybox"), b"payload").unwrap();
    std::fs::hard_link(rootfs.path().join("busybox"), rootfs.path().join("sh")).unwrap();
    let entries = ["busybox", "sh"]
        .into_iter()
        .map(|path| RootfsMetadataEntry {
            path_base64: base64::engine::general_purpose::STANDARD.encode(path.as_bytes()),
            kind: RootfsEntryKind::Regular,
            mode: 0o100755,
            uid: 0,
            gid: 0,
            mtime: 123,
            size: 7,
            link_target_base64: None,
        })
        .collect();
    let output = rootfs.path().join("rootfs.tar");

    create_tar_from_guest_metadata(
        rootfs.path(),
        &RootfsMetadataManifest::new(entries),
        &output,
    )
    .unwrap();

    let mut archive = tar::Archive::new(std::fs::File::open(output).unwrap());
    let mut entries = archive.entries().unwrap();
    let first = entries.next().unwrap().unwrap();
    assert_eq!(first.header().entry_type(), tar::EntryType::Regular);
    drop(first);
    let second = entries.next().unwrap().unwrap();
    assert_eq!(second.header().entry_type(), tar::EntryType::Link);
    assert_eq!(second.link_name().unwrap().unwrap(), Path::new("busybox"));
}

#[test]
fn test_guest_metadata_rejects_parent_traversal() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };
    let rootfs = tempfile::TempDir::new().unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(b"../escape");
    let manifest = RootfsMetadataManifest::new(vec![RootfsMetadataEntry {
        path_base64: encoded,
        kind: RootfsEntryKind::Regular,
        mode: 0o100600,
        uid: 0,
        gid: 0,
        mtime: 0,
        size: 0,
        link_target_base64: None,
    }]);

    let error =
        create_tar_from_guest_metadata(rootfs.path(), &manifest, &rootfs.path().join("rootfs.tar"))
            .unwrap_err();
    assert!(error.to_string().contains("Unsafe rootfs metadata path"));
}

#[test]
fn test_guest_metadata_rejects_symlink_parent_without_reading_outside_rootfs() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };

    let rootfs = tempfile::TempDir::new().unwrap();
    let outside = tempfile::TempDir::new().unwrap();
    std::fs::write(outside.path().join("secret"), b"host secret").unwrap();
    let link = rootfs.path().join("escape");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    #[cfg(windows)]
    if let Err(error) = std::os::windows::fs::symlink_dir(outside.path(), &link) {
        if error.raw_os_error() == Some(1314) {
            return;
        }
        panic!("failed to create directory symlink: {error}");
    }

    let manifest = RootfsMetadataManifest::new(vec![RootfsMetadataEntry {
        path_base64: base64::engine::general_purpose::STANDARD.encode(b"escape/secret"),
        kind: RootfsEntryKind::Regular,
        mode: 0o100600,
        uid: 0,
        gid: 0,
        mtime: 0,
        size: 11,
        link_target_base64: None,
    }]);
    let output = rootfs.path().join("rootfs.tar");
    let error = create_tar_from_guest_metadata(rootfs.path(), &manifest, &output).unwrap_err();

    assert!(error.to_string().contains("Link or non-directory parent"));
    assert_eq!(
        std::fs::read(outside.path().join("secret")).unwrap(),
        b"host secret"
    );
}

#[test]
fn guest_metadata_preserves_raw_linux_symlink_target_bytes() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };

    let rootfs = tempfile::TempDir::new().unwrap();
    let link = rootfs.path().join("link");
    #[cfg(unix)]
    std::os::unix::fs::symlink("actual", &link).unwrap();
    #[cfg(windows)]
    if let Err(error) = std::os::windows::fs::symlink_file("actual", &link) {
        if error.raw_os_error() == Some(1314) {
            return;
        }
        panic!("failed to create file symlink: {error}");
    }

    let target = b"name\\with-backslash-\xff";
    let manifest = RootfsMetadataManifest::new(vec![RootfsMetadataEntry {
        path_base64: base64::engine::general_purpose::STANDARD.encode(b"link"),
        kind: RootfsEntryKind::Symlink,
        mode: 0o120777,
        uid: 0,
        gid: 0,
        mtime: 0,
        size: 0,
        link_target_base64: Some(base64::engine::general_purpose::STANDARD.encode(target)),
    }]);
    let output = rootfs.path().join("rootfs.tar");

    create_tar_from_guest_metadata(rootfs.path(), &manifest, &output).unwrap();

    let mut archive = tar::Archive::new(std::fs::File::open(output).unwrap());
    let entry = archive.entries().unwrap().next().unwrap().unwrap();
    let actual = entry
        .link_name_bytes()
        .expect("symlink archive entry should contain a link target");
    assert_eq!(actual.as_ref(), target);
}

#[test]
fn reserved_metadata_path_is_detected_after_curdir_normalization() {
    assert!(kbox_core::rootfs_metadata::is_runtime_internal_rootfs_path(
        Path::new("./.krootfs_metadata_v1.json")
    ));
    assert!(kbox_core::rootfs_metadata::is_runtime_internal_rootfs_path(
        Path::new(".krootfs_metadata_v1.previous.json")
    ));
    assert!(kbox_core::rootfs_metadata::is_runtime_internal_rootfs_path(
        Path::new("init-rust.log")
    ));
}

#[cfg(windows)]
#[test]
fn windows_guest_paths_reject_aliases_and_reserved_names() {
    for path in [
        b"file:stream".as_slice(),
        b"CON".as_slice(),
        b"dir/name.".as_slice(),
        b"dir/name ".as_slice(),
        b"dir\\name".as_slice(),
    ] {
        assert!(guest_entry_bytes_to_host_path(path, "test path").is_err());
    }
    assert_eq!(
        windows_guest_path_key(b"Dir/Foo", "test path").unwrap(),
        windows_guest_path_key(b"dir/foo", "test path").unwrap()
    );
}

#[cfg(windows)]
#[test]
fn windows_guest_metadata_rejects_case_equivalent_duplicates() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };

    let rootfs = tempfile::TempDir::new().unwrap();
    std::fs::write(rootfs.path().join("Probe"), b"payload").unwrap();
    let entries = ["Probe", "probe"]
        .into_iter()
        .map(|path| RootfsMetadataEntry {
            path_base64: base64::engine::general_purpose::STANDARD.encode(path.as_bytes()),
            kind: RootfsEntryKind::Regular,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            mtime: 0,
            size: 7,
            link_target_base64: None,
        })
        .collect();

    let error = create_tar_from_guest_metadata(
        rootfs.path(),
        &RootfsMetadataManifest::new(entries),
        &rootfs.path().join("rootfs.tar"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("Windows-equivalent duplicate"));
}

#[cfg(windows)]
#[test]
fn windows_guest_metadata_rejects_final_directory_reparse_point() {
    use kbox_core::rootfs_metadata::{
        RootfsEntryKind, RootfsMetadataEntry, RootfsMetadataManifest,
    };

    let rootfs = tempfile::TempDir::new().unwrap();
    let outside = tempfile::TempDir::new().unwrap();
    let link = rootfs.path().join("junction");
    if let Err(error) = std::os::windows::fs::symlink_dir(outside.path(), &link) {
        if error.raw_os_error() == Some(1314) {
            return;
        }
        panic!("failed to create directory symlink: {error}");
    }
    let manifest = RootfsMetadataManifest::new(vec![RootfsMetadataEntry {
        path_base64: base64::engine::general_purpose::STANDARD.encode(b"junction"),
        kind: RootfsEntryKind::Directory,
        mode: 0o40755,
        uid: 0,
        gid: 0,
        mtime: 0,
        size: 0,
        link_target_base64: None,
    }]);

    let error =
        create_tar_from_guest_metadata(rootfs.path(), &manifest, &rootfs.path().join("rootfs.tar"))
            .unwrap_err();
    assert!(error.to_string().contains("changed type"));
}

#[test]
fn offline_commit_rejects_transitional_state_and_live_pid() {
    use crate::test_helpers::fixtures::make_record;

    let transitional = make_record("id", "box", "starting", None);
    assert!(commit_capture_mode(&transitional).is_err());

    let stopped_but_live = make_record("id", "box", "stopped", Some(std::process::id()));
    assert!(commit_capture_mode(&stopped_but_live).is_err());

    let stopped = make_record("id", "box", "stopped", None);
    assert_eq!(
        commit_capture_mode(&stopped).unwrap(),
        CommitCaptureMode::OfflineDirectory
    );
}

#[cfg(not(windows))]
#[test]
fn running_box_selects_guest_archive_commit_after_microvm_cutover() {
    use crate::test_helpers::fixtures::make_record;
    use kbox_core::ExecutionBoundary;

    let mut box_record = make_record("id", "box", "running", Some(std::process::id()));

    assert_eq!(
        commit_capture_mode(&box_record).unwrap(),
        CommitCaptureMode::LiveGuest
    );
}

#[cfg(not(windows))]
#[test]
fn paused_box_commit_fails_closed_after_microvm_cutover() {
    use crate::test_helpers::fixtures::make_record;
    use kbox_core::ExecutionBoundary;

    let mut box_record = make_record("id", "box", "paused", Some(std::process::id()));

    let error = commit_capture_mode(&box_record).unwrap_err().to_string();
    assert!(error.contains("paused MicroVM"));
}

#[cfg(not(windows))]
#[test]
fn paused_microvm_commit_fails_closed() {
    use crate::test_helpers::fixtures::make_record;
    use kbox_core::ExecutionBoundary;

    let microvm = make_record("id", "vm", "paused", Some(std::process::id()));

    let error = commit_capture_mode(&microvm).unwrap_err().to_string();
    assert!(error.contains("paused MicroVM"));
}

#[cfg(all(unix, target_os = "linux"))]
#[test]
fn stopped_managed_box_selects_offline_directory_after_microvm_cutover() {
    use crate::test_helpers::fixtures::make_record;
    use kbox_core::{
        BoxConfig, CreateExecutionRequest, ExecutionBoundary, ExecutionGeneration, OperationId,
    };
    use kbox_runtime::ManagedExecutionMetadata;
    use std::collections::BTreeMap;

    let id = "11111111-1111-4111-8111-111111111111";
    let mut box_record = make_record(id, "box", "stopped", None);
    box_record.exec_socket_path = std::path::PathBuf::new();
    box_record.managed_execution = Some(
        ManagedExecutionMetadata::new(
            OperationId::new("operation-create").unwrap(),
            ExecutionGeneration::INITIAL,
            CreateExecutionRequest {
                external_sandbox_id: "external-1".to_string(),
                config: BoxConfig {
                    image: box_record.image.clone(),
                    ..Default::default()
                },
                labels: BTreeMap::new(),
                policy: Default::default(),
                rootfs_snapshot_id: None,
            },
        )
        .unwrap(),
    );

    assert_eq!(
        commit_capture_mode(&box_record).unwrap(),
        CommitCaptureMode::OfflineDirectory
    );
}

#[cfg(unix)]
fn sandbox_commit_record(status: &str) -> crate::state::BoxRecord {
    use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionGeneration, OperationId};

    let id = "33333333-3333-4333-8333-333333333333";
    // Sandbox records carry no host PID.
    let mut record = crate::test_helpers::fixtures::make_record(id, "sbx", status, None);
    let mut metadata = kbox_runtime::ManagedExecutionMetadata::new(
        OperationId::new("commit-route-create").unwrap(),
        ExecutionGeneration::INITIAL,
        CreateExecutionRequest {
            external_sandbox_id: "commit-route-external".to_string(),
            config: BoxConfig {
                image: record.image.clone(),
                ..Default::default()
            },
            labels: std::collections::BTreeMap::new(),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        },
    )
    .unwrap();
    metadata.runtime_route = kbox_runtime::ManagedRuntimeRoute::Sandbox;
    record.managed_execution = Some(metadata);
    record
}

#[cfg(unix)]
#[test]
fn running_sandbox_commit_skips_the_host_pid_gate() {
    let record = sandbox_commit_record("running");
    assert!(record.pid.is_none());

    assert_eq!(
        commit_capture_mode(&record).unwrap(),
        CommitCaptureMode::LiveSandbox
    );
}

#[cfg(unix)]
#[test]
fn running_microvm_without_host_pid_still_fails_commit() {
    let microvm = crate::test_helpers::fixtures::make_record("id", "vm", "running", None);

    let error = commit_capture_mode(&microvm).unwrap_err().to_string();
    assert!(error.contains("host process is not live"), "{error}");
}

#[cfg(unix)]
#[tokio::test]
async fn running_sandbox_commit_capture_reports_engine_failure_not_pid_failure() {
    let home = tempfile::tempdir().unwrap();
    let mut record = sandbox_commit_record("running");
    record.box_dir = home.path().join("boxes").join(&record.id);
    std::fs::create_dir_all(&record.box_dir).unwrap();
    std::fs::write(record.box_dir.join("sandbox.container_id"), "cafebabe").unwrap();
    let mut sink = tokio::io::sink();

    let error = super::super::rootfs_capture::archive_live_sandbox_rootfs_in(
        home.path(),
        &record,
        &mut sink,
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(!error.contains("host process is not live"), "{error}");
    assert!(error.contains("daemon unreachable"), "{error}");
}

#[cfg(not(windows))]
#[test]
fn running_microvm_selects_guest_archive_commit() {
    use crate::test_helpers::fixtures::make_record;
    use kbox_core::ExecutionBoundary;

    let microvm = make_record("id", "vm", "running", Some(std::process::id()));

    assert_eq!(
        commit_capture_mode(&microvm).unwrap(),
        CommitCaptureMode::LiveGuest
    );
}

#[test]
fn stopped_guest_native_commit_selects_maintenance_capture() {
    use crate::test_helpers::fixtures::make_record;

    let temporary = tempfile::tempdir().unwrap();
    let mut stopped = make_record("id", "box", "stopped", None);
    stopped.box_dir = temporary.path().join("box");
    std::fs::create_dir_all(stopped.box_dir.join("rootfs-ext4-v1")).unwrap();

    assert_eq!(
        commit_capture_mode(&stopped).unwrap(),
        CommitCaptureMode::OfflineGuestNative
    );
}

#[test]
fn test_build_oci_image() {
    let rootfs = tempfile::tempdir().unwrap();
    std::fs::write(rootfs.path().join("test.txt"), "data").unwrap();

    let output = tempfile::tempdir().unwrap();
    build_oci_image(
        output.path(),
        rootfs.path(),
        "test:latest",
        &Some("test commit".to_string()),
        &Some("tester".to_string()),
        &[],
    )
    .unwrap();

    // Verify OCI layout
    assert!(output.path().join("oci-layout").exists());
    assert!(output.path().join("index.json").exists());
    assert!(output.path().join("blobs/sha256").exists());

    // Verify index.json is valid
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(output.path().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(index["schemaVersion"], 2);
    assert!(index["manifests"][0]["digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}
