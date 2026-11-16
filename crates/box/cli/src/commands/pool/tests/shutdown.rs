use super::*;

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
            BoundPoolListener::bind(&socket).unwrap(),
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
