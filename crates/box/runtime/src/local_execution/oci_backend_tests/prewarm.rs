use super::*;

#[tokio::test]
async fn claim_preserves_the_exact_oci_provider_binding_and_uses_the_new_box_generation() {
    let directory = tempfile::tempdir().unwrap();
    let service = Arc::new(FakeRuntimeService::launch_ready());
    let provider = Arc::new(FakeBundleProvider::default());
    let manager = manager(&directory, test_endpoint(), service.clone(), provider);
    let mut template = request("pool");
    template.config.image = format!("base@sha256:{}", "1".repeat(64));
    template.config.entrypoint_override = Some(vec!["/bin/sleep".into()]);
    template.config.cmd = vec!["infinity".into()];
    template.config.extra_env.clear();
    let reserved = manager
        .ensure_prewarmed_reservation(template.clone(), &box_operation("warm"), "engine", "owner")
        .await
        .unwrap();
    manager
        .start(&reserved.execution_id, reserved.generation)
        .await
        .unwrap();
    let before = persisted(&manager, &reserved.execution_id)
        .managed_execution
        .unwrap()
        .oci_runtime
        .unwrap();
    template.external_sandbox_id = "actual-run".into();
    let claimed = manager
        .claim_prewarmed(template, &box_operation("run"), "engine")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.execution_id, reserved.execution_id);
    assert_eq!(claimed.generation.get(), reserved.generation.get() + 1);
    let after = persisted(&manager, &claimed.execution_id)
        .managed_execution
        .unwrap()
        .oci_runtime;
    assert_eq!(after.as_ref(), Some(&before));
    assert!(manager
        .execute(
            &reserved.execution_id,
            reserved.generation,
            box_exec_request(Some("stale"))
        )
        .await
        .is_err());
    assert!(service.exec_requests.lock().unwrap().is_empty());
    manager
        .execute(
            &claimed.execution_id,
            claimed.generation,
            box_exec_request(Some("actual")),
        )
        .await
        .unwrap();
    assert_eq!(service.create_requests().len(), 1);
    assert_eq!(service.start_requests().len(), 1);
    assert_eq!(service.exec_requests.lock().unwrap().len(), 1);
    manager
        .kill(&claimed.execution_id, claimed.generation)
        .await
        .unwrap();
    manager
        .remove(&claimed.execution_id, claimed.generation)
        .await
        .unwrap();
}
