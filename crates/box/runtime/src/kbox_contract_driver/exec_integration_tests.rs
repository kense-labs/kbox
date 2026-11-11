use kbox_contract::contract::{RuntimeExecRequest, RuntimeUnitClass, RuntimeUnitState};
use kbox_contract::{RuntimeDriver, RuntimeError};
use kbox_core::exec::{ExecOutput, ExecRequest};

use super::test_support::{accepted, action, fake_driver, fixture_tempdir, runtime_spec, unit};

#[cfg(unix)]
#[tokio::test]
async fn process_stream_bridge_preserves_raw_bytes_half_close_and_execution_identity() {
    use kbox_core::exec::{ExecChunk, ExecEvent, ExecExit, StreamType};
    let directory = fixture_tempdir();
    let (driver, _) = fake_driver(&directory);
    let spec = runtime_spec("raw-process", 1, RuntimeUnitClass::Service);
    let running = driver
        .apply(&super::test_support::apply_request(&spec), &accepted(&spec))
        .await
        .unwrap();
    let record = driver.find_generation(&spec).await.unwrap().unwrap();
    std::fs::create_dir_all(record.exec_socket_path.parent().unwrap()).unwrap();
    let listener = tokio::net::UnixListener::bind(&record.exec_socket_path).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, write) = tokio::io::split(stream);
        let mut reader = ktransport::FrameReader::new(read);
        let mut writer = ktransport::FrameWriter::new(write);
        let initial = reader.read_frame().await.unwrap().unwrap();
        let request: ExecRequest = serde_json::from_slice(&initial.payload).unwrap();
        assert!(request.streaming && request.stdin_streaming);
        assert!(request.request_id.is_none());
        writer
            .write_data(
                &serde_json::to_vec(&ExecChunk {
                    stream: StreamType::Stdout,
                    data: vec![255, 0, 10],
                })
                .unwrap(),
            )
            .await
            .unwrap();
        let input = reader.read_frame().await.unwrap().unwrap();
        assert_eq!(input.frame_type, ktransport::FrameType::Data);
        assert_eq!(input.payload, b"input\0\xff");
        let close = reader.read_frame().await.unwrap().unwrap();
        assert_eq!(close.frame_type, ktransport::FrameType::Control);
        assert_eq!(close.payload, b"stdin-close");
        writer
            .write_data(
                &serde_json::to_vec(&ExecChunk {
                    stream: StreamType::Stderr,
                    data: b"tail after EOF".to_vec(),
                })
                .unwrap(),
            )
            .await
            .unwrap();
        writer
            .write_control(
                &serde_json::to_vec(&ExecExit {
                    exit_code: 37,
                    oom_killed: false,
                })
                .unwrap(),
            )
            .await
            .unwrap();
    });
    let request = ExecRequest {
        request_id: None,
        cmd: vec!["/bin/cat".into()],
        timeout_ns: 5_000_000_000,
        env: Vec::new(),
        working_dir: None,
        rootfs: None,
        stdin: None,
        stdin_streaming: true,
        user: None,
        streaming: false,
    };
    let running_unit = unit(spec.clone(), running.clone());
    let generation = record.managed_execution.as_ref().unwrap().generation.get();
    for binding in [
        ("another-execution", generation),
        (record.id.as_str(), generation + 1),
    ] {
        assert!(matches!(
            driver
                .start_runtime_process_bound(&running_unit, request.clone(), Some(binding))
                .await,
            Err(RuntimeError::InvalidRequest(_))
        ));
    }
    let mut keyed = request.clone();
    keyed.request_id = Some("unary-replay".into());
    assert!(matches!(
        driver.start_runtime_process(&running_unit, keyed).await,
        Err(RuntimeError::InvalidRequest(_))
    ));
    let (execution_id, generation, mut stream) = driver
        .start_runtime_process(&running_unit, request.clone())
        .await
        .unwrap();
    assert_eq!(
        Some(execution_id.as_str()),
        running.provider_resource_id.as_deref()
    );
    assert_eq!(generation, kbox_core::ExecutionGeneration::INITIAL);
    assert!(
        matches!(stream.next_event().await.unwrap(), Some(ExecEvent::Chunk(chunk))
        if matches!(chunk.stream, StreamType::Stdout) && chunk.data == [255, 0, 10])
    );
    stream.input().write_stdin(b"input\0\xff").await.unwrap();
    stream.input().close_stdin().await.unwrap();
    assert!(
        matches!(stream.next_event().await.unwrap(), Some(ExecEvent::Chunk(chunk))
        if matches!(chunk.stream, StreamType::Stderr) && chunk.data == b"tail after EOF")
    );
    assert!(
        matches!(stream.next_event().await.unwrap(), Some(ExecEvent::Exit(exit)) if exit.exit_code == 37)
    );
    assert!(stream.next_event().await.unwrap().is_none());
    server.await.unwrap();
    driver
        .stop(&running_unit, &action("raw-stop", &spec))
        .await
        .unwrap();
    assert!(matches!(
        driver.start_runtime_process(&running_unit, request).await,
        Err(RuntimeError::InvalidRequest(_))
    ));
    driver
        .remove(&running_unit, &action("raw-remove", &spec))
        .await
        .unwrap();
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn exec_binds_identity_timeout_replay_and_truncation_to_the_running_generation() {
    let directory = fixture_tempdir();
    let (driver, _backend) = fake_driver(&directory);
    let spec = runtime_spec("exec-wire", 1, RuntimeUnitClass::Service);
    let running = driver
        .apply(
            &crate::kbox_contract_driver::test_support::apply_request(&spec),
            &accepted(&spec),
        )
        .await
        .unwrap();
    let provider_id = running.provider_resource_id.clone().unwrap();
    let record = driver.find_generation(&spec).await.unwrap().unwrap();
    std::fs::create_dir_all(record.exec_socket_path.parent().unwrap()).unwrap();
    let listener = tokio::net::UnixListener::bind(&record.exec_socket_path).unwrap();
    let (request_tx, mut request_rx) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let (read, write) = tokio::io::split(stream);
            let mut reader = ktransport::FrameReader::new(read);
            let mut writer = ktransport::FrameWriter::new(write);
            let frame = reader.read_frame().await.unwrap().unwrap();
            assert_eq!(frame.frame_type, ktransport::FrameType::Data);
            let request: ExecRequest = serde_json::from_slice(&frame.payload).unwrap();
            request_tx.send(request).unwrap();
            writer
                .write_data(
                    &serde_json::to_vec(&ExecOutput {
                        stdout: b"replayed stdout\n".to_vec(),
                        stderr: b"bounded stderr\n".to_vec(),
                        exit_code: 23,
                        truncated: true,
                    })
                    .unwrap(),
                )
                .await
                .unwrap();
        }
    });

    let request = RuntimeExecRequest {
        schema: RuntimeExecRequest::SCHEMA.into(),
        request_id: "exec-request-1".into(),
        unit_id: spec.unit_id.clone(),
        generation: spec.generation,
        command: vec!["/bin/sh".into(), "-c".into(), "printf ready".into()],
        timeout_ms: 500,
        deadline_at_ms: None,
    };
    let running_unit = unit(spec.clone(), running.clone());
    let first = driver.exec(&running_unit, &request).await.unwrap();
    let replayed = driver.exec(&running_unit, &request).await.unwrap();
    assert_eq!(first.request_id, request.request_id);
    assert_eq!(
        first.observation.provider_resource_id.as_deref(),
        Some(provider_id.as_str())
    );
    assert_eq!(first.exit_code, 23);
    assert_eq!(first.stdout, "replayed stdout\n");
    assert_eq!(first.stderr, "bounded stderr\n");
    assert!(first.truncated);
    assert_eq!(replayed.exit_code, first.exit_code);
    assert_eq!(replayed.stdout, first.stdout);
    assert_eq!(replayed.stderr, first.stderr);
    assert!(replayed.truncated);

    for _ in 0..2 {
        let guest_request = request_rx.recv().await.unwrap();
        assert_eq!(guest_request.request_id.as_deref(), Some("exec-request-1"));
        assert_eq!(guest_request.cmd, request.command);
        assert_eq!(guest_request.timeout_ns, 500_000_000);
        assert!(!guest_request.streaming);
    }
    server.await.unwrap();

    let mut wrong_generation = request.clone();
    wrong_generation.generation += 1;
    assert!(matches!(
        driver.exec(&running_unit, &wrong_generation).await,
        Err(RuntimeError::InvalidRequest(message)) if message.contains("identity")
    ));

    let stopped = driver
        .stop(&running_unit, &action("exec-service-stop", &spec))
        .await
        .unwrap();
    assert_eq!(stopped.state, RuntimeUnitState::Stopped);
    assert!(matches!(
        driver.exec(&unit(spec, stopped), &request).await,
        Err(RuntimeError::InvalidRequest(message)) if message.contains("running unit")
    ));
}
