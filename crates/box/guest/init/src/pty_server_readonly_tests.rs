use super::*;
use std::os::unix::net::UnixStream;

#[test]
fn readonly_rootfs_mount_failure_refuses_pty_before_launch() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "pty_server::readonly_tests::readonly_rootfs_pty_without_mount_authority_child",
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
#[ignore = "invoked by parent in an isolated process without mount authority"]
fn readonly_rootfs_pty_without_mount_authority_child() {
    use kbox_core::pty::{parse_frame, read_frame, write_request, PtyFrame, PtyRequest};
    crate::namespace::drop_capabilities(&["SYS_ADMIN".into()]).unwrap();
    let root = tempfile::tempdir().unwrap();
    let (server, mut client) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    write_request(
        &mut client,
        &PtyRequest {
            cmd: vec![
                "/bin/sh".into(),
                "-c".into(),
                "printf forbidden > /workload-marker".into(),
            ],
            env: vec!["K_SEC_READONLY_ROOTFS=1".into()],
            working_dir: None,
            rootfs: Some(root.path().to_str().unwrap().into()),
            user: None,
            cols: 80,
            rows: 24,
        },
    )
    .unwrap();
    client.shutdown(std::net::Shutdown::Write).unwrap();
    let handled = handle_pty_connection(server.into());
    assert!(
        handled.is_ok(),
        "refusal must reach the caller as a PTY error frame: {handled:?}"
    );
    let (kind, payload) = read_frame(&mut client).unwrap().unwrap();
    assert!(
        matches!(parse_frame(kind, payload).unwrap(), PtyFrame::Error(message) if message.contains("read-only")),
        "PTY proceeded after unenforced readonly root"
    );
    assert!(!root.path().join("workload-marker").exists());
    assert!(!nix::sys::statvfs::statvfs(root.path())
        .unwrap()
        .flags()
        .contains(nix::sys::statvfs::FsFlags::ST_RDONLY));
}
