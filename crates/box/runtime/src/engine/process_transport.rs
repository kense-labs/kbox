use std::sync::Arc;

use kbox_core::exec::{ExecEvent, ExecExit, StreamType};
use kbox_core::{ExecutionProcess, ExecutionProcessInput};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::UnixStream;
use zeroize::Zeroizing;

use crate::pool::attach::{AttachInput, AttachOutput};
use crate::pool::client::write_frame;

use super::process::PendingPrimary;
use super::process_protocol::MAX_BOOTSTRAP_BYTES;
use super::transport::{write_reply, CONTROL_TIMEOUT};
use super::{EngineError, EngineErrorCode, EngineReplyValue, EngineResult, LocalEngine};

pub(super) const MAX_PROCESS_DATA_BYTES: usize = 64 * 1024;

pub(super) async fn serve_primary(
    mut control: UnixStream,
    engine: Arc<LocalEngine>,
    pending: PendingPrimary,
) -> EngineResult<()> {
    let PendingPrimary {
        ticket: _,
        request,
        data,
        owner: _owner,
    } = pending;
    let bootstrap = if request.bootstrap {
        read_bootstrap(&mut control).await?
    } else {
        Zeroizing::new(Vec::new())
    };
    let mut marker = [0; 1];
    let mut data = tokio::select! {
        biased;
        _ = control.read(&mut marker) => return Err(stream_lost()),
        data = tokio::time::timeout(CONTROL_TIMEOUT, data) => data
            .map_err(|_| stream_lost())?.map_err(|_| stream_lost())?,
    };
    let started = tokio::select! {
        biased;
        _ = control.read(&mut marker) => return Err(stream_lost()),
        result = engine.start_primary(&request, &bootstrap) => result,
    };
    drop(bootstrap);
    let (info, mut process) = match started {
        Ok(started) => started,
        Err(error) => {
            write_reply(&mut data, Err(error)).await?;
            return Err(stream_lost());
        }
    };
    write_reply(&mut data, Ok(EngineReplyValue::PrimaryProcess(info))).await?;
    let (reader, mut writer) = data.into_split();
    let input = process.input();
    let exit = {
        let input = pump_input(reader, input);
        let output = pump_output(&mut process, &mut writer);
        tokio::pin!(input, output);
        let mut input_finished = false;
        loop {
            tokio::select! {
                biased;
                _ = control.read(&mut marker) => return Err(stream_lost()),
                result = &mut input, if !input_finished => { result?; input_finished = true; },
                result = &mut output => break result?,
            }
        }
    };
    engine
        .finish_primary(&request.request_id, Some(exit.clone()))
        .await?;
    let final_frame = AttachOutput::Exit(exit.exit_code).encode();
    tokio::select! {
        biased;
        _ = control.read(&mut marker) => Err(stream_lost()),
        result = write_frame(&mut writer, &final_frame) => result.map_err(Into::into),
    }
}

pub(super) async fn read_bootstrap(
    reader: &mut (impl AsyncRead + Unpin),
) -> EngineResult<Zeroizing<Vec<u8>>> {
    tokio::time::timeout(CONTROL_TIMEOUT, async {
        let size = reader.read_u32_le().await? as usize;
        if size == 0 || size > MAX_BOOTSTRAP_BYTES {
            return Err(EngineError::new(
                EngineErrorCode::FrameTooLarge,
                "invalid primary bootstrap size",
            ));
        }
        let mut bytes = Zeroizing::new(vec![0; size]);
        reader.read_exact(&mut bytes).await?;
        Ok(bytes)
    })
    .await
    .map_err(|_| stream_lost())?
}

pub(super) async fn read_process_frame(
    reader: &mut (impl AsyncRead + Unpin),
) -> EngineResult<Vec<u8>> {
    let mut header = [0; 4];
    // Idle time is unbounded; only an already-started frame has a deadline.
    reader.read_exact(&mut header[..1]).await?;
    tokio::time::timeout(CONTROL_TIMEOUT, async {
        reader.read_exact(&mut header[1..]).await?;
        let size = u32::from_le_bytes(header) as usize;
        if size == 0 || size > MAX_PROCESS_DATA_BYTES + 1 {
            return Err(EngineError::new(
                EngineErrorCode::FrameTooLarge,
                "invalid process frame size",
            ));
        }
        let mut bytes = vec![0; size];
        reader.read_exact(&mut bytes).await?;
        Ok(bytes)
    })
    .await
    .map_err(|_| {
        EngineError::new(
            EngineErrorCode::Transport,
            "incomplete process frame timed out",
        )
    })?
}

async fn pump_input(
    mut reader: impl AsyncRead + Unpin,
    input: Arc<dyn ExecutionProcessInput>,
) -> EngineResult<()> {
    let mut closed = false;
    loop {
        let bytes = read_process_frame(&mut reader).await?;
        let event = AttachInput::parse(&bytes).map_err(|_| {
            EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid process input frame",
            )
        })?;
        match event {
            AttachInput::Data(bytes) if !closed => {
                // Closing guest stdin must not discard its remaining output.
                if input.write_stdin(&bytes).await.is_err() {
                    return Ok(());
                }
            }
            AttachInput::Close if !closed => {
                closed = true;
                if input.close_stdin().await.is_err() {
                    return Ok(());
                }
            }
            _ => {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "process stdin is already closed",
                ))
            }
        }
    }
}

async fn pump_output(
    process: &mut ExecutionProcess,
    writer: &mut (impl AsyncWrite + Unpin),
) -> EngineResult<ExecExit> {
    let mut exit = None;
    while let Some(event) = process.next_event().await.map_err(|_| stream_lost())? {
        match event {
            ExecEvent::Chunk(chunk) if exit.is_none() => {
                for bytes in chunk.data.chunks(MAX_PROCESS_DATA_BYTES) {
                    let event = match chunk.stream {
                        StreamType::Stdout => AttachOutput::Stdout(bytes.to_vec()),
                        StreamType::Stderr => AttachOutput::Stderr(bytes.to_vec()),
                    };
                    write_frame(writer, &event.encode()).await?;
                }
            }
            ExecEvent::FlushAck => {}
            ExecEvent::Exit(result) if exit.is_none() => exit = Some(result),
            _ => {
                return Err(EngineError::new(
                    EngineErrorCode::Internal,
                    "provider sent events after process exit",
                ))
            }
        }
    }
    exit.ok_or_else(stream_lost)
}

fn stream_lost() -> EngineError {
    EngineError::new(
        EngineErrorCode::StreamLost,
        "primary process stream lost; inspect its request and wait for cleanup",
    )
}
