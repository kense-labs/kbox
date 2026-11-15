use super::*;
use serde_json::json;

#[tokio::test]
async fn keyed_exec_with_reconstructed_capability_sets_replays_once() {
    let temporary = tempfile::tempdir().unwrap();
    let bundle_directory = temporary.path().join("bundle");
    std::fs::create_dir(&bundle_directory).unwrap();
    let driver = Arc::new(RecordingDriver::with_control_operations());
    let service = open_service(&temporary, Arc::clone(&driver)).await;
    let create = create_request(&bundle_directory, "caps-create");
    let created = service.create(create.clone()).await.unwrap();
    let target = ContainerTarget::exact(create.id, created.generation);
    service
        .start(StartRequest {
            context: OperationContext::new(operation_id("caps-start")),
            target: target.clone(),
        })
        .await
        .unwrap();
    let mut request = exec_request(target, "caps-exec", "caps-process");
    let mut process = serde_json::to_value(&request.process).unwrap();
    process["capabilities"] = json!({"bounding":["CAP_SYS_ADMIN","CAP_CHOWN","CAP_DAC_OVERRIDE"],"effective":["CAP_SYS_ADMIN","CAP_CHOWN"],"permitted":["CAP_SYS_ADMIN","CAP_CHOWN"],"inheritable":[],"ambient":[]});
    request.process = serde_json::from_value(process.clone()).unwrap();
    let executed = service.exec(request.clone()).await.unwrap();
    for _ in 0..16 {
        request.process = serde_json::from_value(process.clone()).unwrap();
        assert_eq!(service.exec(request.clone()).await.unwrap(), executed);
    }
    drop(service);
    let reopened = open_service(&temporary, Arc::clone(&driver)).await;
    assert_eq!(reopened.exec(request.clone()).await.unwrap(), executed);
    process["args"] = json!(["/bin/false"]);
    request.process = serde_json::from_value(process).unwrap();
    assert_eq!(
        reopened.exec(request).await.unwrap_err().code,
        ErrorCode::FailedPrecondition
    );
    assert_eq!(
        driver
            .calls()
            .iter()
            .filter(|call| matches!(call, DriverCall::Exec(_)))
            .count(),
        1
    );
}
