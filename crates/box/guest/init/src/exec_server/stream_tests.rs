use super::*;

struct PausedWriter {
    entered: mpsc::Sender<()>,
    release: Option<mpsc::Receiver<()>>,
    bytes: Vec<u8>,
}

impl Write for PausedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if let Some(release) = self.release.take() {
            self.entered.send(()).map_err(std::io::Error::other)?;
            release.recv().map_err(std::io::Error::other)?;
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn output(bytes: &[u8]) -> (Vec<u8>, i32) {
    let mut cursor = std::io::Cursor::new(bytes);
    let mut stdout = Vec::new();
    let mut code = None;
    while let Some((kind, payload)) = read_frame(&mut cursor).unwrap() {
        if kind == FrameType::Data as u8 {
            let chunk: ExecChunk = serde_json::from_slice(&payload).unwrap();
            if chunk.stream == StreamType::Stdout {
                stdout.extend(chunk.data);
            }
        } else if kind == FrameType::Control as u8 {
            assert!(code.is_none(), "only one terminal result");
            code = Some(
                serde_json::from_slice::<ExecExit>(&payload)
                    .unwrap()
                    .exit_code,
            );
        }
    }
    (stdout, code.expect("terminal result"))
}

#[test]
fn slow_stream_consumer_applies_backpressure_to_the_real_child() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("producer-finished");
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let env = vec![format!("PROBE_MARKER={}", marker.to_str().unwrap())];
    let worker = std::thread::spawn(move || {
        let mut writer = PausedWriter {
            entered,
            release: Some(release_rx),
            bytes: Vec::new(),
        };
        execute_command_streaming(
            ExecCommandSpec {
                cmd: &[
                    "sh".into(),
                    "-c".into(),
                    "head -c 2097152 /dev/zero; : > \"$PROBE_MARKER\"".into(),
                ],
                timeout_ns: 10_000_000_000,
                env: &env,
                working_dir: None,
                rootfs: None,
                stdin_data: None,
                stdin_streaming: false,
                user: None,
            },
            None,
            &mut writer,
        )
        .unwrap();
        writer.bytes
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let produced_without_consumer = marker.exists();
    release.send(()).unwrap();
    let bytes = worker.join().unwrap();
    assert!(marker.exists());
    let (stdout, code) = output(&bytes);
    assert_eq!(stdout, vec![0; 2 * 1024 * 1024]);
    assert_eq!(code, 0);
    assert!(
        !produced_without_consumer,
        "the child outran a paused consumer into unbounded buffering"
    );
}

#[test]
fn simultaneous_large_stdin_and_stdout_make_progress_and_preserve_tail() {
    for initial in [false, true] {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let input = vec![123; 2 * 1024 * 1024];
            execute_command_streaming(
                ExecCommandSpec {
                    cmd: &[
                        "sh".into(),
                        "-c".into(),
                        "head -c 2097152 /dev/zero; cat".into(),
                    ],
                    timeout_ns: 10_000_000_000,
                    env: &[],
                    working_dir: None,
                    rootfs: None,
                    stdin_data: initial.then_some(input.as_slice()),
                    stdin_streaming: !initial,
                    user: None,
                },
                (!initial).then_some(receiver),
                &mut bytes,
            )
            .unwrap();
            bytes
        });
        if !initial {
            sender
                .send(ExecInputEvent::Stdin(vec![123; 2 * 1024 * 1024]))
                .unwrap();
            sender.send(ExecInputEvent::StdinClose).unwrap();
        }
        let bytes = worker.join().unwrap();
        let (stdout, code) = output(&bytes);
        assert_eq!(code, 0);
        assert_eq!(&stdout[..2 * 1024 * 1024], &vec![0; 2 * 1024 * 1024]);
        assert_eq!(&stdout[2 * 1024 * 1024..], &vec![123; 2 * 1024 * 1024]);
    }
}

#[test]
fn explicit_unlimited_stream_outlives_the_legacy_five_second_default() {
    let mut bytes = Vec::new();
    execute_command_streaming(
        ExecCommandSpec {
            cmd: &["sh".into(), "-c".into(), "sleep 6; printf completed".into()],
            timeout_ns: kbox_core::exec::UNLIMITED_STREAM_TIMEOUT_NS,
            env: &[],
            working_dir: None,
            rootfs: None,
            stdin_data: None,
            stdin_streaming: false,
            user: None,
        },
        None,
        &mut bytes,
    )
    .unwrap();
    let (stdout, code) = output(&bytes);
    assert_eq!(stdout, b"completed");
    assert_eq!(code, 0);
}

struct FailedWriter;
impl Write for FailedWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "consumer disconnected",
        ))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn failed_output_drops_the_bounded_queue_and_reaps_the_child() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("child.pid");
    let env = vec![format!("PID_FILE={}", pid_file.to_str().unwrap())];
    let result = execute_command_streaming(
        ExecCommandSpec {
            cmd: &[
                "sh".into(),
                "-c".into(),
                "printf '%s' \"$$\" > \"$PID_FILE\"; head -c 2097152 /dev/zero; sleep 60".into(),
            ],
            timeout_ns: kbox_core::exec::UNLIMITED_STREAM_TIMEOUT_NS,
            env: &env,
            working_dir: None,
            rootfs: None,
            stdin_data: None,
            stdin_streaming: false,
            user: None,
        },
        None,
        &mut FailedWriter,
    );
    assert!(result.is_err());
    let pid = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<i32>()
        .unwrap();
    assert_eq!(
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None),
        Err(nix::errno::Errno::ESRCH)
    );
}
