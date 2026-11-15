use std::collections::BTreeMap;

use kbox_core::exec::{ExecChunk, ExecExit, ExecRequest, StreamType, UNLIMITED_STREAM_TIMEOUT_NS};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;

use super::*;
use crate::engine::{
    EnginePrimaryInfo, EnginePrimaryOpen, EnginePrimaryTicket, EngineProcessState,
};
use crate::kbox_contract_driver::test_support::DriverFakeBackend;
use crate::pool::attach::{AttachInput, AttachOutput};
use crate::BoxRuntimeDriver;

struct Fixture {
    root: tempfile::TempDir,
    engine: Arc<LocalEngine>,
    driver: Arc<BoxRuntimeDriver>,
    backend: Arc<DriverFakeBackend>,
    id: EngineKey,
    path: std::path::PathBuf,
    stop: oneshot::Sender<()>,
    server: tokio::task::JoinHandle<crate::engine::EngineResult<()>>,
}

impl Fixture {
    async fn new() -> Self {
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
        Self {
            root,
            engine,
            driver,
            backend,
            id,
            path,
            stop,
            server,
        }
    }

    fn process(&self) -> EnginePrimaryOpen {
        EnginePrimaryOpen {
            schema: "kbox.engine.primary-open.v1".into(),
            engine_id: self.id.clone(),
            instance_id: EngineKey::parse("instance").unwrap(),
            request_id: EngineKey::parse("primary-1").unwrap(),
            generation: 1,
            command: vec!["/application/launch".into(), "".into()],
            environment: BTreeMap::from([("TOKEN".into(), "synthetic-secret".into())]),
            working_directory: Some("/workspace".into()),
            timeout_ms: None,
            bootstrap: false,
        }
    }

    async fn open(&self) -> (UnixStream, EnginePrimaryTicket) {
        self.open_request(self.process()).await
    }

    async fn open_request(&self, process: EnginePrimaryOpen) -> (UnixStream, EnginePrimaryTicket) {
        let mut control = UnixStream::connect(&self.path).await.unwrap();
        write_frame(
            &mut control,
            &serde_json::to_vec(&request(EngineCommand::OpenPrimaryProcess {
                process: Box::new(process),
            }))
            .unwrap(),
        )
        .await
        .unwrap();
        let reply: EngineReply =
            serde_json::from_slice(&read_frame(&mut control).await.unwrap()).unwrap();
        let EngineReplyBody::Ok { result } = reply.body else {
            panic!("{reply:?}")
        };
        let EngineReplyValue::PrimaryTicket(ticket) = *result else {
            panic!("expected primary ticket")
        };
        (control, ticket)
    }

    async fn attach(&self, ticket: &EnginePrimaryTicket) -> (UnixStream, EnginePrimaryInfo) {
        let mut data = UnixStream::connect(&self.path).await.unwrap();
        write_frame(
            &mut data,
            &serde_json::to_vec(&request(EngineCommand::AttachPrimaryProcess {
                token: ticket.attach_token.clone(),
            }))
            .unwrap(),
        )
        .await
        .unwrap();
        let reply: EngineReply =
            serde_json::from_slice(&read_frame(&mut data).await.unwrap()).unwrap();
        let EngineReplyBody::Ok { result } = reply.body else {
            panic!("{reply:?}")
        };
        let EngineReplyValue::PrimaryProcess(info) = *result else {
            panic!("expected primary process")
        };
        (data, info)
    }

    async fn error(&self, command: EngineCommand) -> EngineErrorCode {
        let mut stream = UnixStream::connect(&self.path).await.unwrap();
        write_frame(&mut stream, &serde_json::to_vec(&request(command)).unwrap())
            .await
            .unwrap();
        let reply: EngineReply =
            serde_json::from_slice(&read_frame(&mut stream).await.unwrap()).unwrap();
        let EngineReplyBody::Error { error } = reply.body else {
            panic!("expected error: {reply:?}")
        };
        error.code
    }

    async fn guest(&self) -> (UnixListener, String) {
        let record = self
            .driver
            .manager
            .managed_records()
            .await
            .unwrap()
            .remove(0);
        std::fs::create_dir_all(record.exec_socket_path.parent().unwrap()).unwrap();
        (
            UnixListener::bind(&record.exec_socket_path).unwrap(),
            record.id,
        )
    }

    async fn replaced(&self, old: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let EngineReplyValue::Execution(info) = rpc(
                    &self.path,
                    EngineCommand::InspectExecution {
                        instance_id: EngineKey::parse("instance").unwrap(),
                    },
                )
                .await
                else {
                    panic!("expected execution")
                };
                if info.target_generation == 2 {
                    if let Some(RuntimeInspection::Found { observation, .. }) = info.runtime {
                        if observation.generation == 2
                            && observation.state == RuntimeUnitState::Running
                        {
                            assert_ne!(observation.provider_resource_id.as_deref(), Some(old));
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(self.backend.starts(), 2);
        assert_eq!(self.backend.kills(), 1);
        assert_eq!(
            self.driver.manager.managed_records().await.unwrap().len(),
            1
        );
    }

    async fn close(self) {
        self.engine
            .accept(intent(&self.id, 2, DesiredState::Down))
            .await
            .unwrap();
        wait_state(&self.path, false).await;
        assert!(self
            .driver
            .manager
            .managed_records()
            .await
            .unwrap()
            .is_empty());
        self.stop.send(()).unwrap();
        self.server.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn engine_bootstrap_is_sent_before_user_input_and_absent_from_durable_records() {
    let fixture = Fixture::new().await;
    let (listener, old) = fixture.guest().await;
    let marker = "synthetic-credential-bootstrap";
    let mut payload = marker.as_bytes().to_vec();
    payload.resize(80 * 1024, 0xa5);
    let expected = payload.clone();
    let guest = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, writer) = stream.into_split();
        let mut reader = ktransport::FrameReader::new(reader);
        let mut writer = ktransport::FrameWriter::new(writer);
        let request = reader.read_frame().await.unwrap().unwrap();
        assert!(!std::str::from_utf8(&request.payload)
            .unwrap()
            .contains(marker));
        let request: ExecRequest = serde_json::from_slice(&request.payload).unwrap();
        assert!(request
            .env
            .iter()
            .any(|value| value == "BOOTSTRAP_TEST=ordinary"));
        assert!(request.stdin.is_none());
        for bytes in expected.chunks(64 * 1024) {
            let frame = reader.read_frame().await.unwrap().unwrap();
            assert_eq!(frame.payload, bytes);
        }
        let raw = reader.read_frame().await.unwrap().unwrap();
        assert_eq!(raw.payload, b"raw-acp");
        writer
            .write_control(
                &serde_json::to_vec(&ExecExit {
                    exit_code: 0,
                    oom_killed: false,
                })
                .unwrap(),
            )
            .await
            .unwrap();
    });
    let mut request = fixture.process();
    request.bootstrap = true;
    request.environment = BTreeMap::from([("BOOTSTRAP_TEST".into(), "ordinary".into())]);
    let (mut control, ticket) = fixture.open_request(request).await;
    write_frame(&mut control, &payload).await.unwrap();
    let (mut data, _) = fixture.attach(&ticket).await;
    write_frame(&mut data, &AttachInput::Data(b"raw-acp".to_vec()).encode())
        .await
        .unwrap();
    assert_eq!(
        AttachOutput::parse(&read_frame(&mut data).await.unwrap()).unwrap(),
        AttachOutput::Exit(0)
    );
    guest.await.unwrap();
    drop(control);
    fixture.replaced(&old).await;
    assert!(
        !std::fs::read_to_string(fixture.root.path().join("home/engine/intent.json"))
            .unwrap()
            .contains(marker)
    );
    fixture.close().await;
}

#[tokio::test]
async fn engine_incomplete_bootstrap_never_reaches_the_guest() {
    let fixture = Fixture::new().await;
    let (guest, old) = fixture.guest().await;
    let mut request = fixture.process();
    request.bootstrap = true;
    let (mut control, ticket) = fixture.open_request(request).await;
    control.write_u32_le(12).await.unwrap();
    control.write_all(b"part").await.unwrap();
    drop(control);
    fixture.replaced(&old).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(50), guest.accept())
            .await
            .is_err()
    );
    assert_eq!(
        fixture
            .error(EngineCommand::AttachPrimaryProcess {
                token: ticket.attach_token,
            })
            .await,
        EngineErrorCode::StreamLost
    );
    fixture.close().await;
}

#[tokio::test]
async fn engine_primary_stream_preserves_bytes_eof_exit_and_fences_replays() {
    let fixture = Fixture::new().await;
    let (listener, old) = fixture.guest().await;
    let guest = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, writer) = stream.into_split();
        let mut reader = ktransport::FrameReader::new(reader);
        let mut writer = ktransport::FrameWriter::new(writer);
        let frame = reader.read_frame().await.unwrap().unwrap();
        let request: ExecRequest = serde_json::from_slice(&frame.payload).unwrap();
        assert_eq!(request.timeout_ns, UNLIMITED_STREAM_TIMEOUT_NS);
        assert_eq!(request.cmd, ["/application/launch", ""]);
        assert!(request.env.iter().any(|v| v == "TOKEN=synthetic-secret"));
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
        assert_eq!(input.payload, [0, 255, 7]);
        let close = reader.read_frame().await.unwrap().unwrap();
        assert_eq!(close.frame_type, ktransport::FrameType::Control);
        assert_eq!(close.payload, b"stdin-close");
        writer
            .write_data(
                &serde_json::to_vec(&ExecChunk {
                    stream: StreamType::Stderr,
                    data: "尾部 stderr\n".as_bytes().to_vec(),
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
    let (control, ticket) = fixture.open().await;
    let (mut data, info) = fixture.attach(&ticket).await;
    assert_eq!(info.state, EngineProcessState::Running);
    assert_eq!(info.execution_id.as_deref(), Some(old.as_str()));
    assert_eq!(
        fixture
            .error(EngineCommand::OpenPrimaryProcess {
                process: Box::new(fixture.process())
            })
            .await,
        EngineErrorCode::AlreadyConnected
    );
    assert_eq!(
        fixture
            .error(EngineCommand::AttachPrimaryProcess {
                token: ticket.attach_token
            })
            .await,
        EngineErrorCode::AlreadyConnected
    );
    assert_eq!(
        AttachOutput::parse(&read_frame(&mut data).await.unwrap()).unwrap(),
        AttachOutput::Stdout(vec![255, 0, 10])
    );
    write_frame(&mut data, &AttachInput::Data(vec![0, 255, 7]).encode())
        .await
        .unwrap();
    write_frame(&mut data, &AttachInput::Close.encode())
        .await
        .unwrap();
    assert_eq!(
        AttachOutput::parse(&read_frame(&mut data).await.unwrap()).unwrap(),
        AttachOutput::Stderr("尾部 stderr\n".as_bytes().to_vec())
    );
    assert_eq!(
        AttachOutput::parse(&read_frame(&mut data).await.unwrap()).unwrap(),
        AttachOutput::Exit(37)
    );
    assert!(read_frame(&mut data).await.is_err());
    guest.await.unwrap();
    drop(control);
    fixture.replaced(&old).await;
    let mut stale = fixture.process();
    stale.request_id = EngineKey::parse("late-primary").unwrap();
    assert_eq!(
        fixture
            .error(EngineCommand::OpenPrimaryProcess {
                process: Box::new(stale)
            })
            .await,
        EngineErrorCode::GenerationMismatch
    );
    assert_eq!(
        fixture
            .error(EngineCommand::OpenPrimaryProcess {
                process: Box::new(fixture.process())
            })
            .await,
        EngineErrorCode::StreamLost
    );
    let EngineReplyValue::PrimaryProcess(result) = rpc(
        &fixture.path,
        EngineCommand::InspectPrimaryProcess {
            request_id: fixture.process().request_id,
        },
    )
    .await
    else {
        panic!("expected process")
    };
    assert_eq!(result.state, EngineProcessState::Exited);
    assert_eq!(result.exit.unwrap().exit_code, 37);
    assert!(
        !std::fs::read_to_string(fixture.root.path().join("home/engine/intent.json"))
            .unwrap()
            .contains("synthetic-secret")
    );
    fixture.close().await;
}

#[tokio::test]
async fn engine_lost_primary_ticket_retires_environment_without_launching_the_process() {
    let fixture = Fixture::new().await;
    let old = fixture.driver.manager.managed_records().await.unwrap()[0]
        .id
        .clone();
    let (control, ticket) = fixture.open().await;
    drop(control);
    fixture.replaced(&old).await;
    assert_eq!(
        fixture
            .error(EngineCommand::AttachPrimaryProcess {
                token: ticket.attach_token
            })
            .await,
        EngineErrorCode::StreamLost
    );
    assert_eq!(
        fixture
            .error(EngineCommand::OpenPrimaryProcess {
                process: Box::new(fixture.process())
            })
            .await,
        EngineErrorCode::StreamLost
    );
    let EngineReplyValue::PrimaryProcess(result) = rpc(
        &fixture.path,
        EngineCommand::InspectPrimaryProcess {
            request_id: fixture.process().request_id,
        },
    )
    .await
    else {
        panic!("expected process")
    };
    assert_eq!(result.state, EngineProcessState::Lost);
    assert!(result.execution_id.is_none());
    assert!(result.exit.is_none());
    fixture.close().await;
}

#[tokio::test]
async fn engine_control_disconnect_interrupts_a_backpressured_input_with_data_still_open() {
    let fixture = Fixture::new().await;
    let (listener, old) = fixture.guest().await;
    let guest = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = ktransport::FrameReader::new(stream);
        reader.read_frame().await.unwrap().unwrap();
        std::future::pending::<()>().await;
        drop(reader);
    });
    let (control, ticket) = fixture.open().await;
    let (data, _) = fixture.attach(&ticket).await;
    let (reader, mut writer) = data.into_split();
    let writer = tokio::spawn(async move {
        let frame = AttachInput::Data(vec![123; 64 * 1024]).encode();
        for _ in 0..256 {
            write_frame(&mut writer, &frame).await?;
        }
        Ok::<_, std::io::Error>(())
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !writer.is_finished(),
        "the input fixture failed to produce backpressure"
    );
    drop(control);
    fixture.replaced(&old).await;
    assert!(tokio::time::timeout(Duration::from_secs(3), writer)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    drop(reader);
    guest.abort();
    assert!(guest.await.unwrap_err().is_cancelled());
    fixture.close().await;
}

#[tokio::test]
async fn engine_shutdown_retires_live_primary_and_restart_prepares_without_replaying() {
    use tokio::io::AsyncReadExt;
    let fixture = Fixture::new().await;
    let (listener, old) = fixture.guest().await;
    let guest = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = ktransport::FrameReader::new(stream);
        reader.read_frame().await.unwrap().unwrap();
        std::future::pending::<()>().await;
        drop(reader);
    });
    let (mut control, ticket) = fixture.open().await;
    let (mut data, info) = fixture.attach(&ticket).await;
    assert_eq!(info.state, EngineProcessState::Running);
    let Fixture {
        root,
        engine,
        driver,
        backend,
        id,
        path,
        stop,
        server,
    } = fixture;
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    assert_eq!(backend.starts(), 1);
    assert_eq!(backend.kills(), 1);
    let mut byte = [0];
    assert_eq!(control.read(&mut byte).await.unwrap(), 0);
    assert!(read_frame(&mut data).await.is_err());
    guest.abort();
    assert!(guest.await.unwrap_err().is_cancelled());
    drop(engine);
    let engine = Arc::new(
        LocalEngine::with_runtime(root.path().join("home/engine"), driver.clone()).unwrap(),
    );
    assert_eq!(discover(&engine).await, id);
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(serve_engine(path.clone(), engine.clone(), async {
        let _ = stopped.await;
    }));
    ready(&path).await;
    let fixture = Fixture {
        root,
        engine,
        driver,
        backend,
        id,
        path,
        stop,
        server,
    };
    fixture.replaced(&old).await;
    assert_eq!(
        fixture
            .error(EngineCommand::OpenPrimaryProcess {
                process: Box::new(fixture.process())
            })
            .await,
        EngineErrorCode::StreamLost
    );
    assert_eq!(
        fixture
            .error(EngineCommand::AttachPrimaryProcess {
                token: ticket.attach_token
            })
            .await,
        EngineErrorCode::StreamLost
    );
    let EngineReplyValue::PrimaryProcess(process) = rpc(
        &fixture.path,
        EngineCommand::InspectPrimaryProcess {
            request_id: fixture.process().request_id,
        },
    )
    .await
    else {
        panic!("expected process")
    };
    assert_eq!(process.state, EngineProcessState::Lost);
    assert_eq!(process.execution_id.as_deref(), Some(old.as_str()));
    assert!(process.exit.is_none());
    fixture.close().await;
}

#[tokio::test]
async fn engine_primary_drains_tail_when_guest_stdin_closes_before_exit() {
    let fixture = Fixture::new().await;
    let (listener, old) = fixture.guest().await;
    let guest = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut reader = ktransport::FrameReader::new(&mut stream);
        reader.read_frame().await.unwrap().unwrap();
        drop(reader);
        let stream = stream.into_std().unwrap();
        stream.shutdown(std::net::Shutdown::Read).unwrap();
        let stream = UnixStream::from_std(stream).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut writer = ktransport::FrameWriter::new(stream);
        writer
            .write_data(
                &serde_json::to_vec(&ExecChunk {
                    stream: StreamType::Stderr,
                    data: b"tail after closed input".to_vec(),
                })
                .unwrap(),
            )
            .await
            .unwrap();
        writer
            .write_control(
                &serde_json::to_vec(&ExecExit {
                    exit_code: 23,
                    oom_killed: false,
                })
                .unwrap(),
            )
            .await
            .unwrap();
    });
    let (control, ticket) = fixture.open().await;
    let (data, _) = fixture.attach(&ticket).await;
    let (mut reader, mut writer) = data.into_split();
    let input = tokio::spawn(async move {
        let frame = AttachInput::Data(vec![123; 64 * 1024]).encode();
        for _ in 0..256 {
            if write_frame(&mut writer, &frame).await.is_err() {
                break;
            }
        }
        writer
    });
    let tail = read_frame(&mut reader).await.unwrap();
    assert_eq!(
        AttachOutput::parse(&tail).unwrap(),
        AttachOutput::Stderr(b"tail after closed input".to_vec())
    );
    assert_eq!(
        AttachOutput::parse(&read_frame(&mut reader).await.unwrap()).unwrap(),
        AttachOutput::Exit(23)
    );
    guest.await.unwrap();
    input.await.unwrap();
    drop(control);
    drop(reader);
    fixture.replaced(&old).await;
    fixture.close().await;
}
