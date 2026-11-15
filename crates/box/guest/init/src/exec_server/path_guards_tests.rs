//! Requested path restrictions must take effect before a command can launch.
use super::*;

#[test]
fn path_guard_mount_failure_refuses_exec_preparation() {
    let mut failures = Vec::new();
    for kind in ["MASKED", "READONLY"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "exec_server::path_guards_tests::path_guard_exec_without_mount_authority_child",
                "--nocapture",
            ])
            .env("K_BOX_TEST_PATH_GUARD_KIND", kind)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            failures.push(format!(
                "{kind}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "invoked by parent in an isolated process without mount authority"]
fn path_guard_exec_without_mount_authority_child() {
    crate::namespace::drop_capabilities(&["SYS_ADMIN".into()]).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("restricted"), b"private-canary").unwrap();
    let kind = std::env::var("K_BOX_TEST_PATH_GUARD_KIND").unwrap();
    let command = vec![
        "/bin/sh".into(),
        "-c".into(),
        "printf forbidden > /workload-marker".into(),
    ];
    let environment = vec![format!("K_SEC_{kind}_PATHS=/restricted")];
    let result = build_command(
        ExecCommandSpec {
            cmd: &command,
            timeout_ns: 0,
            env: &environment,
            working_dir: None,
            rootfs: Some(root.path().to_str().unwrap()),
            stdin_data: None,
            stdin_streaming: false,
            user: None,
        },
        None,
    );
    assert!(
        result.is_err(),
        "{kind} command admitted after failed path enforcement"
    );
    assert!(String::from_utf8_lossy(&result.unwrap_err().stderr).contains("path restriction"));
    assert_eq!(
        std::fs::read(root.path().join("restricted")).unwrap(),
        b"private-canary"
    );
    assert!(!root.path().join("workload-marker").exists());
}

#[test]
#[ignore = "requires mount authority in a disposable owned Linux Guest"]
fn path_guard_masks_preexisting_unrestricted_mount_instead_of_skipping_it() {
    use nix::mount::{mount, umount2, MntFlags, MsFlags};
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("restricted");
    std::fs::create_dir(&target).unwrap();
    mount(
        Some("tmpfs"),
        &target,
        Some("tmpfs"),
        MsFlags::empty(),
        Some("size=1048576"),
    )
    .unwrap();
    std::fs::write(target.join("canary"), b"must-be-masked").unwrap();
    apply_container_path_restrictions(root.path().to_str().unwrap(), &["/restricted"], &[])
        .unwrap();
    assert_eq!(
        std::fs::read_dir(&target).unwrap().count(),
        0,
        "an unrelated mount was mistaken for an enforced mask"
    );
    assert_eq!(
        std::fs::write(target.join("forbidden"), b"no")
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EROFS)
    );
    umount2(&target, MntFlags::MNT_DETACH).unwrap();
    assert_eq!(
        std::fs::read(target.join("canary")).unwrap(),
        b"must-be-masked"
    );
    umount2(&target, MntFlags::MNT_DETACH).unwrap();
}

#[test]
fn path_guard_invalid_or_missing_paths_preserve_sources() {
    let root = tempfile::tempdir().unwrap();
    let canary = root.path().join("canary");
    std::fs::write(&canary, b"retain").unwrap();
    for invalid in ["relative", "/", "////", "/.", "/a/../b", "/a\0b"] {
        for masked in [true, false] {
            let paths = ["/canary", invalid];
            let result = if masked {
                apply_container_path_restrictions(root.path().to_str().unwrap(), &paths, &[])
            } else {
                apply_container_path_restrictions(root.path().to_str().unwrap(), &[], &paths)
            };
            assert_eq!(
                result.unwrap_err().kind(),
                std::io::ErrorKind::InvalidInput,
                "{invalid:?}"
            );
            assert_eq!(std::fs::read(&canary).unwrap(), b"retain");
        }
    }
    assert_eq!(
        apply_container_path_restrictions(
            root.path().to_str().unwrap(),
            &vec!["/absent"; 129],
            &[]
        )
        .unwrap_err()
        .kind(),
        std::io::ErrorKind::InvalidInput
    );
    apply_container_path_restrictions(
        root.path().to_str().unwrap(),
        &["/missing/child"],
        &["/also-missing"],
    )
    .unwrap();
    assert!(apply_container_path_restrictions(
        root.path().to_str().unwrap(),
        &["/canary/child"],
        &[]
    )
    .is_err());
    assert_eq!(std::fs::read(&canary).unwrap(), b"retain");
}

#[test]
fn path_guard_symlink_components_refuse_without_touching_other_roots() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("canary"), b"outside-retain").unwrap();
    symlink(outside.path(), root.path().join("parent-alias")).unwrap();
    symlink(
        outside.path().join("canary"),
        root.path().join("file-alias"),
    )
    .unwrap();
    let before = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    for path in ["/parent-alias/canary", "/file-alias"] {
        assert!(
            apply_container_path_restrictions(root.path().to_str().unwrap(), &[path], &[]).is_err()
        );
        assert!(
            apply_container_path_restrictions(root.path().to_str().unwrap(), &[], &[path]).is_err()
        );
        assert_eq!(
            std::fs::read(outside.path().join("canary")).unwrap(),
            b"outside-retain"
        );
    }
    assert_eq!(
        std::fs::read_to_string("/proc/self/mountinfo").unwrap(),
        before
    );
}

#[test]
#[ignore = "requires mount authority in a disposable owned Linux Guest"]
fn path_guard_masks_files_and_directories_idempotently_without_fd_leaks() {
    use nix::mount::{umount2, MntFlags};
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    let dir = root.path().join("directory");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(&file, b"private-file").unwrap();
    std::fs::write(dir.join("canary"), b"private-directory").unwrap();
    std::fs::write(root.path().join("sibling"), b"retained-sibling").unwrap();
    let mut old = std::fs::File::open(&file).unwrap();
    let initial_fds = std::fs::read_dir("/proc/self/fd").unwrap().count();
    let stages = || {
        std::fs::read_dir("/dev/shm")
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|n| n.to_string_lossy().starts_with(".kbox-mask-"))
            .collect::<std::collections::BTreeSet<_>>()
    };
    let initial_stages = stages();
    let apply = |phase| {
        let result = apply_container_path_restrictions(
            root.path().to_str().unwrap(),
            &["/file", "/directory"],
            &[],
        );
        assert!(
            result.is_ok(),
            "{phase}: {result:?}; mounts: {}",
            std::fs::read_to_string("/proc/self/mountinfo").unwrap()
        );
    };
    apply("initial");
    assert!(file.metadata().unwrap().is_file());
    assert_eq!(file.metadata().unwrap().len(), 0);
    assert_eq!(file.metadata().unwrap().mode() & 0o777, 0);
    assert_eq!(std::fs::read(&file).unwrap(), b"");
    assert_eq!(
        std::fs::write(&file, b"forbidden")
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EROFS)
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    assert_eq!(
        std::fs::write(dir.join("forbidden"), b"no")
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EROFS)
    );
    let before = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    for _ in 0..32 {
        apply("repeat");
    }
    assert_eq!(
        std::fs::read_to_string("/proc/self/mountinfo").unwrap(),
        before
    );
    assert_eq!(
        std::fs::read_dir("/proc/self/fd").unwrap().count(),
        initial_fds
    );
    assert_eq!(stages(), initial_stages);
    let mut retained = Vec::new();
    old.read_to_end(&mut retained).unwrap();
    assert_eq!(retained, b"private-file");
    assert_eq!(
        std::fs::read(root.path().join("sibling")).unwrap(),
        b"retained-sibling"
    );
    umount2(&file, MntFlags::MNT_DETACH).unwrap();
    umount2(&dir, MntFlags::MNT_DETACH).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"private-file");
    assert_eq!(
        std::fs::read(dir.join("canary")).unwrap(),
        b"private-directory"
    );
}

#[test]
#[ignore = "requires mount authority in a disposable owned Linux Guest"]
fn path_guard_readonly_files_directories_and_nested_mounts_are_idempotent() {
    use nix::mount::{mount, umount2, MntFlags, MsFlags};
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    let dir = root.path().join("directory");
    let inner = dir.join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    std::fs::write(&file, b"retained-file").unwrap();
    std::fs::write(dir.join("seed"), b"retained-directory").unwrap();
    mount(
        Some("tmpfs"),
        &inner,
        Some("tmpfs"),
        MsFlags::empty(),
        Some("size=1048576"),
    )
    .unwrap();
    std::fs::write(inner.join("seed"), b"retained-inner").unwrap();
    let apply = || {
        apply_container_path_restrictions(
            root.path().to_str().unwrap(),
            &[],
            &["/file", "/directory"],
        )
        .unwrap()
    };
    apply();
    for path in [
        &file,
        &dir.join("seed"),
        &inner.join("seed"),
        &inner.join("new"),
    ] {
        assert_eq!(
            std::fs::write(path, b"forbidden")
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EROFS),
            "{}",
            path.display()
        );
    }
    assert_eq!(
        std::fs::remove_file(&file).unwrap_err().raw_os_error(),
        Some(libc::EBUSY)
    );
    assert_eq!(
        std::fs::rename(dir.join("seed"), dir.join("renamed"))
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EROFS)
    );
    std::fs::write(root.path().join("sibling"), b"allowed").unwrap();
    let before = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    let fds = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..32 {
        apply();
    }
    assert_eq!(
        std::fs::read_to_string("/proc/self/mountinfo").unwrap(),
        before
    );
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), fds);
    umount2(&file, MntFlags::MNT_DETACH).unwrap();
    umount2(&dir, MntFlags::MNT_DETACH).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"retained-file");
    assert_eq!(
        std::fs::read(dir.join("seed")).unwrap(),
        b"retained-directory"
    );
    assert_eq!(
        std::fs::read(inner.join("seed")).unwrap(),
        b"retained-inner"
    );
    umount2(&inner, MntFlags::MNT_DETACH).unwrap();
}

#[test]
#[ignore = "requires mount authority in a disposable owned Linux Guest"]
fn path_guard_does_not_trust_caller_readonly_tmpfs_with_writable_alias() {
    use nix::mount::{mount, umount2, MntFlags, MsFlags};
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("masked");
    let alias = root.path().join("writable-alias");
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&alias).unwrap();
    mount(
        Some("caller-tmpfs"),
        &target,
        Some("tmpfs"),
        MsFlags::empty(),
        Some("size=1048576"),
    )
    .unwrap();
    mount(
        Some(&target),
        &alias,
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )
    .unwrap();
    mount(
        None::<&str>,
        &target,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
        None::<&str>,
    )
    .unwrap();
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
    assert!(nix::sys::statvfs::statvfs(&target)
        .unwrap()
        .flags()
        .contains(nix::sys::statvfs::FsFlags::ST_RDONLY));
    apply_container_path_restrictions(root.path().to_str().unwrap(), &["/masked"], &[]).unwrap();
    std::fs::write(alias.join("late-secret"), b"must-stay-hidden").unwrap();
    assert!(
        !target.join("late-secret").exists(),
        "caller readonly tmpfs alias bypassed the mask"
    );
    umount2(&target, MntFlags::MNT_DETACH).unwrap();
    assert_eq!(
        std::fs::read(target.join("late-secret")).unwrap(),
        b"must-stay-hidden"
    );
    umount2(&target, MntFlags::MNT_DETACH).unwrap();
    umount2(&alias, MntFlags::MNT_DETACH).unwrap();
}
