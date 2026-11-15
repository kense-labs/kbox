use super::*;

#[tokio::test]
async fn kill_cannot_forget_an_unbound_runtime_after_an_interrupted_create() {
    for status in [
        ContainerState::Creating,
        ContainerState::Created,
        ContainerState::Running,
        ContainerState::Stopped,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let service = Arc::new(FakeRuntimeService::launch_ready());
        let provider = Arc::new(FakeBundleProvider::default());
        let manager = manager(
            &directory,
            test_endpoint(),
            service.clone(),
            provider.clone(),
        );
        let id = ExecutionId::new("unbound-create").unwrap();
        let record = build_managed_record(
            &directory.path().join("home"),
            &id,
            box_operation("unbound-create-operation"),
            request("unbound-create"),
            Utc::now(),
        )
        .unwrap();
        let record = manager.reserve(record).await.unwrap().into_record();
        manager
            .transition(
                &record,
                ManagedExecutionState::Created,
                ManagedExecutionState::Starting,
                RuntimeUpdate::None,
            )
            .await
            .unwrap();
        service.seed(&id, ExecutionBoundary::Microvm, status);
        assert!(matches!(
            manager.kill(&id, ExecutionGeneration::INITIAL).await,
            Err(ExecutionManagerError::Conflict { .. })
        ));
        let retained = persisted(&manager, &id);
        assert_eq!(retained.status, ManagedExecutionState::Killing.as_status());
        assert_eq!(
            retained.managed_execution.unwrap().generation,
            ExecutionGeneration::INITIAL
        );
        assert!(manager
            .remove(&id, ExecutionGeneration::INITIAL)
            .await
            .is_err());
        assert!(service.kill_requests.lock().unwrap().is_empty());
        assert!(service.delete_requests.lock().unwrap().is_empty());
        assert_eq!(provider.cleanups.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn kill_of_a_never_started_reservation_still_proves_runtime_absence() {
    let directory = tempfile::tempdir().unwrap();
    let service = Arc::new(FakeRuntimeService::launch_ready());
    let provider = Arc::new(FakeBundleProvider::default());
    let manager = manager(&directory, test_endpoint(), service, provider);
    let lease = manager
        .create(request("unstarted"), &box_operation("unstarted-create"))
        .await
        .unwrap();
    assert_eq!(
        manager
            .kill(&lease.execution_id, lease.generation)
            .await
            .unwrap(),
        KillOutcome::AlreadyStopped
    );
    assert_eq!(
        persisted(&manager, &lease.execution_id).status,
        ManagedExecutionState::Stopped.as_status()
    );
}
