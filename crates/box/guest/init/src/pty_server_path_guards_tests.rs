use super::*;
use std::os::unix::net::UnixStream;

#[test]
fn path_guard_mount_failure_refuses_pty_before_launch() {
    let mut failures = Vec::new();
    for kind in ["MASKED", "READONLY"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "pty_server::path_guards_tests::path_guard_pty_without_mount_authority_child",
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
fn path_guard_pty_without_mount_authority_child() {
    use kbox_core::pty::{parse_frame, read_frame, write_request, PtyFrame, PtyRequest};
    crate::namespace::drop_capabilities(&["SYS_ADMIN".into()]).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("restricted"), b"private-canary").unwrap();
    let kind = std::env::var("K_BOX_TEST_PATH_GUARD_KIND").unwrap();
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
            env: vec![format!("K_SEC_{kind}_PATHS=/restricted")],
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
        "refusal must arrive as a PTY Error frame: {handled:?}"
    );
    let (kind, payload) = read_frame(&mut client).unwrap().unwrap();
    assert!(
        matches!(parse_frame(kind, payload).unwrap(), PtyFrame::Error(message) if message.contains("path restriction")),
        "PTY proceeded after unenforced path restriction"
    );
    assert_eq!(
        std::fs::read(root.path().join("restricted")).unwrap(),
        b"private-canary"
    );
    assert!(!root.path().join("workload-marker").exists());
}
