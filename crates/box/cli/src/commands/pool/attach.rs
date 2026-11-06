//! `kbox pool attach` client and daemon-side live stdio relay.

use clap::Parser;

use kbox_runtime::pool::{
    attach_client, AttachInput, AttachOutput, PoolAttachRequest, PoolAttachResponse,
    PoolClientAttach,
};

#[derive(Parser)]
pub struct PoolAttachArgs {
    /// Unix socket of the `pool start --prewarm` daemon.
    #[arg(long, default_value = super::DEFAULT_SOCKET)]
    pub socket: String,

    /// Image to attach to (defaults to the daemon's --image).
    #[arg(long)]
    pub image: Option<String>,

    /// Volume key override. By default the daemon's prewarm mounts are selected.
    #[arg(long = "volume", short = 'v')]
    pub volumes: Vec<String>,

    /// vCPU key for the prewarmed pool.
    #[arg(long, default_value_t = super::DEFAULT_POOL_VCPUS)]
    pub cpus: u32,

    /// Memory key for the prewarmed pool.
    #[arg(long, default_value = super::DEFAULT_POOL_MEMORY)]
    pub memory: String,
}

#[cfg(not(windows))]
pub async fn execute(args: PoolAttachArgs) -> Result<(), Box<dyn std::error::Error>> {
    let memory_mb = crate::output::parse_memory(&args.memory)
        .map_err(|error| format!("Invalid --memory: {error}"))?;
    let exit_code = attach_client(PoolClientAttach {
        socket: args.socket,
        image: args.image,
        volumes: args.volumes,
        vcpus: args.cpus,
        memory_mb,
    })
    .await?;
    std::process::exit(exit_code);
}

#[cfg(windows)]
pub async fn execute(_args: PoolAttachArgs) -> Result<(), Box<dyn std::error::Error>> {
    Err("`pool attach` is not supported on Windows".into())
}

#[cfg(not(windows))]
fn invalid_data(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

/// Acquire one process-prewarmed VM and relay it until process exit or client
/// disconnect. The VM is always destroyed; replenishment happens in WarmPool.
#[cfg(not(windows))]
pub async fn handle(
    registry: &super::PoolRegistry,
    request: PoolAttachRequest,
    stream: &mut tokio::net::UnixStream,
) -> std::io::Result<()> {
    use kbox_runtime::pool::client::write_frame;

    let started = std::time::Instant::now();
    if registry.prewarm.is_none() {
        let payload = serde_json::to_vec(&PoolAttachResponse {
            error: Some(
                "process prewarm is disabled; restart the daemon with `kbox pool start --prewarm`"
                    .to_string(),
            ),
        })
        .map_err(invalid_data)?;
        return write_frame(stream, &payload).await;
    }

    let acquired = async {
        let image = registry.resolve_image(request.image.clone()).ok_or_else(|| {
            "no image: pass --image or start the daemon with --image".to_string()
        })?;
        let volumes = if request.volumes.is_empty() {
            registry.default_volumes.clone()
        } else {
            request.volumes.clone()
        };
        let key = super::PoolKey {
            image: image.clone(),
            volumes,
            vcpus: request.vcpus.unwrap_or(super::DEFAULT_POOL_VCPUS),
            memory_mb: request.memory_mb.unwrap_or(super::DEFAULT_POOL_MEMORY_MB),
        };
        let entry = registry
            .get_or_create(key)
            .await
            .map_err(|error| format!("pool for {image}: {error}"))?;
        let permit = entry
            .sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "pool semaphore closed".to_string())?;
        let mut vm = entry
            .pool
            .acquire()
            .await
            .map_err(|error| format!("acquire failed: {error}"))?;
        let Some(prewarmed) = vm.take_prewarmed_exec() else {
            let _ = super::destroy_vm_or_reap(&mut vm).await;
            drop(permit);
            return Err(
                "selected VM has no prewarmed process; restart the daemon with `kbox pool start --prewarm`"
                    .to_string(),
            );
        };
        Ok::<_, String>((vm, permit, prewarmed))
    }
    .await;

    let (mut vm, permit, prewarmed) = match acquired {
        Ok(acquired) => acquired,
        Err(error) => {
            let payload = serde_json::to_vec(&PoolAttachResponse { error: Some(error) })
                .map_err(invalid_data)?;
            return write_frame(stream, &payload).await;
        }
    };

    let ack = serde_json::to_vec(&PoolAttachResponse { error: None }).map_err(invalid_data)?;
    if let Err(error) = write_frame(stream, &ack).await {
        let _ = super::destroy_vm_or_reap(&mut vm).await;
        drop(permit);
        return Err(error);
    }

    tracing::debug!(
        attach_setup_us = started.elapsed().as_micros(),
        "process-prewarmed VM attached"
    );
    let relay_result = relay(stream, prewarmed).await;
    // Destruction is authoritative even if relaying fails or the client drops.
    if let Err(error) = super::destroy_vm_or_reap(&mut vm).await {
        tracing::warn!(%error, "failed to destroy process-prewarmed VM after attach");
    }
    drop(permit);
    relay_result
}

#[cfg(not(windows))]
async fn relay(
    stream: &mut tokio::net::UnixStream,
    prewarmed: kbox_runtime::pool::PrewarmedExec,
) -> std::io::Result<()> {
    use kbox_core::exec::{ExecEvent, StreamType};
    use kbox_runtime::pool::{client::write_frame, AttachFrameReader};

    let (mut exec, pending_stdout, pending_stderr) = prewarmed.into_parts();
    for event in [
        AttachOutput::Stdout(pending_stdout),
        AttachOutput::Stderr(pending_stderr),
    ] {
        let empty = matches!(&event, AttachOutput::Stdout(data) | AttachOutput::Stderr(data) if data.is_empty());
        if !empty {
            write_frame(stream, &event.encode()).await?;
        }
    }

    let (reader, mut writer) = stream.split();
    let mut frames = AttachFrameReader::new(reader);
    let input = exec.input();
    let mut client_open = true;
    loop {
        tokio::select! {
            frame = frames.read_frame(), if client_open => {
                let frame = match frame {
                    Ok(frame) => frame,
                    Err(_) => {
                        let _ = input.cancel().await;
                        return Ok(());
                    }
                };
                match AttachInput::parse(&frame).map_err(invalid_data)? {
                    AttachInput::Data(data) => input.write_stdin(&data).await.map_err(invalid_data)?,
                    AttachInput::Close => {
                        client_open = false;
                        input.close_stdin().await.map_err(invalid_data)?;
                    }
                }
            }
            event = exec.next_event() => {
                match event.map_err(invalid_data)? {
                    Some(ExecEvent::Chunk(chunk)) => {
                        let output = match chunk.stream {
                            StreamType::Stdout => AttachOutput::Stdout(chunk.data),
                            StreamType::Stderr => AttachOutput::Stderr(chunk.data),
                        };
                        write_frame(&mut writer, &output.encode()).await?;
                    }
                    Some(ExecEvent::FlushAck) => {}
                    Some(ExecEvent::Exit(exit)) => {
                        write_frame(&mut writer, &AttachOutput::Exit(exit.exit_code).encode()).await?;
                        return Ok(());
                    }
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "prewarmed exec stream closed without an exit event",
                        ));
                    }
                }
            }
        }
    }
}
