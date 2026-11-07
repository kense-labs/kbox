use super::*;
#[test]
fn test_reap_removes_box_dir() {
    // A box dir with no live shim / mount (e.g. left by a crash) is removed.
    let home = tempfile::tempdir().unwrap();
    let box_id = "reap-test-no-such-shim-uuid";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(box_dir.join("logs")).unwrap();
    std::fs::write(box_dir.join("logs/shim.stdout.log"), b"x").unwrap();
    assert!(box_dir.exists());

    reap_orphaned_box_in(home.path(), box_id);
    assert!(!box_dir.exists(), "orphaned box dir should be removed");
}

#[test]
fn test_reap_retains_box_dir_when_host_netdevice_teardown_fails() {
    // Corrupt lease must fail closed: wipe would drop durable claim while host
    // publish/NAT fabric may still exist.
    let home = tempfile::tempdir().unwrap();
    let box_id = "44444444-4444-4444-8444-444444444444";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(box_dir.join("sandbox")).unwrap();
    std::fs::write(
        box_dir.join("sandbox/host-netdevice.json"),
        b"{not-valid-host-netdevice-lease",
    )
    .unwrap();

    reap_orphaned_box_in(home.path(), box_id);

    assert!(
        box_dir.exists(),
        "orphaned box dir must be retained when host-netdevice teardown fails"
    );
    assert!(
        box_dir.join("sandbox/host-netdevice.json").exists(),
        "corrupt lease claim must remain for a later fail-closed retry"
    );
}

#[test]
fn test_reap_retains_box_dir_when_file_mount_staging_remove_fails() {
    // A non-directory staging path makes remove_dir_all fail. Wipe must not
    // invent a clean orphan reap while that host claim remains (and must not
    // wipe first, or a later pass would skip staging cleanup).
    let home = tempfile::tempdir().unwrap();
    let box_id = "55555555-5555-5555-8555-555555555555";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(&box_dir).unwrap();

    let staging = std::env::temp_dir().join(format!("kfs-mount-{box_id}"));
    std::fs::write(&staging, b"not-a-directory").unwrap();
    struct StagingGuard(std::path::PathBuf);
    impl Drop for StagingGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = StagingGuard(staging.clone());

    reap_orphaned_box_in(home.path(), box_id);

    assert!(
        box_dir.exists(),
        "orphaned box dir must be retained when file-mount staging cleanup fails"
    );
    assert!(
        staging.exists(),
        "failed staging claim must remain for a later fail-closed retry"
    );
}

#[test]
fn test_reap_retains_box_dir_when_external_socket_dir_remove_fails() {
    // A regular file at the external socket path makes remove_dir_all fail.
    // Wipe must not invent a clean orphan reap while that host claim remains.
    let home = tempfile::tempdir().unwrap();
    let box_id = "66666666-6666-6666-8666-666666666666";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(&box_dir).unwrap();

    let external = crate::vm::runtime_socket_dir(home.path(), box_id);
    if let Some(parent) = external.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&external, b"not-a-directory").unwrap();
    struct SocketGuard(std::path::PathBuf);
    impl Drop for SocketGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = SocketGuard(external.clone());

    reap_orphaned_box_in(home.path(), box_id);

    assert!(
        box_dir.exists(),
        "orphaned box dir must be retained when external socket cleanup fails"
    );
    assert!(
        external.exists(),
        "failed external socket claim must remain for a later fail-closed retry"
    );
}

#[test]
fn test_reap_absent_box_is_noop() {
    let home = tempfile::tempdir().unwrap();
    // No boxes/<id> dir at all - must not panic or error.
    reap_orphaned_box_in(home.path(), "absent-box-uuid");
}

#[test]
fn cleanup_absent_sandbox_runtime_preserves_box_directory() {
    let home = tempfile::tempdir().unwrap();
    let box_id = "cleanup-test-no-runtime-record";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(&box_dir).unwrap();

    cleanup_recorded_sandbox_runtime_in(home.path(), &box_dir, box_id).unwrap();

    assert!(box_dir.exists());
}

#[test]
fn cleanup_reaps_a_terminal_recovered_log_worker() {
    let worker = std::process::Command::new("true").spawn().unwrap();
    let pid = worker.id();
    let start_time = crate::process::pid_start_time(pid).unwrap();
    drop(worker);
    let record = RecordedSandboxRuntime {
        runtime_path: Path::new("/bin/true").to_path_buf(),
        runtime_sha256: None,
        agent_path: None,
        agent_sha256: None,
        runtime_root: Path::new("/tmp/recovered-runtime").to_path_buf(),
        runtime_socket: None,
        bundle_dir: Path::new("/tmp/recovered-bundle").to_path_buf(),
        init_pid: 42,
        generation: None,
        owner_pid: None,
        owner_pid_start_time: None,
        log_worker_pid: Some(pid),
        log_worker_pid_start_time: Some(start_time),
    };

    drain_recorded_log_worker(&record, "terminal-recovered-worker");

    assert!(
        !crate::process::is_process_alive_with_identity(pid, Some(start_time)),
        "cleanup must not leave its completed child as a zombie"
    );
}

/// Crash-recovery honesty for #373 / pool stop: a PPID-1 shim whose `--config`
/// fences under `home` must be discoverable and cleared by
/// [`reap_orphaned_boxes_for_home`] (the same helper `pool start` and
/// `pool stop` invoke).
#[test]
fn home_fenced_ppid1_shim_orphan_is_reaped() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::time::Duration;

    let home = tempfile::tempdir().unwrap();
    let box_id = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
    let box_dir = home.path().join("boxes").join(box_id);
    std::fs::create_dir_all(box_dir.join("merged")).unwrap();

    let shim = home.path().join("kbox-shim");
    // Stay as the shebang shell so /proc/cmdline keeps the script path and
    // `--config` JSON. `exec sleep` would replace argv and hide the shim fence.
    std::fs::write(&shim, "#!/bin/sh\nsleep 300\n").unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();

    let config = format!(
        r#"{{"box_id":"{box_id}","rootfs":{{"path":"{}/boxes/{box_id}/merged"}}}}"#,
        home.path().display()
    );
    // Background the shim in a subshell so it is reparented to init when the
    // short-lived bash exits (PPID 1 fence).
    let status = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "({shim} --config '{config}' >/dev/null 2>&1 &) ; sleep 0.4",
            shim = shim.display(),
            config = config.replace('\'', r#"'\''"#),
        ))
        .status()
        .expect("spawn orphan shim helper");
    assert!(status.success(), "orphan shim helper failed: {status}");

    let mut discovered = Vec::new();
    for _ in 0..40 {
        discovered = discover_orphan_shim_box_ids(home.path());
        if discovered.iter().any(|id| id == box_id) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        discovered.iter().any(|id| id == box_id),
        "PPID-1 shim under home must be discoverable before reap; got {discovered:?}"
    );

    let reaped = reap_orphaned_boxes_for_home(home.path());
    assert!(reaped >= 1, "reap must claim at least the spawned orphan");
    assert!(
        !discover_orphan_shim_box_ids(home.path())
            .iter()
            .any(|id| id == box_id),
        "reaped orphan must no longer appear under home"
    );
    assert!(
        !box_dir.exists(),
        "home-fenced orphan box directory must be removed"
    );
}
