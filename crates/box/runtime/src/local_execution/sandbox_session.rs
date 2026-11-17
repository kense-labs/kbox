//! Streaming process sessions for Spec050 Sandbox via Docker exec attach.
//!
//! `start_pty` hijacks a TTY exec; `start_process` hijacks a non-TTY multiplexed
//! attach (stdout/stderr frames). Never falls through to the MicroVM guest
//! PTY/exec sockets.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kbox_core::exec::{ExecChunk, ExecEvent, ExecExit, ExecRequest, StreamType};
use kbox_core::pty::PtyRequest;
use kbox_core::{
    ExecutionManagerError, ExecutionManagerResult, ExecutionProcess, ExecutionProcessInput,
    ExecutionProcessSignal, ExecutionProcessStream,
};
use tokio::sync::mpsc;

use super::sandbox_backend::SandboxLocalExecutionBackend;
use crate::BoxRecord;

const OUTPUT_CHANNEL: usize = 64;

pub(super) async fn start_process(
    backend: &SandboxLocalExecutionBackend,
    record: &BoxRecord,
    request: ExecRequest,
) -> ExecutionManagerResult<ExecutionProcess> {
    #[cfg(not(unix))]
    {
        let _ = (backend, record, request);
        return Err(ExecutionManagerError::Unavailable(
            "sandbox engine is unavailable on this host".into(),
        ));
    }
    #[cfg(unix)]
    {
        backend.metadata(record)?;
        if request.request_id.is_some() {
            return Err(ExecutionManagerError::InvalidRequest(
                "sandbox streaming exec does not accept a one-shot request_id".into(),
            ));
        }
        if request.cmd.is_empty() {
            return Err(ExecutionManagerError::InvalidRequest(
                "sandbox streaming exec requires a non-empty command".into(),
            ));
        }
        if request.rootfs.as_ref().is_some_and(|path| !path.is_empty()) {
            return Err(ExecutionManagerError::Unavailable(
                "sandbox streaming exec does not support guest rootfs chroot".into(),
            ));
        }
        let id = backend.load_container_id(record)?.ok_or_else(|| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox container id missing for execution {}",
                record.id
            ))
        })?;
        // Fail closed: never spawn a daemon just for streaming, and never fall
        // through to the MicroVM guest exec socket.
        let client = backend.client(false)?;
        let env = if request.env.is_empty() {
            None
        } else {
            Some(request.env.as_slice())
        };
        let hijack = client
            .container_exec_attach(
                &id,
                &request.cmd,
                env,
                request.user.as_deref(),
                request.working_dir.as_deref(),
            )
            .map_err(|error| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox streaming exec failed: {error}"
                ))
            })?;
        let mut process = SandboxHijackStream::spawn(hijack, false)?;
        if let Some(stdin) = request.stdin.filter(|bytes| !bytes.is_empty()) {
            process.input().write_stdin(&stdin).await?;
        }
        if !request.stdin_streaming {
            process.input().close_stdin().await?;
        }
        Ok(Box::new(process))
    }
}

pub(super) async fn start_pty(
    backend: &SandboxLocalExecutionBackend,
    record: &BoxRecord,
    request: PtyRequest,
) -> ExecutionManagerResult<ExecutionProcess> {
    #[cfg(not(unix))]
    {
        let _ = (backend, record, request);
        return Err(ExecutionManagerError::Unavailable(
            "sandbox engine is unavailable on this host".into(),
        ));
    }
    #[cfg(unix)]
    {
        backend.metadata(record)?;
        if request.cmd.is_empty() {
            return Err(ExecutionManagerError::InvalidRequest(
                "sandbox PTY requires a non-empty command".into(),
            ));
        }
        if request.cols == 0 || request.rows == 0 {
            return Err(ExecutionManagerError::InvalidRequest(
                "PTY width and height must both be positive".into(),
            ));
        }
        if request.rootfs.as_ref().is_some_and(|path| !path.is_empty()) {
            return Err(ExecutionManagerError::Unavailable(
                "sandbox PTY does not support guest rootfs chroot".into(),
            ));
        }
        let id = backend.load_container_id(record)?.ok_or_else(|| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox container id missing for execution {}",
                record.id
            ))
        })?;
        // Fail closed: never spawn a daemon just for PTY, and never fall
        // through to the MicroVM guest PTY socket.
        let client = backend.client(false)?;
        let env = if request.env.is_empty() {
            None
        } else {
            Some(request.env.as_slice())
        };
        let hijack = client
            .container_exec_pty(
                &id,
                &request.cmd,
                env,
                request.user.as_deref(),
                request.working_dir.as_deref(),
                request.cols,
                request.rows,
            )
            .map_err(|error| {
                ExecutionManagerError::Unavailable(format!("sandbox PTY exec failed: {error}"))
            })?;
        Ok(Box::new(SandboxHijackStream::spawn(hijack, true)?))
    }
}

#[cfg(unix)]
struct SandboxHijackInput {
    writer: Mutex<Option<std::os::unix::net::UnixStream>>,
    exec_id: String,
    socket: std::path::PathBuf,
    tty: bool,
    closed: Mutex<bool>,
}

#[cfg(unix)]
struct SandboxHijackStream {
    input: Arc<SandboxHijackInput>,
    events: mpsc::Receiver<ExecutionManagerResult<ExecEvent>>,
    done: bool,
}

#[cfg(unix)]
impl SandboxHijackStream {
    fn spawn(
        mut hijack: kbox_sandbox_engine::ExecHijack,
        tty: bool,
    ) -> ExecutionManagerResult<Self> {
        let writer = hijack.try_clone_writer().map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox attach could not clone socket: {error}"
            ))
        })?;
        let input = Arc::new(SandboxHijackInput {
            writer: Mutex::new(Some(writer)),
            exec_id: hijack.exec_id.clone(),
            socket: hijack.socket.clone(),
            tty,
            closed: Mutex::new(false),
        });
        let (tx, rx) = mpsc::channel(OUTPUT_CHANNEL);
        let exec_id = hijack.exec_id.clone();
        let socket = hijack.socket.clone();
        let thread_name = if tty { "sandbox-pty" } else { "sandbox-stream" };
        std::thread::Builder::new()
            .name(thread_name.into())
            .spawn(move || {
                loop {
                    match hijack.next_output() {
                        Ok(None) => break,
                        Ok(Some(chunk)) => {
                            let event = match chunk {
                                kbox_sandbox_engine::HijackOutput::Stdout(data) => {
                                    ExecEvent::Chunk(ExecChunk {
                                        stream: StreamType::Stdout,
                                        data,
                                    })
                                }
                                kbox_sandbox_engine::HijackOutput::Stderr(data) => {
                                    ExecEvent::Chunk(ExecChunk {
                                        stream: StreamType::Stderr,
                                        data,
                                    })
                                }
                            };
                            if tx.blocking_send(Ok(event)).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tx.blocking_send(Err(ExecutionManagerError::Unavailable(
                                format!("sandbox attach read failed: {error}"),
                            )));
                            return;
                        }
                    }
                }
                let exit_code = kbox_sandbox_engine::DockerHttpClient::new(socket)
                    .exec_inspect_exit(&exec_id)
                    .unwrap_or(-1);
                let _ = tx.blocking_send(Ok(ExecEvent::Exit(ExecExit {
                    exit_code,
                    oom_killed: false,
                })));
            })
            .map_err(|error| {
                ExecutionManagerError::Internal(format!(
                    "sandbox attach reader thread failed to start: {error}"
                ))
            })?;
        Ok(Self {
            input,
            events: rx,
            done: false,
        })
    }
}

#[cfg(unix)]
#[async_trait]
impl ExecutionProcessInput for SandboxHijackInput {
    async fn write_stdin(&self, data: &[u8]) -> ExecutionManagerResult<()> {
        let payload = data.to_vec();
        let stream = {
            let guard = self.writer.lock().map_err(|_| {
                ExecutionManagerError::Internal("sandbox attach stdin lock poisoned".into())
            })?;
            let Some(stream) = guard.as_ref() else {
                return Err(ExecutionManagerError::Unavailable(
                    "sandbox attach stdin is closed".into(),
                ));
            };
            stream.try_clone().map_err(|error| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox attach stdin clone failed: {error}"
                ))
            })?
        };
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let mut stream = stream;
            stream.write_all(&payload).map_err(|error| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox attach stdin write failed: {error}"
                ))
            })
        })
        .await
        .map_err(|error| {
            ExecutionManagerError::Internal(format!("sandbox attach stdin task failed: {error}"))
        })?
    }

    async fn close_stdin(&self) -> ExecutionManagerResult<()> {
        let mut closed = self.closed.lock().map_err(|_| {
            ExecutionManagerError::Internal("sandbox attach stdin lock poisoned".into())
        })?;
        if *closed {
            return Ok(());
        }
        *closed = true;
        let mut guard = self.writer.lock().map_err(|_| {
            ExecutionManagerError::Internal("sandbox attach stdin lock poisoned".into())
        })?;
        if let Some(stream) = guard.take() {
            let _ = stream.shutdown(std::net::Shutdown::Write);
        }
        Ok(())
    }

    async fn cancel(&self) -> ExecutionManagerResult<()> {
        // Best-effort: shutting down attach ends the host pump; the guest
        // process may still run. Docker has no exec-kill API on this path.
        let stream = {
            let mut guard = self.writer.lock().map_err(|_| {
                ExecutionManagerError::Internal("sandbox attach stdin lock poisoned".into())
            })?;
            guard.take()
        };
        if let Some(stream) = stream {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        if let Ok(mut closed) = self.closed.lock() {
            *closed = true;
        }
        Ok(())
    }

    async fn send_signal(&self, signal: ExecutionProcessSignal) -> ExecutionManagerResult<()> {
        match signal {
            ExecutionProcessSignal::Kill => self.cancel().await,
            ExecutionProcessSignal::Terminate => Err(ExecutionManagerError::InvalidRequest(
                "sandbox attach does not support graceful termination".into(),
            )),
        }
    }

    async fn resize_pty(&self, cols: u16, rows: u16) -> ExecutionManagerResult<()> {
        if !self.tty {
            return Err(ExecutionManagerError::InvalidRequest(
                "sandbox non-TTY streaming session does not support PTY resize".into(),
            ));
        }
        let exec_id = self.exec_id.clone();
        let socket = self.socket.clone();
        tokio::task::spawn_blocking(move || {
            kbox_sandbox_engine::DockerHttpClient::new(socket)
                .exec_resize(&exec_id, cols, rows)
                .map_err(|error| {
                    ExecutionManagerError::Unavailable(format!(
                        "sandbox PTY resize failed: {error}"
                    ))
                })
        })
        .await
        .map_err(|error| {
            ExecutionManagerError::Internal(format!("sandbox PTY resize task failed: {error}"))
        })?
    }
}

#[cfg(unix)]
#[async_trait]
impl ExecutionProcessStream for SandboxHijackStream {
    fn input(&self) -> Arc<dyn ExecutionProcessInput> {
        self.input.clone()
    }

    async fn next_event(&mut self) -> ExecutionManagerResult<Option<ExecEvent>> {
        if self.done {
            return Ok(None);
        }
        match self.events.recv().await {
            Some(Ok(event)) => {
                if matches!(event, ExecEvent::Exit(_)) {
                    self.done = true;
                }
                Ok(Some(event))
            }
            Some(Err(error)) => {
                self.done = true;
                Err(error)
            }
            None => {
                self.done = true;
                Ok(None)
            }
        }
    }
}
