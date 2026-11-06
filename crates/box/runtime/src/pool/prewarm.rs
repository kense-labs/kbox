//! Application-process prewarming for warm-pool VMs.
//!
//! The pool starts the OCI entrypoint over the ordinary streaming exec channel
//! and waits for an explicit readiness marker before publishing the VM as idle.
//! Attaching a client therefore reuses a live process instead of spawning one.

use kbox_core::error::{BoxError, Result};
use kbox_core::exec::{ExecEvent, StreamType};

use crate::grpc::StreamingExec;
use crate::vm::VmManager;

/// Environment switch understood by prewarm-aware workloads.
pub const PREWARM_READY_ENV: &str = "KBOX_PREWARM_READY";
/// Stderr line emitted after the workload has completed application startup.
pub const PREWARM_READY_MARKER: &[u8] = b"kbox-prewarm-ready-v1\n";

const PREWARM_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_PREWARM_OUTPUT_BYTES: usize = 1024 * 1024;

/// A live application process and output produced while waiting for readiness.
pub struct PrewarmedExec {
    stream: StreamingExec,
    pending_stdout: Vec<u8>,
    pending_stderr: Vec<u8>,
}

impl std::fmt::Debug for PrewarmedExec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrewarmedExec")
            .field("pending_stdout_bytes", &self.pending_stdout.len())
            .field("pending_stderr_bytes", &self.pending_stderr.len())
            .finish_non_exhaustive()
    }
}

impl PrewarmedExec {
    /// Take ownership of the live stream and any pre-attach output.
    pub fn into_parts(self) -> (StreamingExec, Vec<u8>, Vec<u8>) {
        (self.stream, self.pending_stdout, self.pending_stderr)
    }
}

fn append_bounded(output: &mut Vec<u8>, chunk: &[u8], stream: StreamType) -> Result<()> {
    if output.len().saturating_add(chunk.len()) > MAX_PREWARM_OUTPUT_BYTES {
        return Err(BoxError::PoolError(format!(
            "prewarmed process produced more than {MAX_PREWARM_OUTPUT_BYTES} bytes on {stream} before readiness"
        )));
    }
    output.extend_from_slice(chunk);
    Ok(())
}

fn remove_ready_marker(stderr: &mut Vec<u8>) -> bool {
    let Some(position) = stderr
        .windows(PREWARM_READY_MARKER.len())
        .position(|window| window == PREWARM_READY_MARKER)
    else {
        return false;
    };
    stderr.drain(position..position + PREWARM_READY_MARKER.len());
    true
}

async fn wait_for_ready(stream: &mut StreamingExec) -> Result<(Vec<u8>, Vec<u8>)> {
    let wait = async {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        loop {
            match stream.next_event().await? {
                Some(ExecEvent::Chunk(chunk)) => {
                    let stream_type = chunk.stream;
                    let output = match stream_type {
                        StreamType::Stdout => &mut stdout,
                        StreamType::Stderr => &mut stderr,
                    };
                    append_bounded(output, &chunk.data, stream_type)?;
                    if stream_type == StreamType::Stderr && remove_ready_marker(&mut stderr) {
                        return Ok((stdout, stderr));
                    }
                }
                Some(ExecEvent::FlushAck) => {}
                Some(ExecEvent::Exit(exit)) => {
                    return Err(BoxError::PoolError(format!(
                        "prewarmed process exited with code {} before readiness",
                        exit.exit_code
                    )));
                }
                None => {
                    return Err(BoxError::PoolError(
                        "prewarmed process closed its exec stream before readiness".to_string(),
                    ));
                }
            }
        }
    };

    tokio::time::timeout(PREWARM_READY_TIMEOUT, wait)
        .await
        .map_err(|_| {
            BoxError::PoolError(format!(
                "prewarmed process did not become ready within {}s; it must honor {PREWARM_READY_ENV}=1 and write the readiness marker to stderr",
                PREWARM_READY_TIMEOUT.as_secs()
            ))
        })?
}

impl VmManager {
    /// Start and verify the image entrypoint while this VM is being prewarmed.
    #[cfg(unix)]
    pub(crate) async fn start_prewarmed_exec(&mut self) -> Result<()> {
        let Some(prewarm) = self.config.pool.prewarm.clone() else {
            return Ok(());
        };
        let image = self.image_config.as_ref().ok_or_else(|| {
            BoxError::PoolError(
                "process prewarm requires the resolved OCI image config".to_string(),
            )
        })?;
        let cmd: Vec<String> = image
            .entrypoint
            .iter()
            .flatten()
            .chain(image.cmd.iter().flatten())
            .cloned()
            .collect();
        if cmd.is_empty() {
            return Err(BoxError::PoolError(
                "process prewarm requires a non-empty OCI entrypoint or cmd".to_string(),
            ));
        }

        // Match OCI main-process semantics: image env/user/workdir are defaults,
        // explicit pool settings override them, and the readiness switch cannot
        // be disabled by an identically named image or caller variable.
        let mut merged_env: std::collections::BTreeMap<String, String> =
            image.env.iter().cloned().collect();
        merged_env.extend(prewarm.env);
        merged_env.insert(PREWARM_READY_ENV.to_string(), "1".to_string());
        let env = merged_env
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        let request = kbox_core::exec::ExecRequest {
            request_id: None,
            cmd,
            timeout_ns: u64::MAX,
            env,
            working_dir: prewarm.workdir.or_else(|| image.working_dir.clone()),
            rootfs: None,
            stdin: None,
            stdin_streaming: true,
            user: prewarm.user.or_else(|| image.user.clone()),
            streaming: true,
        };
        let client = self.exec_client.as_ref().ok_or_else(|| {
            BoxError::PoolError("process prewarm requires an authenticated exec client".to_string())
        })?;
        let mut stream = client.exec_stream(&request).await?;
        let (pending_stdout, pending_stderr) = wait_for_ready(&mut stream).await?;
        self.prewarmed_exec = Some(PrewarmedExec {
            stream,
            pending_stdout,
            pending_stderr,
        });
        Ok(())
    }

    #[cfg(not(unix))]
    pub(crate) async fn start_prewarmed_exec(&mut self) -> Result<()> {
        if self.config.pool.prewarm.is_some() {
            return Err(BoxError::PoolError(
                "process prewarm is not supported on this host".to_string(),
            ));
        }
        Ok(())
    }

    /// Take the initialized process for a one-shot client attachment.
    pub fn take_prewarmed_exec(&mut self) -> Option<PrewarmedExec> {
        self.prewarmed_exec.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_marker_is_removed_across_output_boundaries() {
        let mut stderr = b"starting\nkbox-prewarm-".to_vec();
        assert!(!remove_ready_marker(&mut stderr));
        stderr.extend_from_slice(b"ready-v1\nafter\n");
        assert!(remove_ready_marker(&mut stderr));
        assert_eq!(stderr, b"starting\nafter\n");
        assert!(!remove_ready_marker(&mut stderr));
    }

    #[test]
    fn prewarm_output_is_bounded() {
        let mut output = vec![0; MAX_PREWARM_OUTPUT_BYTES];
        let error = append_bounded(&mut output, &[1], StreamType::Stderr).unwrap_err();
        assert!(error.to_string().contains("more than"));
        append_bounded(&mut Vec::new(), b"ready", StreamType::Stdout).unwrap();
    }

    #[test]
    fn prewarm_environment_is_not_serialized_or_debug_printed() {
        let config = kbox_core::config::PrewarmConfig {
            env: vec![("SECRET_KEY".to_string(), "secret-value".to_string())],
            ..Default::default()
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let debug = format!("{config:?}");
        assert!(!serialized.contains("secret-value"));
        assert!(!serialized.contains("SECRET_KEY"));
        assert!(debug.contains("SECRET_KEY"));
        assert!(!debug.contains("secret-value"));
    }
}
