//! Permission failures must refuse command preparation before any workload is spawned.
use super::*;

#[test]
fn readonly_rootfs_path_validation_preserves_file_and_missing_path() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    std::fs::write(&file, b"retain").unwrap();
    assert_eq!(
        remount_rootfs_readonly(file.to_str().unwrap())
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(
        remount_rootfs_readonly(root.path().join("absent").to_str().unwrap())
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"retain");
}

#[test]
fn readonly_rootfs_mount_failure_refuses_exec_preparation() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "exec_server::readonly_tests::readonly_rootfs_exec_without_mount_authority_child",
            "--nocapture",
        ])
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "invoked by parent after dropping mount authority in an isolated process"]
fn readonly_rootfs_exec_without_mount_authority_child() {
    crate::namespace::drop_capabilities(&["SYS_ADMIN".into()]).unwrap();
    let root = tempfile::tempdir().unwrap();
    let command = vec![
        "/bin/sh".into(),
        "-c".into(),
        "printf forbidden > /workload-marker".into(),
    ];
    let environment = vec!["K_SEC_READONLY_ROOTFS=1".into()];
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
        "explicit readonly root was admitted after failed mount enforcement"
    );
    let error = result.unwrap_err();
    assert!(String::from_utf8_lossy(&error.stderr).contains("read-only"));
    assert!(!nix::sys::statvfs::statvfs(root.path())
        .unwrap()
        .flags()
        .contains(nix::sys::statvfs::FsFlags::ST_RDONLY));
    assert!(!root.path().join("workload-marker").exists());
}

#[test]
#[ignore = "requires an explicitly granted readonly directory in an owned Linux Guest"]
fn readonly_rootfs_already_readonly_is_idempotent_without_mount_authority() {
    crate::namespace::drop_capabilities(&["SYS_ADMIN".into()]).unwrap();
    let root = std::env::var("K_BOX_TEST_READONLY_RO_DIR").unwrap();
    assert!(nix::sys::statvfs::statvfs(root.as_str())
        .unwrap()
        .flags()
        .contains(nix::sys::statvfs::FsFlags::ST_RDONLY));
    let before = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    remount_rootfs_readonly(&root).unwrap();
    remount_rootfs_readonly(&root).unwrap();
    assert_eq!(
        std::fs::read_to_string("/proc/self/mountinfo").unwrap(),
        before
    );
    assert_eq!(
        std::fs::read(std::path::Path::new(&root).join("canary")).unwrap(),
        b"readonly-source"
    );
}

#[test]
#[ignore = "requires mount authority in a disposable owned Linux Guest"]
fn readonly_rootfs_enforces_writes_preserves_inner_mounts_and_is_idempotent() {
    use nix::mount::{mount, umount2, MntFlags, MsFlags};
    let root = tempfile::tempdir().unwrap();
    let inner = root.path().join("inner");
    std::fs::create_dir(&inner).unwrap();
    std::fs::write(root.path().join("seed"), b"retain").unwrap();
    mount(
        Some("tmpfs"),
        &inner,
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        Some("size=1048576"),
    )
    .unwrap();
    remount_rootfs_readonly(root.path().to_str().unwrap()).unwrap();
    let before = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    remount_rootfs_readonly(root.path().to_str().unwrap()).unwrap();
    assert_eq!(
        std::fs::read_to_string("/proc/self/mountinfo").unwrap(),
        before
    );
    assert_eq!(
        std::fs::write(root.path().join("forbidden"), b"no")
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EROFS)
    );
    std::fs::write(inner.join("allowed"), b"writable-inner").unwrap();
    assert_eq!(
        std::fs::read(inner.join("allowed")).unwrap(),
        b"writable-inner"
    );
    assert_eq!(std::fs::read(root.path().join("seed")).unwrap(), b"retain");
    umount2(root.path(), MntFlags::MNT_DETACH).unwrap();
    umount2(&inner, MntFlags::MNT_DETACH).unwrap();
}
