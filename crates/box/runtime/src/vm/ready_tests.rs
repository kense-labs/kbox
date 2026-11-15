use super::*;
use kbox_core::config::BoxConfig;
use kbox_core::event::EventEmitter;

#[test]
fn test_parse_exec_ready_timeout_ms() {
    assert_eq!(
        parse_exec_ready_timeout_ms(None),
        DEFAULT_EXEC_READY_TIMEOUT_MS
    );
    assert_eq!(
        parse_exec_ready_timeout_ms(Some("0")),
        DEFAULT_EXEC_READY_TIMEOUT_MS
    );
    assert_eq!(
        parse_exec_ready_timeout_ms(Some("not-a-number")),
        DEFAULT_EXEC_READY_TIMEOUT_MS
    );
    assert_eq!(parse_exec_ready_timeout_ms(Some("2500")), 2500);
}

#[tokio::test]
async fn test_set_boot_completion_state_without_exec_client() {
    let vm = VmManager::with_box_id(
        BoxConfig::default(),
        EventEmitter::new(16),
        "box-boot-completion".to_string(),
    );
    let ready = vm.set_boot_completion_state().await;
    assert!(
        !ready,
        "boot completion must not invent Ready without retained exec heartbeat (#425 / #414)"
    );
    assert_eq!(vm.state().await, BoxState::Created);
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn heartbeat_within_the_total_boot_deadline_can_publish_ready() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("exec.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 5];
        stream.read_exact(&mut request).await.unwrap();
        stream.write_all(&request).await.unwrap();
    });
    let mut vm = VmManager::with_box_id(
        BoxConfig::default(),
        EventEmitter::new(16),
        "ready-heartbeat".to_string(),
    );
    vm.home_dir = root.path().to_path_buf();

    vm.wait_for_exec_ready_with_timeout(&socket, 40)
        .await
        .unwrap();

    assert!(vm.exec_client.is_some());
    assert!(vm.set_boot_completion_state().await);
    assert_eq!(vm.state().await, BoxState::Ready);
    server.await.unwrap();
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn heartbeat_after_the_total_boot_deadline_cannot_publish_ready() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("exec.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 5];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(
            request,
            ktransport::Frame::heartbeat().encode().unwrap()[..]
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
        let _ = stream.write_all(&request).await;
    });
    let mut vm = VmManager::with_box_id(
        BoxConfig::default(),
        EventEmitter::new(16),
        "late-heartbeat".to_string(),
    );
    vm.home_dir = root.path().to_path_buf();

    let result = vm.wait_for_exec_ready_with_timeout(&socket, 40).await;

    assert!(
        matches!(result, Err(BoxError::BoxBootError { .. })),
        "a heartbeat after the total deadline must be rejected: {result:?}"
    );
    assert!(vm.exec_client.is_none());
    assert!(!vm.set_boot_completion_state().await);
    assert_eq!(vm.state().await, BoxState::Created);
    server.abort();
    let _ = server.await;
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn wedged_heartbeat_attempt_cannot_outlive_the_total_boot_deadline() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("exec.sock");
    let _listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let mut vm = VmManager::with_box_id(
        BoxConfig::default(),
        EventEmitter::new(16),
        "wedged-heartbeat".to_string(),
    );
    vm.home_dir = root.path().to_path_buf();

    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        vm.wait_for_exec_ready_with_timeout(&socket, 40),
    )
    .await
    .expect("the total40ms deadline must bound the500ms heartbeat attempt");

    assert!(matches!(result, Err(BoxError::BoxBootError { .. })));
    assert!(vm.exec_client.is_none());
}
