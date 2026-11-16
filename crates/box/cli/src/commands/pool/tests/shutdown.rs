use super::*;

#[tokio::test]
async fn initial_bind_preserves_a_caller_regular_file() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("调用方 socket");
    std::fs::write(&path, b"caller-data-must-survive-start").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let before = std::fs::symlink_metadata(&path).unwrap();
    let bound = BoundPoolListener::bind(path.to_str().unwrap()).await;
    assert!(
        bound.is_err(),
        "pool start must refuse a caller regular file"
    );
    let after = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        (after.dev(), after.ino(), after.mode()),
        (before.dev(), before.ino(), before.mode())
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"caller-data-must-survive-start"
    );
}

#[tokio::test]
async fn initial_bind_rejects_directory_shaped_input_before_side_effects() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("caller");
    std::fs::create_dir(&directory).unwrap();
    let canary = directory.join("canary");
    std::fs::write(&canary, b"caller-directory").unwrap();
    let before = std::fs::metadata(&directory).unwrap();
    for path in [directory.join(""), directory.join(".")] {
        assert_eq!(
            BoundPoolListener::bind(path.to_str().unwrap())
                .await
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    let after = std::fs::metadata(&directory).unwrap();
    assert_eq!(
        (after.dev(), after.ino(), after.mode()),
        (before.dev(), before.ino(), before.mode())
    );
    assert_eq!(std::fs::read(&canary).unwrap(), b"caller-directory");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
}

#[tokio::test]
async fn initial_bind_preserves_links_directories_and_non_stream_sockets() {
    use std::os::unix::fs::{symlink, MetadataExt};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("caller");
    std::fs::write(&source, b"caller-source").unwrap();
    let link = root.path().join("link.sock");
    symlink(&source, &link).unwrap();
    let directory = root.path().join("directory.sock");
    std::fs::create_dir(&directory).unwrap();
    let datagram_path = root.path().join("datagram.sock");
    let _datagram = std::os::unix::net::UnixDatagram::bind(&datagram_path).unwrap();
    for path in [&link, &directory, &datagram_path] {
        let before = std::fs::symlink_metadata(path).unwrap();
        assert!(BoundPoolListener::bind(path.to_str().unwrap())
            .await
            .is_err());
        let after = std::fs::symlink_metadata(path).unwrap();
        assert_eq!(
            (after.dev(), after.ino(), after.mode()),
            (before.dev(), before.ino(), before.mode())
        );
    }
    assert_eq!(std::fs::read_link(&link).unwrap(), source);
    assert_eq!(std::fs::read(&source).unwrap(), b"caller-source");
}

#[tokio::test]
async fn initial_bind_keeps_live_listener_and_recovers_only_its_stale_endpoint() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pool.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let before = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        BoundPoolListener::bind(path.to_str().unwrap())
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    let after = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
    let client = tokio::net::UnixStream::connect(&path).await.unwrap();
    let (peer, _) = listener.accept().await.unwrap();
    drop((client, peer, listener));
    assert_eq!(
        BoundPoolListener::bind(path.to_str().unwrap())
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    // This legacy socket has no lifetime-owner receipt; only its fixture may remove it.
    let legacy = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!((legacy.dev(), legacy.ino()), (before.dev(), before.ino()));
    std::fs::remove_file(&path).unwrap();
    let first_owner = BoundPoolListener::bind(path.to_str().unwrap())
        .await
        .unwrap();
    drop(first_owner);
    let owner = BoundPoolListener::bind(path.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        BoundPoolListener::bind(path.to_str().unwrap())
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    let current = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        (current.dev(), current.ino()),
        (owner.identity.dev(), owner.identity.ino())
    );
    drop(owner);
    BoundPoolListener::bind(path.to_str().unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_initial_binds_keep_exactly_one_live_owner() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pool.sock");
    let gate = std::sync::Arc::new(tokio::sync::Barrier::new(8));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let path = path.clone();
        let gate = gate.clone();
        tasks.spawn(async move {
            gate.wait().await;
            BoundPoolListener::bind(path.to_str().unwrap()).await
        });
    }
    let mut owners = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(owner) => owners.push(owner),
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock),
        }
    }
    assert_eq!(owners.len(), 1);
    let visible = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        (visible.dev(), visible.ino()),
        (owners[0].identity.dev(), owners[0].identity.ino())
    );
    let stream = tokio::net::UnixStream::connect(&path).await.unwrap();
    owners[0].listener.accept().await.unwrap();
    drop(stream);
}

#[tokio::test]
async fn initial_bind_preserves_a_live_listener_with_a_full_accept_queue() {
    use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("busy.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    // SAFETY: listener owns a live Unix socket; listen only changes its backlog.
    assert_eq!(unsafe { libc::listen(listener.as_raw_fd(), 1) }, 0);
    let before = std::fs::symlink_metadata(&path).unwrap();
    let mut clients = Vec::new();
    let mut full = false;
    for _ in 0..16 {
        match tokio::time::timeout(
            std::time::Duration::from_millis(50),
            tokio::net::UnixStream::connect(&path),
        )
        .await
        {
            Ok(Ok(client)) => clients.push(client),
            Ok(Err(_)) | Err(_) => {
                full = true;
                break;
            }
        }
    }
    assert!(full, "fixture must actually saturate the accept queue");
    let attempted = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        BoundPoolListener::bind(path.to_str().unwrap()),
    )
    .await
    .unwrap();
    assert!(
        attempted.is_err(),
        "full live queue must not be mistaken for a stale socket"
    );
    let after = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
    assert!(!clients.is_empty());
    listener.accept().await.unwrap();
}

#[test]
fn pool_locks_refuse_fifo_without_waiting_or_modifying_it() {
    use std::os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt},
    };
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("pool.sock");
    let lock = pool_autostart_lock_path(socket.to_str().unwrap());
    let bytes = std::ffi::CString::new(lock.as_os_str().as_bytes()).unwrap();
    // SAFETY: bytes is a live NUL-terminated pathname; mkfifo retains no pointer.
    assert_eq!(unsafe { libc::mkfifo(bytes.as_ptr(), 0o600) }, 0);
    let before = std::fs::symlink_metadata(&lock).unwrap();
    assert_eq!(
        PoolSocketLock::acquire(socket.to_str().unwrap())
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let after = std::fs::symlink_metadata(&lock).unwrap();
    assert!(after.file_type().is_fifo());
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
}

#[test]
fn malformed_listener_records_are_bounded_refused_and_kept_exactly() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().unwrap();
    for (n, contents) in [
        vec![],
        vec![0; 31],
        vec![0; 32],
        vec![0; 33],
        vec![b'x'; 4096],
    ]
    .into_iter()
    .enumerate()
    {
        let socket = root.path().join(format!("{n}.sock"));
        let path = socket.with_extension("sock.listener.lock");
        std::fs::write(&path, &contents).unwrap();
        let before = std::fs::metadata(&path).unwrap();
        let error = PoolSocketLock::acquire_listener(socket.to_str().unwrap())
            .err()
            .unwrap();
        assert_eq!(
            error.kind(),
            if contents.is_empty() {
                std::io::ErrorKind::WouldBlock
            } else {
                std::io::ErrorKind::PermissionDenied
            }
        );
        assert_eq!(std::fs::read(&path).unwrap(), contents);
        let after = std::fs::metadata(&path).unwrap();
        assert_eq!(
            (after.dev(), after.ino(), after.mode()),
            (before.dev(), before.ino(), before.mode())
        );
    }
}

#[test]
fn pool_locks_refuse_aliases_shared_writers_and_replaced_identities_without_mutation() {
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("pool.sock");
    let source = root.path().join("caller");
    std::fs::write(&source, b"caller-lock-canary").unwrap();
    let source_identity = std::fs::metadata(&source).unwrap();
    for listener in [false, true] {
        let socket = socket.to_str().unwrap();
        let path = if listener {
            std::path::PathBuf::from(format!("{socket}.listener.lock"))
        } else {
            pool_autostart_lock_path(socket)
        };
        let acquire = || {
            if listener {
                PoolSocketLock::acquire_listener(socket)
            } else {
                PoolSocketLock::acquire(socket)
            }
        };
        symlink(&source, &path).unwrap();
        assert!(acquire().is_err());
        assert_eq!(std::fs::read_link(&path).unwrap(), source);
        std::fs::remove_file(&path).unwrap();
        std::fs::hard_link(&source, &path).unwrap();
        assert!(acquire().is_err());
        assert_eq!(
            std::fs::metadata(&path).unwrap().ino(),
            source_identity.ino()
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"legacy-lock-content").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(
            acquire().err().unwrap().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"legacy-lock-content");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        if listener {
            assert!(
                acquire().is_err(),
                "unrecognized listener lock content must be preserved"
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"legacy-lock-content");
            std::fs::remove_file(&path).unwrap();
        }
        let guard = acquire().unwrap();
        if !listener {
            assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o644);
            assert_eq!(std::fs::read(&path).unwrap(), b"legacy-lock-content");
        }
        let moved = path.with_extension("moved");
        std::fs::rename(&path, &moved).unwrap();
        std::fs::write(&path, b"replacement").unwrap();
        assert_eq!(
            guard.verify_identity().unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        drop(guard);
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
    }
    assert_eq!(std::fs::read(&source).unwrap(), b"caller-lock-canary");
    assert_eq!(
        std::fs::metadata(&source).unwrap().ino(),
        source_identity.ino()
    );
}

#[tokio::test]
async fn pool_endpoint_refuses_shared_nonsticky_parent_before_creating_a_lock() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    let path = root.path().join("pool.sock");
    assert_eq!(
        BoundPoolListener::bind(path.to_str().unwrap())
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(!path.exists());
    assert!(!path.with_extension("sock.listener.lock").exists());
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn stop_ack_waits_for_existing_requests_and_excludes_its_own_connection() {
    let registry = test_registry_with_lease_ttl(0);
    let request = registry.request_guard();
    let (mut client, mut server) = tokio::net::UnixStream::pair().unwrap();
    let (shutdown, mut stopped) = tokio::sync::mpsc::unbounded_channel();
    let owner = registry.clone();
    let handler = tokio::spawn(async move { handle_conn(&owner, &shutdown, &mut server).await });
    write_frame(
        &mut client,
        &serde_json::to_vec(&PoolRequest::Stop).unwrap(),
    )
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(50),
            read_frame(&mut client)
        )
        .await
        .is_err(),
        "Stop must not acknowledge while another request still owns work"
    );
    assert!(
        stopped.try_recv().is_err(),
        "daemon must stay alive before actual drain"
    );
    drop(request);
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), read_frame(&mut client))
        .await
        .unwrap()
        .unwrap();
    let response: PoolStopResponse = serde_json::from_slice(&frame).unwrap();
    assert!(response.error.is_none());
    handler.await.unwrap().unwrap();
    stopped.recv().await.unwrap();
    assert_eq!(
        registry
            .inflight_requests
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}

#[tokio::test]
async fn cancelled_lease_drain_keeps_the_entry_owner_and_permit_for_retry() {
    let registry = test_registry_with_lease_ttl(0);
    insert_test_lease(&registry, "blocked-close", 0, 0).await;
    let (vm, permit) = {
        let leases = registry.leases.lock().await;
        let leased = leases.get("blocked-close").unwrap();
        (leased.vm.clone(), leased._permit.semaphore().clone())
    };
    let identity = std::sync::Arc::downgrade(&vm);
    let guard = vm.lock_owned().await;
    let owner = registry.clone();
    let draining = tokio::spawn(async move { owner.drain_all().await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !registry.draining.load(std::sync::atomic::Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        registry.leases.lock().await.contains_key("blocked-close"),
        "entry must remain until actual finalization"
    );
    assert_eq!(permit.available_permits(), 0);
    draining.abort();
    assert!(draining.await.unwrap_err().is_cancelled());
    drop(guard);
    assert!(
        identity.upgrade().is_some(),
        "cancelling a waiter must not discard its VM owner"
    );
    assert_eq!(permit.available_permits(), 0);
    registry.drain_all().await.unwrap();
    assert!(registry.leases.lock().await.is_empty());
    assert_eq!(permit.available_permits(), 1);
}

#[tokio::test]
async fn shutdown_socket_cleanup_does_not_unlink_a_replacement_object() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("control.sock");
    let moved = root.path().join("owned-moved.sock");
    let registry = test_registry_with_lease_ttl(0);
    let socket = path.to_str().unwrap().to_owned();
    let serving = tokio::spawn(async move {
        serve(
            registry,
            &socket,
            true,
            BoundPoolListener::bind(&socket).await.unwrap(),
        )
        .await
        .map_err(|e| e.to_string())
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if tokio::net::UnixStream::connect(&path).await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    std::fs::rename(&path, &moved).unwrap();
    std::fs::write(&path, b"caller-replacement-must-survive").unwrap();
    let mut client = tokio::net::UnixStream::connect(&moved).await.unwrap();
    write_frame(
        &mut client,
        &serde_json::to_vec(&PoolRequest::Stop).unwrap(),
    )
    .await
    .unwrap();
    let response: PoolStopResponse =
        serde_json::from_slice(&read_frame(&mut client).await.unwrap()).unwrap();
    assert!(response.error.is_none());
    let result = serving.await.unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"caller-replacement-must-survive"
    );
    assert!(
        result.is_err(),
        "changed endpoint identity must be reported instead of unlinked"
    );
}

#[tokio::test]
async fn changing_a_parent_alias_cannot_redirect_listener_cleanup() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original");
    let other = root.path().join("other");
    std::fs::create_dir(&original).unwrap();
    std::fs::create_dir(&other).unwrap();
    let alias = root.path().join("alias");
    symlink(&original, &alias).unwrap();
    let caller = other.join("pool.sock");
    std::fs::write(&caller, b"caller-behind-new-alias").unwrap();
    let path = alias.join("pool.sock");
    let bound = BoundPoolListener::bind(path.to_str().unwrap())
        .await
        .unwrap();
    std::fs::remove_file(&alias).unwrap();
    symlink(&other, &alias).unwrap();
    let registry = test_registry_with_lease_ttl(0);
    let shutdown = bound.shutdown_tx.clone();
    shutdown.send(()).unwrap();
    serve(registry, path.to_str().unwrap(), true, bound)
        .await
        .unwrap();
    assert!(!original.join("pool.sock").exists());
    assert_eq!(std::fs::read(&caller).unwrap(), b"caller-behind-new-alias");
}
