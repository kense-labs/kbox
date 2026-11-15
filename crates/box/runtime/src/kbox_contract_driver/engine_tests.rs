use std::sync::Arc;
use std::time::Duration;

use kbox_contract::contract::{RuntimeInspection, RuntimeUnitClass, RuntimeUnitState};
use tokio::net::UnixStream;
use tokio::sync::oneshot;

use crate::engine::{
    serve_engine, DesiredState, EngineCommand, EngineErrorCode, EngineIntent, EngineJournal,
    EngineKey, EngineReply, EngineReplyBody, EngineReplyValue, EngineRequest,
    EngineServiceTemplate, LocalEngine,
};
use crate::pool::client::{read_frame, write_frame, MAX_FRAME_SIZE};

use super::test_support::{fake_driver, fake_driver_with_backend, fixture_tempdir, runtime_spec};

#[path = "engine_tests/primary.rs"]
mod primary;

fn request(command: EngineCommand) -> EngineRequest {
    EngineRequest {
        schema: "kbox.engine.request.v1".into(),
        command,
    }
}

async fn discover(engine: &LocalEngine) -> EngineKey {
    let EngineReplyValue::Discovery(result) = engine
        .dispatch(request(EngineCommand::Discover))
        .await
        .unwrap()
    else {
        panic!("expected discovery")
    };
    result.engine_id
}

#[tokio::test]
async fn engine_diagnostics_do_not_submit_work_or_modify_the_journal() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let directory = root.path().join("home/engine");
    let engine = LocalEngine::with_runtime(directory.clone(), Arc::new(driver)).unwrap();
    let EngineReplyValue::Discovery(discovery) = engine
        .dispatch(request(EngineCommand::Discover))
        .await
        .unwrap()
    else {
        panic!("expected discovery")
    };
    assert_eq!(
        discovery.protocols,
        [
            "kbox.engine.v2",
            "engine-diagnostics.v1",
            "process-stream.primary.v1",
            "process-stream.bootstrap.v1",
        ]
    );
    let before = std::fs::read(directory.join("intent.json")).unwrap();
    let EngineReplyValue::Diagnostics(diagnostics) = engine
        .dispatch(request(EngineCommand::Diagnose {}))
        .await
        .unwrap()
    else {
        panic!("expected diagnostics")
    };
    assert_eq!(diagnostics.schema, "kbox.engine.diagnostics.v1");
    assert_eq!(diagnostics.engine_id, discovery.engine_id);
    assert_eq!(diagnostics.version, kbox_core::VERSION);
    assert_eq!(diagnostics.platform, discovery.platform);
    assert_eq!(
        std::fs::read(directory.join("intent.json")).unwrap(),
        before
    );
    assert_eq!(backend.starts(), 0);
    assert_eq!(backend.kills(), 0);
    assert!(serde_json::from_value::<EngineRequest>(serde_json::json!({
        "schema":"kbox.engine.request.v1", "command":{"op":"diagnose", "image":"must-not-launch"}
    }))
    .is_err());
}

#[tokio::test]
async fn engine_down_observation_waits_for_owned_stopped_spare_removal() {
    use kbox_contract::contract::RestartPolicy;
    use kbox_core::ExecutionManager;

    let root = fixture_tempdir();
    let (driver, _) = fake_driver(&root);
    let driver = Arc::new(driver.with_environment_prewarm("fixture-engine").unwrap());
    let directory = root.path().join("home/engine");
    let mut journal = EngineJournal::open(directory.join("intent.json")).unwrap();
    let id = journal.engine_id().clone();
    let mut up = intent(&id, 1, DesiredState::Up);
    up.prewarm_bindings_digest = Some(format!("sha256:{}", "3".repeat(64)));
    let mut spec = runtime_spec("spare", 1, RuntimeUnitClass::Service);
    spec.semantics_profile_digest = Some(super::NATIVE_GUEST_PROFILE_DIGEST.into());
    spec.process.command = vec!["/bin/sleep".into()];
    spec.process.args = vec!["infinity".into()];
    spec.process.environment.clear();
    spec.health = None;
    spec.restart = RestartPolicy::Never;
    let service = up.service.as_mut().unwrap();
    service.process = spec.process.clone();
    service.semantics_profile_digest = spec.semantics_profile_digest.clone();
    service.health = None;
    service.restart = RestartPolicy::Never;
    journal.accept(up.clone()).unwrap();
    let spare = driver
        .prepare_environment(&spec, up.instance_id.as_str())
        .await
        .unwrap();
    let unrelated = driver
        .prepare_environment(&spec, "other-instance")
        .await
        .unwrap();
    driver
        .manager
        .kill(&spare.execution_id, spare.generation)
        .await
        .unwrap();
    let down = intent(&id, 2, DesiredState::Down);
    journal.accept(down.clone()).unwrap();
    drop(journal);
    let engine = LocalEngine::with_runtime(directory, driver.clone()).unwrap();
    let inspect = || {
        request(EngineCommand::InspectExecution {
            instance_id: down.instance_id.clone(),
        })
    };
    let EngineReplyValue::Execution(pending) = engine.dispatch(inspect()).await.unwrap() else {
        panic!("expected execution observation")
    };
    assert!(matches!(
        pending.runtime,
        Some(RuntimeInspection::NotFound { .. })
    ));
    assert_eq!(pending.last_error, Some(EngineErrorCode::CleanupPending));
    driver
        .retire_unclaimed_environments(down.instance_id.as_str())
        .await
        .unwrap();
    let EngineReplyValue::Execution(finished) = engine.dispatch(inspect()).await.unwrap() else {
        panic!("expected execution observation")
    };
    assert!(matches!(
        finished.runtime,
        Some(RuntimeInspection::NotFound { .. })
    ));
    assert_eq!(finished.last_error, None);
    assert!(driver
        .manager
        .managed_record(&unrelated.execution_id)
        .await
        .unwrap()
        .is_some());
    driver
        .retire_unclaimed_environments("other-instance")
        .await
        .unwrap();
}

fn intent(engine: &EngineKey, revision: u64, desired_state: DesiredState) -> EngineIntent {
    let spec = runtime_spec("fixture", 1, RuntimeUnitClass::Service);
    let template = EngineServiceTemplate {
        artifact: spec.artifact,
        process: spec.process,
        mounts: spec.mounts,
        secrets: spec.secrets,
        network: spec.network,
        resources: spec.resources,
        isolation: spec.isolation,
        health: spec.health,
        service_lifecycle: spec.service_lifecycle,
        restart: spec.restart,
        semantics_profile_digest: spec.semantics_profile_digest,
    };
    EngineIntent {
        directories: Vec::new(),
        prewarm_bindings_digest: None,
        schema: "kbox.engine.intent.v2".into(),
        engine_id: engine.clone(),
        instance_id: EngineKey::parse("instance").unwrap(),
        request_id: EngineKey::parse(format!("request-{revision}")).unwrap(),
        revision,
        desired_state,
        plan_digest: format!("sha256:{}", "1".repeat(64)),
        service: (desired_state == DesiredState::Up).then_some(template),
    }
}

async fn rpc(path: &std::path::Path, command: EngineCommand) -> EngineReplyValue {
    let mut stream = UnixStream::connect(path).await.unwrap();
    write_frame(&mut stream, &serde_json::to_vec(&request(command)).unwrap())
        .await
        .unwrap();
    let response: EngineReply =
        serde_json::from_slice(&read_frame(&mut stream).await.unwrap()).unwrap();
    assert_eq!(response.schema, "kbox.engine.response.v1");
    let EngineReplyBody::Ok { result } = response.body else {
        panic!("unexpected error: {response:?}")
    };
    *result
}

async fn ready(path: &std::path::Path) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while UnixStream::connect(path).await.is_err() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

async fn wait_state(path: &std::path::Path, running: bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let result = rpc(
                path,
                EngineCommand::InspectExecution {
                    instance_id: EngineKey::parse("instance").unwrap(),
                },
            )
            .await;
            if let EngineReplyValue::Execution(execution) = result {
                match execution.runtime {
                    Some(RuntimeInspection::Found { observation, .. })
                        if running
                            && observation.state == RuntimeUnitState::Running
                            && observation.generation == execution.target_generation =>
                    {
                        break
                    }
                    Some(RuntimeInspection::NotFound { .. }) | None
                        if !running && execution.desired_state == DesiredState::Down =>
                    {
                        break
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn engine_uses_only_the_explicit_service_profile_and_preserves_creation_identity() {
    let root = fixture_tempdir();
    let (driver, _) = fake_driver(&root);
    let driver = Arc::new(driver);
    let engine = Arc::new(
        LocalEngine::with_runtime(root.path().join("home/engine"), driver.clone()).unwrap(),
    );
    let id = discover(&engine).await;
    let mut up = intent(&id, 1, DesiredState::Up);
    up.service.as_mut().unwrap().semantics_profile_digest =
        Some(super::NATIVE_GUEST_PROFILE_DIGEST.into());
    engine.accept(up).await.unwrap();
    let path = root.path().join("control/s");
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    wait_state(&path, true).await;
    let records = driver.manager.managed_records().await.unwrap();
    assert_eq!(records.len(), 1);
    let config = &records[0]
        .managed_execution
        .as_ref()
        .unwrap()
        .request
        .config;
    assert!(config.privileged);
    assert!(config.cap_drop.is_empty());
    assert!(config.security_opt.is_empty());
    let down = intent(&id, 2, DesiredState::Down);
    engine.accept(down).await.unwrap();
    wait_state(&path, false).await;
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn engine_replacement_retires_previous_generation_and_reports_the_target_generation() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let driver = Arc::new(driver);
    let engine = Arc::new(
        LocalEngine::with_runtime(root.path().join("home/engine"), driver.clone()).unwrap(),
    );
    let id = discover(&engine).await;
    engine
        .accept(intent(&id, 1, DesiredState::Up))
        .await
        .unwrap();
    let path = root.path().join("control/s");
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    wait_state(&path, true).await;
    let old = driver.manager.managed_records().await.unwrap()[0]
        .id
        .clone();
    engine
        .accept(intent(&id, 2, DesiredState::Up))
        .await
        .unwrap();
    wait_state(&path, true).await;
    let records = driver.manager.managed_records().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_ne!(records[0].id, old);
    assert_eq!(backend.starts(), 2);
    assert_eq!(backend.kills(), 1);
    let EngineReplyValue::Execution(execution) = rpc(
        &path,
        EngineCommand::InspectExecution {
            instance_id: EngineKey::parse("instance").unwrap(),
        },
    )
    .await
    else {
        panic!("expected execution")
    };
    assert_eq!(execution.target_generation, 2);
    engine
        .accept(intent(&id, 3, DesiredState::Down))
        .await
        .unwrap();
    wait_state(&path, false).await;
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn engine_oci_service_replays_lost_response_survives_restart_and_cleans_owned_resources() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let directory = root.path().join("home/engine");
    let engine = Arc::new(LocalEngine::with_runtime(directory.clone(), Arc::new(driver)).unwrap());
    let engine_id = discover(&engine).await;
    let up = intent(&engine_id, 1, DesiredState::Up);
    let path = root.path().join("control/s");
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    // Drop the reply, then retry the exact request while the independent worker delivers it.
    let mut stream = UnixStream::connect(&path).await.unwrap();
    write_frame(
        &mut stream,
        &serde_json::to_vec(&request(EngineCommand::ApplyIntent {
            intent: Box::new(up.clone()),
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    drop(stream);
    let EngineReplyValue::Accepted(accepted) = rpc(
        &path,
        EngineCommand::ApplyIntent {
            intent: Box::new(up.clone()),
        },
    )
    .await
    else {
        panic!("expected accepted")
    };
    wait_state(&path, true).await;
    assert_eq!(backend.starts(), 1);
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    drop(engine);

    let driver = Arc::new(fake_driver_with_backend(&root, backend.clone()));
    let engine = Arc::new(LocalEngine::with_runtime(directory, driver.clone()).unwrap());
    assert_eq!(discover(&engine).await, engine_id);
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    let EngineReplyValue::Accepted(replayed) = rpc(
        &path,
        EngineCommand::ApplyIntent {
            intent: Box::new(up),
        },
    )
    .await
    else {
        panic!("expected accepted")
    };
    assert_eq!(accepted, replayed);
    wait_state(&path, true).await;
    assert_eq!(backend.starts(), 1);
    rpc(
        &path,
        EngineCommand::ApplyIntent {
            intent: Box::new(intent(&engine_id, 2, DesiredState::Down)),
        },
    )
    .await;
    wait_state(&path, false).await;
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    assert_eq!(backend.kills(), 1);
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn engine_new_stop_fences_previously_accepted_up_and_late_first_delivery() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let driver = Arc::new(driver);
    let engine = Arc::new(
        LocalEngine::with_runtime(root.path().join("home/engine"), driver.clone()).unwrap(),
    );
    let id = discover(&engine).await;
    let old = intent(&id, 1, DesiredState::Up);
    let accepted = engine.accept(old.clone()).await.unwrap();
    engine
        .accept(intent(&id, 3, DesiredState::Down))
        .await
        .unwrap();
    assert_eq!(engine.accept(old).await.unwrap(), accepted);
    assert_eq!(
        engine
            .accept(intent(&id, 2, DesiredState::Up))
            .await
            .unwrap_err()
            .code,
        EngineErrorCode::StaleIntent
    );
    let path = root.path().join("control/s");
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    let EngineReplyValue::Operation(operation) = rpc(
        &path,
        EngineCommand::InspectOperation {
            request_id: accepted.request_id,
        },
    )
    .await
    else {
        panic!("expected operation")
    };
    assert!(operation.superseded);
    assert!(operation.runtime.is_none());
    wait_state(&path, false).await;
    assert_eq!(backend.starts(), 0);
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn engine_rejects_malformed_and_oversized_control_without_poisoning_listener() {
    use tokio::io::AsyncWriteExt;
    let root = fixture_tempdir();
    let (driver, _) = fake_driver(&root);
    let engine = Arc::new(
        LocalEngine::with_runtime(root.path().join("home/engine"), Arc::new(driver)).unwrap(),
    );
    let path = root.path().join("control/s");
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine, async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    for (data, code) in [
        (b"broken".as_slice(), EngineErrorCode::InvalidRequest),
        (
            br#"{"schema":"kbox.engine.request.v99","command":{"op":"discover"}}"#,
            EngineErrorCode::UnsupportedVersion,
        ),
    ] {
        let mut stream = UnixStream::connect(&path).await.unwrap();
        write_frame(&mut stream, data).await.unwrap();
        let response: EngineReply =
            serde_json::from_slice(&read_frame(&mut stream).await.unwrap()).unwrap();
        assert!(matches!(response.body, EngineReplyBody::Error { error } if error.code == code));
    }
    let mut stream = UnixStream::connect(&path).await.unwrap();
    stream
        .write_all(&((MAX_FRAME_SIZE + 1) as u32).to_le_bytes())
        .await
        .unwrap();
    let response: EngineReply =
        serde_json::from_slice(&read_frame(&mut stream).await.unwrap()).unwrap();
    assert!(
        matches!(response.body, EngineReplyBody::Error { error } if error.code == EngineErrorCode::FrameTooLarge)
    );
    rpc(&path, EngineCommand::Discover).await;
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
