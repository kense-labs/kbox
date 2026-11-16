use super::*;

fn attack_fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let box_dir = home.join("boxes/attack");
    std::fs::create_dir_all(box_dir.join("logs")).unwrap();
    std::fs::create_dir(root.path().join("foreign")).unwrap();
    std::fs::write(root.path().join("foreign/canary"), b"ungranted-canary").unwrap();
    (root, home, box_dir)
}

#[cfg(unix)]
#[test]
fn source_file_symlink_never_copies_ungranted_bytes() {
    let (root, home, box_dir) = attack_fixture();
    std::os::unix::fs::symlink(
        root.path().join("foreign/canary"),
        box_dir.join("logs/alias"),
    )
    .unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    assert!(!home.join("diag/failed-boots/attack/logs/alias").exists());
    assert_eq!(
        std::fs::read(root.path().join("foreign/canary")).unwrap(),
        b"ungranted-canary"
    );
}

#[test]
fn source_hardlink_never_copies_ungranted_bytes() {
    let (root, home, box_dir) = attack_fixture();
    std::fs::hard_link(
        root.path().join("foreign/canary"),
        box_dir.join("logs/alias"),
    )
    .unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    assert!(!home.join("diag/failed-boots/attack/logs/alias").exists());
}

#[cfg(unix)]
#[test]
fn source_directory_symlink_is_not_traversed() {
    let (root, home, box_dir) = attack_fixture();
    std::os::unix::fs::symlink(root.path().join("foreign"), box_dir.join("logs/alias")).unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    assert!(!home.join("diag/failed-boots/attack/logs/alias").exists());
}

#[cfg(unix)]
#[test]
fn source_logs_root_symlink_is_refused() {
    let (root, home, box_dir) = attack_fixture();
    std::fs::remove_dir(box_dir.join("logs")).unwrap();
    std::os::unix::fs::symlink(root.path().join("foreign"), box_dir.join("logs")).unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    assert!(!home.join("diag/failed-boots/attack").exists());
}

#[cfg(unix)]
#[test]
fn destination_ancestor_symlink_never_writes_outside_the_retained_scope() {
    let (root, home, box_dir) = attack_fixture();
    std::fs::write(box_dir.join("logs/console.log"), b"actual-diagnostic").unwrap();
    std::os::unix::fs::symlink(root.path().join("foreign"), home.join("diag")).unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    assert!(!root.path().join("foreign/failed-boots").exists());
    assert_eq!(
        std::fs::read(root.path().join("foreign/canary")).unwrap(),
        b"ungranted-canary"
    );
}

#[test]
fn each_log_is_a_bounded_tail_instead_of_an_unbounded_copy() {
    use std::io::Write;
    let (_root, home, box_dir) = attack_fixture();
    let mut input = std::fs::File::create(box_dir.join("logs/console.log")).unwrap();
    input.write_all(&vec![b'x'; 2 * 1024 * 1024]).unwrap();
    input.write_all(b"last-failure-marker").unwrap();
    drop(input);
    retain_boot_failure_logs(&home, &box_dir, "attack");
    let bytes = std::fs::read(home.join("diag/failed-boots/attack/logs/console.log")).unwrap();
    assert!(
        bytes.len() <= 1024 * 1024,
        "diagnostic bytes must have a per-file bound"
    );
    assert!(bytes.ends_with(b"last-failure-marker"));
    assert_eq!(
        std::fs::metadata(box_dir.join("logs/console.log"))
            .unwrap()
            .len(),
        2 * 1024 * 1024 + 19
    );
}

#[test]
fn entry_limit_still_keeps_the_known_console_diagnostic() {
    let (_root, home, box_dir) = attack_fixture();
    for index in 0..80 {
        std::fs::write(box_dir.join(format!("logs/input-{index}")), b"").unwrap();
    }
    std::fs::write(box_dir.join("logs/console.log"), b"known-priority-log").unwrap();
    retain_boot_failure_logs(&home, &box_dir, "attack");
    let retained = home.join("diag/failed-boots/attack/logs");
    assert!(std::fs::read_dir(&retained).unwrap().count() <= 64);
    assert_eq!(
        std::fs::read(retained.join("console.log")).unwrap(),
        b"known-priority-log"
    );
    let report: serde_json::Value = serde_json::from_slice(
        &std::fs::read(retained.parent().unwrap().join("retention.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["limitReached"], true);
}

#[test]
fn total_snapshot_bytes_are_bounded_even_with_many_large_files() {
    let (_root, home, box_dir) = attack_fixture();
    for index in 0..12 {
        std::fs::File::create(box_dir.join(format!("logs/input-{index}")))
            .unwrap()
            .set_len(2 * 1024 * 1024)
            .unwrap();
    }
    retain_boot_failure_logs(&home, &box_dir, "attack");
    let bytes: u64 = std::fs::read_dir(home.join("diag/failed-boots/attack/logs"))
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    assert!(bytes <= 8 * 1024 * 1024, "diagnostic total must be bounded");
    let report: serde_json::Value = serde_json::from_slice(
        &std::fs::read(home.join("diag/failed-boots/attack/retention.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["bytes"], bytes);
    assert_eq!(report["limitReached"], true);
}

#[test]
fn box_identity_cannot_escape_the_retention_directory() {
    let (_root, home, box_dir) = attack_fixture();
    std::fs::write(box_dir.join("logs/console.log"), b"diagnostic").unwrap();
    retain_boot_failure_logs(&home, &box_dir, "../../escaped");
    assert!(!home.join("escaped").exists());
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("boot-diag-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn copies_console_and_shim_logs_into_the_retained_slot() {
    let root = temp_root("copy");
    let home = root.join("home");
    let box_dir = home.join("boxes").join("box-1");
    std::fs::create_dir_all(box_dir.join("logs")).unwrap();
    std::fs::write(
        box_dir.join("logs/console.log"),
        "K_BOOT_ABORT: stage=event_loop",
    )
    .unwrap();
    std::fs::write(box_dir.join("logs/shim.log"), "shim ready").unwrap();

    retain_boot_failure_logs(&home, &box_dir, "box-1");

    let retained = home.join("diag/failed-boots/box-1/logs");
    assert_eq!(
        std::fs::read_to_string(retained.join("console.log")).unwrap(),
        "K_BOOT_ABORT: stage=event_loop"
    );
    assert_eq!(
        std::fs::read_to_string(retained.join("shim.log")).unwrap(),
        "shim ready"
    );
    // The working box directory is untouched; teardown owns it.
    assert!(box_dir.join("logs/console.log").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_retry_replaces_the_prior_retained_copy() {
    let root = temp_root("replace");
    let home = root.join("home");
    let box_dir = home.join("boxes").join("box-2");
    std::fs::create_dir_all(box_dir.join("logs")).unwrap();
    std::fs::write(box_dir.join("logs/console.log"), "first attempt").unwrap();
    retain_boot_failure_logs(&home, &box_dir, "box-2");

    std::fs::write(box_dir.join("logs/console.log"), "second attempt").unwrap();
    std::fs::write(box_dir.join("logs/guest-init.stderr.log"), "stderr").unwrap();
    retain_boot_failure_logs(&home, &box_dir, "box-2");

    let retained = home.join("diag/failed-boots/box-2/logs");
    assert_eq!(
        std::fs::read_to_string(retained.join("console.log")).unwrap(),
        "second attempt"
    );
    assert!(retained.join("guest-init.stderr.log").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_missing_logs_directory_is_a_logged_no_op() {
    let root = temp_root("noop");
    let home = root.join("home");
    let box_dir = home.join("boxes").join("box-3");
    std::fs::create_dir_all(&box_dir).unwrap();

    retain_boot_failure_logs(&home, &box_dir, "box-3");

    assert!(!home.join("diag/failed-boots/box-3").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn source_fifo_is_skipped_without_opening_a_writer() {
    let (_root, home, box_dir) = attack_fixture();
    assert!(std::process::Command::new("mkfifo")
        .arg(box_dir.join("logs/fifo"))
        .status()
        .unwrap()
        .success());
    let report = retain(&home, &box_dir, "attack").unwrap();
    assert_eq!(report.skipped, 1);
    assert_eq!(report.files, 0);
    assert!(!home.join("diag/failed-boots/attack/logs/fifo").exists());
}

#[cfg(unix)]
#[test]
fn pinned_directories_survive_source_and_destination_path_replacement() {
    let (root, home, box_dir) = attack_fixture();
    std::fs::write(box_dir.join("logs/console.log"), b"pinned-log").unwrap();
    let source = Dir::open_ambient_dir(box_dir.join("logs"), cap_std::ambient_authority()).unwrap();
    let target_path = home.join("target 空格");
    std::fs::create_dir(&target_path).unwrap();
    let target = Dir::open_ambient_dir(&target_path, cap_std::ambient_authority()).unwrap();
    std::fs::rename(box_dir.join("logs"), box_dir.join("pinned")).unwrap();
    std::os::unix::fs::symlink(root.path().join("foreign"), box_dir.join("logs")).unwrap();
    std::fs::rename(&target_path, home.join("target-pinned")).unwrap();
    std::os::unix::fs::symlink(root.path().join("foreign"), &target_path).unwrap();
    let report = copy_logs(&source, &target).unwrap();
    assert_eq!(report.files, 1);
    assert_eq!(
        std::fs::read(home.join("target-pinned/console.log")).unwrap(),
        b"pinned-log"
    );
    assert!(!root.path().join("foreign/console.log").exists());
    assert_eq!(
        std::fs::read(root.path().join("foreign/canary")).unwrap(),
        b"ungranted-canary"
    );
}

#[test]
fn unsafe_previous_slots_are_preserved_without_recursive_deletion() {
    for nested in [false, true] {
        let (_root, home, box_dir) = attack_fixture();
        std::fs::write(box_dir.join("logs/console.log"), b"new-log").unwrap();
        let slot = home.join("diag/failed-boots/attack/logs");
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::write(slot.join("canary"), b"old-log").unwrap();
        if nested {
            std::fs::create_dir(slot.join("nested")).unwrap();
            std::fs::write(slot.join("nested/canary"), b"nested-old-log").unwrap();
        } else {
            for index in 0..MAX_ENTRIES {
                std::fs::write(slot.join(format!("entry-{index}")), b"").unwrap();
            }
        }
        assert_eq!(
            retain(&home, &box_dir, "attack").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(std::fs::read(slot.join("canary")).unwrap(), b"old-log");
        assert!(!slot.join("console.log").exists());
    }
}

#[cfg(unix)]
#[test]
fn replacing_old_slot_aliases_never_changes_their_targets() {
    let (root, home, box_dir) = attack_fixture();
    std::fs::write(box_dir.join("logs/console.log"), b"new-log").unwrap();
    std::fs::write(box_dir.join("logs/shim.stderr.log"), b"new-error").unwrap();
    let slot = home.join("diag/failed-boots/attack/logs");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::hard_link(root.path().join("foreign/canary"), slot.join("console.log")).unwrap();
    std::os::unix::fs::symlink(
        root.path().join("foreign/canary"),
        slot.join("shim.stderr.log"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        root.path().join("foreign/canary"),
        slot.parent().unwrap().join("retention.json"),
    )
    .unwrap();
    retain(&home, &box_dir, "attack").unwrap();
    assert_eq!(std::fs::read(slot.join("console.log")).unwrap(), b"new-log");
    assert_eq!(
        std::fs::read(root.path().join("foreign/canary")).unwrap(),
        b"ungranted-canary"
    );
}

#[test]
fn report_records_bounded_tail_and_non_ascii_empty_file() {
    let (_root, home, box_dir) = attack_fixture();
    std::fs::write(box_dir.join("logs/空 文件.log"), b"").unwrap();
    std::fs::File::create(box_dir.join("logs/console.log"))
        .unwrap()
        .set_len(MAX_FILE_BYTES + 10)
        .unwrap();
    retain(&home, &box_dir, "attack").unwrap();
    let slot = home.join("diag/failed-boots/attack");
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(slot.join("retention.json")).unwrap()).unwrap();
    assert_eq!(report["files"], 2);
    assert_eq!(report["bytes"], MAX_FILE_BYTES);
    assert_eq!(report["truncated"], 1);
    assert_eq!(report["skipped"], 0);
    assert_eq!(report["limitReached"], false);
    assert_eq!(
        std::fs::metadata(slot.join("logs/空 文件.log"))
            .unwrap()
            .len(),
        0
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&slot, &slot.join("logs")] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        for path in [slot.join("logs/console.log"), slot.join("retention.json")] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn invalid_scope_has_no_retention_side_effects() {
    let (_root, home, box_dir) = attack_fixture();
    for id in [
        "",
        ".",
        "..",
        "/attack",
        "attack/",
        "attack/.",
        "attack//",
        "attack/../attack",
    ] {
        assert_eq!(
            retain(&home, &box_dir, id).unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
            "{id}"
        );
    }
    assert!(retain(&home, &box_dir, &"x".repeat(129)).is_err());
    assert!(retain(&home, &home.join("other"), "attack").is_err());
    assert!(!home.join("diag").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "requires owned sentinel home, cached image and installed Box companions"]
fn native_guest_log_aliases_are_confined_during_failed_boot_cleanup() {
    use crate::VmManager;
    use futures::FutureExt;
    use kbox_core::{event::EventEmitter, BoxConfig, NetworkMode};
    use std::path::PathBuf;

    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let home = PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(), home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-boot-diag-native")).unwrap(), "owned-boot-diag-native-test\n");
        let root = home.parent().unwrap();
        let foreign = root.join("foreign");
        std::fs::create_dir(&foreign).unwrap();
        std::fs::write(foreign.join("canary"), b"ungranted-host-canary").unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let box_dir = home.join("boxes").join(&id);
        std::fs::create_dir_all(box_dir.join("logs")).unwrap();
        let config = BoxConfig {
            image: std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
            network: NetworkMode::None,
            cmd: vec!["/bin/sh".into(), "-c".into(), "while :; do /bin/sleep 1; done".into()],
            volumes: vec![format!("{}:/attack-logs:rw", box_dir.join("logs").display())],
            ..BoxConfig::default()
        };
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let mut vm = VmManager::with_box_id(config, EventEmitter::new(16), id.clone());
            assert_eq!(vm.home_dir, home);
            let result = std::panic::AssertUnwindSafe(async {
                vm.boot().await.unwrap();
                let pid = vm.handler.read().await.as_ref().unwrap().pid();
                let socket_dir = vm.socket_dir().to_path_buf();
                std::fs::write(root.join("before-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"id":id,"pid":pid,"socketDir":socket_dir,"authenticated":vm.exec_client().is_some()})).unwrap()).unwrap();
                let script = r#"set -eu
/bin/ln -s "$1/canary" /attack-logs/file-alias
/bin/ln -s "$1" /attack-logs/directory-alias
/bin/dd if=/dev/zero of=/attack-logs/guest-flood.log bs=1048576 count=2
printf guest-tail-marker >> /attack-logs/guest-flood.log
printf guest-planted
"#;
                let output = vm.exec_command(vec!["/bin/sh".into(), "-c".into(), script.into(), "plant-logs".into(), foreign.to_str().unwrap().into()], 10_000_000_000).await.unwrap();
                assert_eq!(output.exit_code, 0, "{output:?}");
                assert_eq!(std::fs::read_link(box_dir.join("logs/file-alias")).unwrap(), foreign.join("canary"));
                assert!(std::fs::symlink_metadata(box_dir.join("logs/directory-alias")).unwrap().file_type().is_symlink());
                vm.cleanup_boot_failure().await;
                assert!(vm.handler.read().await.is_none());
                assert!(!box_dir.exists());
                assert!(!socket_dir.exists());
                let slot = home.join("diag/failed-boots").join(&id);
                let logs = slot.join("logs");
                let bytes = std::fs::read(logs.join("guest-flood.log")).unwrap();
                assert_eq!(bytes.len(), MAX_FILE_BYTES as usize);
                assert!(bytes.ends_with(b"guest-tail-marker"));
                assert!(!logs.join("file-alias").exists());
                assert!(!logs.join("directory-alias").exists());
                assert_eq!(std::fs::read(foreign.join("canary")).unwrap(), b"ungranted-host-canary");
                let report: serde_json::Value = serde_json::from_slice(&std::fs::read(slot.join("retention.json")).unwrap()).unwrap();
                assert!(report["skipped"].as_u64().unwrap() >= 2);
                assert!(report["truncated"].as_u64().unwrap() >= 1);
                assert!(report["bytes"].as_u64().unwrap() <= MAX_SNAPSHOT_BYTES);
                std::fs::write(root.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","execution":id,"pid":pid,"socketDir":socket_dir,"guestPlanted":true,"retainedTailBytes":bytes.len(),"report":report,"outsideCanaryRetained":true,"boxDirectoryAbsent":true,"socketDirectoryAbsent":true,"handlerReleased":true,"runtimeRssMeasured":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            let cleanup = vm.destroy_with_timeout(0).await;
            std::fs::write(root.join("native-cleanup.json"), serde_json::to_vec_pretty(&serde_json::json!({"destroyOk":cleanup.is_ok(),"boxDirectoryAbsent":!box_dir.exists(),"socketDirectoryAbsent":!vm.socket_dir().exists()})).unwrap()).unwrap();
            if let Err(error) = result { std::panic::resume_unwind(error); }
            cleanup.unwrap();
        });
    }).unwrap().join().unwrap();
}

#[cfg(unix)]
#[test]
fn source_and_destination_child_directory_aliases_are_refused() {
    for child in [
        "boxes",
        "boxes/attack",
        "diag/failed-boots",
        "diag/failed-boots/attack",
        "diag/failed-boots/attack/logs",
    ] {
        let (root, home, box_dir) = attack_fixture();
        std::fs::write(box_dir.join("logs/console.log"), b"real-log").unwrap();
        let path = home.join(child);
        if path.exists() {
            std::fs::rename(&path, root.path().join("saved")).unwrap();
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(root.path().join("foreign"), &path).unwrap();
        assert!(retain(&home, &box_dir, "attack").is_err(), "{child}");
        assert_eq!(
            std::fs::read(root.path().join("foreign/canary")).unwrap(),
            b"ungranted-canary"
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("foreign"))
                .unwrap()
                .count(),
            1
        );
    }
}

#[test]
fn report_collision_preserves_existing_file_and_failed_rename_removes_only_its_temporary() {
    let root = tempfile::tempdir().unwrap();
    let slot = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
    std::fs::write(root.path().join(".retention-existing"), b"existing-canary").unwrap();
    assert_eq!(
        write_report(&slot, ".retention-existing", &Snapshot::default())
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        std::fs::read(root.path().join(".retention-existing")).unwrap(),
        b"existing-canary"
    );
    std::fs::create_dir(root.path().join("retention.json")).unwrap();
    std::fs::write(
        root.path().join("retention.json/canary"),
        b"directory-canary",
    )
    .unwrap();
    assert!(write_report(&slot, ".retention-new", &Snapshot::default()).is_err());
    assert!(!root.path().join(".retention-new").exists());
    assert_eq!(
        std::fs::read(root.path().join("retention.json/canary")).unwrap(),
        b"directory-canary"
    );
}
