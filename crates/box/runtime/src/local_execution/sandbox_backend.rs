//! Spec050 fused-sandbox `LocalExecutionBackend`.
//!
//! Pins new records to [`ManagedRuntimeRoute::Sandbox`] and drives
//! create/start/inspect/exec/kill against the fused sandbox engine daemon.
//! Missing daemon fails closed (typed Unavailable). Never falls back to
//! BoxVm/OciSdk or the MicroVM guest exec socket. Start path may
//! spawn-on-demand via `ensure_daemon`.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use kbox_core::secret::{
    validate_environment_variable_name, SecretEnvironmentBinding, SECRET_ENVIRONMENT_MANIFEST,
    SECRET_GUEST_ROOT,
};
use kbox_core::{
    pty::PtyRequest, ExecOutput, ExecRequest, ExecutionManagerError, ExecutionManagerResult,
    ExecutionProcess, ExecutionState, FileOp, FileRequest, FileResponse, FilesystemEntry,
    FilesystemEntryKind, FilesystemOp, FilesystemRequest, FilesystemResponse, KillOutcome,
};
use zeroize::Zeroizing;

use super::{
    LocalExecutionBackend, LocalExecutionHandle, LocalExecutionObservation,
    LocalExecutionResourcePlan, LocalExecutionTermination,
};
use crate::{BoxRecord, ManagedExecutionMetadata, ManagedRuntimeRoute};

/// Production adapter that routes isolation-graded Sandbox work to the fused
/// sandbox engine library surface.
#[derive(Debug, Clone)]
pub struct SandboxLocalExecutionBackend {
    home_dir: PathBuf,
}

impl SandboxLocalExecutionBackend {
    /// Construct an adapter keyed by the Box home directory.
    pub fn new(home_dir: impl Into<PathBuf>) -> Self {
        Self {
            home_dir: home_dir.into(),
        }
    }

    /// Box home used for engine state/socket layout.
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    pub(super) fn metadata<'a>(
        &self,
        record: &'a BoxRecord,
    ) -> ExecutionManagerResult<&'a ManagedExecutionMetadata> {
        let expected_box_dir = self.home_dir.join("boxes").join(&record.id);
        if record.box_dir != expected_box_dir {
            return Err(ExecutionManagerError::Internal(format!(
                "sandbox execution {} has an unexpected host directory {}",
                record.id,
                record.box_dir.display()
            )));
        }
        let metadata = record.managed_execution.as_ref().ok_or_else(|| {
            ExecutionManagerError::Internal(format!(
                "sandbox execution {} lost managed lifecycle metadata",
                record.id
            ))
        })?;
        metadata
            .validate()
            .map_err(|error| ExecutionManagerError::Internal(error.to_string()))?;
        let resolved = super::router::resolved_runtime_route(record)?;
        if resolved != ManagedRuntimeRoute::Sandbox {
            return Err(ExecutionManagerError::Internal(format!(
                "sandbox backend refused execution {}: pinned to {:?}, not Sandbox",
                record.id, resolved
            )));
        }
        Ok(metadata)
    }

    fn persist_container_id(
        &self,
        record: &BoxRecord,
        id: &str,
    ) -> ExecutionManagerResult<PathBuf> {
        kbox_sandbox_engine::write_container_id(&record.box_dir, id).map_err(|error| {
            ExecutionManagerError::Internal(format!(
                "sandbox could not persist container id: {error}"
            ))
        })
    }

    pub(super) fn load_container_id(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<Option<String>> {
        kbox_sandbox_engine::read_container_id(&record.box_dir).map_err(|error| {
            ExecutionManagerError::Internal(format!("sandbox could not read container id: {error}"))
        })
    }

    /// Bind a Docker HTTP client to the Box-home daemon socket.
    ///
    /// When `allow_spawn` is true (production start path), missing sockets may
    /// be created via `ensure_daemon` if `KBOX_SANDBOX_AUTOSTART` allows it.
    /// Inspect/kill keep spawn off so hermetic and post-mortem paths stay
    /// daemon-optional and fail closed without launching a VM.
    #[cfg(unix)]
    pub(super) fn client(
        &self,
        allow_spawn: bool,
    ) -> ExecutionManagerResult<kbox_sandbox_engine::DockerHttpClient> {
        let socket = if allow_spawn {
            kbox_sandbox_engine::ensure_daemon(&self.home_dir)
                .map_err(ExecutionManagerError::Unavailable)?
        } else {
            let socket = kbox_sandbox_engine::docker_socket_path(&self.home_dir);
            if !socket.exists() {
                return Err(ExecutionManagerError::Unavailable(format!(
                    "sandbox engine daemon unreachable at {}",
                    socket.display()
                )));
            }
            socket
        };
        let client = kbox_sandbox_engine::DockerHttpClient::new(socket);
        client
            .ensure_reachable()
            .map_err(ExecutionManagerError::Unavailable)?;
        Ok(client)
    }

    fn handle_for(
        &self,
        record: &BoxRecord,
        socket: PathBuf,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> LocalExecutionHandle {
        LocalExecutionHandle {
            started_at,
            // Guest/container PIDs are not host identities for this isolation.
            pid: None,
            pid_start_time: None,
            // Durable evidence path: daemon socket used for this generation.
            // Container id is persisted under box_dir separately.
            exec_socket_path: socket,
            console_log: if record.console_log.as_os_str().is_empty() {
                // Match CLI foreground tails: `logs/console.log`.
                record.box_dir.join("logs").join("console.log")
            } else {
                record.console_log.clone()
            },
            anonymous_volumes: record.anonymous_volumes.clone(),
            oci_runtime: None,
        }
    }

    fn map_env(record: &BoxRecord) -> ExecutionManagerResult<SandboxProcessEnv> {
        let mut durable: Vec<(String, String)> = record
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        // Fused-engine containers have no guest-init Secret consumer. Expand the
        // non-sensitive manifest into process-only Env from host bind sources.
        // Durable create Env keeps non-secret keys only; Secret values ride the
        // engine ephemeral-env overlay so containers.json never stores them.
        let ephemeral = take_secret_environment(&mut durable, &record.volumes)?;
        durable.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(SandboxProcessEnv { durable, ephemeral })
    }
}

/// Durable create Env plus process-only Secret Env for the fused engine.
struct SandboxProcessEnv {
    durable: Vec<(String, String)>,
    ephemeral: Vec<(String, String)>,
}

impl std::fmt::Debug for SandboxProcessEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxProcessEnv")
            .field("durable", &self.durable)
            .field("ephemeral_len", &self.ephemeral.len())
            .finish()
    }
}

/// Replace `K_BOX_SECRET_ENV_V1` with values read from matching read-only binds.
///
/// Returns Secret pairs for the process-only engine overlay. Fail closed on
/// missing/ambiguous binds or invalid material. Never logs secret bytes.
fn take_secret_environment(
    env: &mut Vec<(String, String)>,
    volumes: &[String],
) -> ExecutionManagerResult<Vec<(String, String)>> {
    let manifests: Vec<usize> = env
        .iter()
        .enumerate()
        .filter(|(_, (key, _))| key == SECRET_ENVIRONMENT_MANIFEST)
        .map(|(index, _)| index)
        .collect();
    if manifests.is_empty() {
        return Ok(Vec::new());
    }
    if manifests.len() != 1 {
        return Err(ExecutionManagerError::InvalidRequest(
            "duplicate Box Secret environment manifests".into(),
        ));
    }
    let (_, encoded) = env.remove(manifests[0]);
    let encoded = Zeroizing::new(encoded);
    let bindings: Vec<SecretEnvironmentBinding> = serde_json::from_str(&encoded).map_err(|_| {
        ExecutionManagerError::InvalidRequest(
            "Box Secret environment manifest is not valid version-1 JSON".into(),
        )
    })?;
    if bindings.is_empty() || bindings.len() > 128 {
        return Err(ExecutionManagerError::InvalidRequest(
            "Box Secret environment manifest has an invalid binding count".into(),
        ));
    }

    let mut variables: BTreeSet<String> = env.iter().map(|(key, _)| key.clone()).collect();
    let mut paths = BTreeSet::new();
    let mut additions = Vec::with_capacity(bindings.len());
    for binding in bindings {
        binding.validate().map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!(
                "Box Secret environment binding is invalid: {error}"
            ))
        })?;
        validate_environment_variable_name(&binding.variable).map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!(
                "Box Secret environment variable is invalid: {error}"
            ))
        })?;
        if !variables.insert(binding.variable.clone()) || !paths.insert(binding.path.clone()) {
            return Err(ExecutionManagerError::InvalidRequest(
                "Box Secret environment binding is duplicate or conflicts with existing Env".into(),
            ));
        }
        if !binding.path.starts_with(&format!("{SECRET_GUEST_ROOT}/")) {
            return Err(ExecutionManagerError::InvalidRequest(
                "Box Secret environment file escaped the reserved guest directory".into(),
            ));
        }
        let host = host_bind_for_guest(volumes, &binding.path)?;
        let value = read_secret_bind_value(&host)?;
        additions.push((binding.variable, value));
    }
    Ok(additions)
}

fn host_bind_for_guest(volumes: &[String], guest_path: &str) -> ExecutionManagerResult<PathBuf> {
    let mut matches = Vec::new();
    for spec in volumes {
        let Some((mount, mode)) = spec.rsplit_once(':') else {
            continue;
        };
        if mode != "ro" && mode != "rw" {
            continue;
        }
        let Some((host, guest)) = mount.rsplit_once(':') else {
            continue;
        };
        if guest != guest_path {
            continue;
        }
        if mode != "ro" {
            return Err(ExecutionManagerError::InvalidRequest(
                "Box Secret environment bind must be read-only".into(),
            ));
        }
        matches.push(PathBuf::from(host));
    }
    if matches.len() != 1 {
        return Err(ExecutionManagerError::InvalidRequest(
            "Box Secret environment binding has missing or ambiguous file authority".into(),
        ));
    }
    Ok(matches.remove(0))
}

fn read_secret_bind_value(path: &Path) -> ExecutionManagerResult<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|error| {
                ExecutionManagerError::Unavailable(format!(
                    "Box Secret environment file is unavailable: {error}"
                ))
            })?;
        let metadata = file.metadata().map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "Box Secret environment file could not be inspected: {error}"
            ))
        })?;
        if !metadata.file_type().is_file()
            || metadata.file_type().is_symlink()
            || metadata.nlink() != 1
            || metadata.len() == 0
            || metadata.len() > 1024 * 1024
            || metadata.mode() & 0o777 != 0o400
        {
            return Err(ExecutionManagerError::InvalidRequest(
                "Box Secret environment file violates regular-file, size, or mode contract".into(),
            ));
        }
        let mut bytes = Zeroizing::new(vec![0_u8; metadata.len() as usize]);
        (&file).read_exact(bytes.as_mut_slice()).map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "Box Secret environment file could not be read: {error}"
            ))
        })?;
        let value = std::str::from_utf8(bytes.as_slice()).map_err(|_| {
            ExecutionManagerError::InvalidRequest(
                "Box Secret environment material must be UTF-8 without NUL bytes".into(),
            )
        })?;
        if value.contains('\0') {
            return Err(ExecutionManagerError::InvalidRequest(
                "Box Secret environment material must be UTF-8 without NUL bytes".into(),
            ));
        }
        Ok(value.to_owned())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(ExecutionManagerError::Unavailable(
            "Box Secret environment projection requires a Unix host".into(),
        ))
    }
}

/// DNS aliases for Compose service discovery on a user-defined Docker network.
///
/// Container create names use the execution id; aliases carry the Compose
/// service name (`com.kbox.compose.service`) and the `{project}-{service}` box
/// name so `nslookup web` / `ping web` can work when engine DNS is live.
fn compose_network_aliases(record: &BoxRecord) -> Vec<String> {
    let mut aliases = std::collections::BTreeSet::new();
    if let Some(service) = record.labels.get("com.kbox.compose.service") {
        if !service.is_empty() {
            aliases.insert(service.clone());
        }
    }
    if !record.name.is_empty() {
        aliases.insert(record.name.clone());
    }
    aliases.into_iter().collect()
}

/// Ensure fused-engine Docker networks exist for Sandbox compose (not NetworkStore).
#[cfg(unix)]
pub fn ensure_sandbox_docker_networks(
    home: &Path,
    names: &[String],
) -> Result<(), ExecutionManagerError> {
    let socket =
        kbox_sandbox_engine::ensure_daemon(home).map_err(ExecutionManagerError::Unavailable)?;
    let client = kbox_sandbox_engine::DockerHttpClient::new(socket);
    for name in names {
        client.network_ensure(name).map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox network ensure failed for {name}: {error}"
            ))
        })?;
    }
    Ok(())
}

/// Remove fused-engine Docker networks created for Sandbox compose.
#[cfg(unix)]
pub fn remove_sandbox_docker_networks(
    home: &Path,
    names: &[String],
) -> Result<(), ExecutionManagerError> {
    let socket = kbox_sandbox_engine::docker_socket_path(home);
    if !socket.exists() {
        return Ok(());
    }
    let client = kbox_sandbox_engine::DockerHttpClient::new(socket);
    for name in names.iter().rev() {
        client.network_remove(name).map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox network remove failed for {name}: {error}"
            ))
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn ensure_sandbox_docker_networks(
    _home: &Path,
    _names: &[String],
) -> Result<(), ExecutionManagerError> {
    Err(ExecutionManagerError::Unavailable(
        "sandbox engine is unavailable on this host".into(),
    ))
}

#[cfg(not(unix))]
pub fn remove_sandbox_docker_networks(
    _home: &Path,
    _names: &[String],
) -> Result<(), ExecutionManagerError> {
    Ok(())
}

#[async_trait]
impl LocalExecutionBackend for SandboxLocalExecutionBackend {
    fn route_for_create(&self, _record: &BoxRecord) -> ExecutionManagerResult<ManagedRuntimeRoute> {
        Ok(ManagedRuntimeRoute::Sandbox)
    }

    async fn preflight_launch(&self) -> ExecutionManagerResult<()> {
        // Layout discovery only — daemon-optional. Spawn stays on the start
        // path via `client()` → `ensure_daemon` (gated by KBOX_SANDBOX_AUTOSTART).
        let caps = kbox_sandbox_engine::discover_capabilities(&self.home_dir);
        tracing::debug!(
            socket = %caps.socket_path.display(),
            reachable = caps.daemon_reachable,
            "sandbox engine capability probe"
        );
        Ok(())
    }

    async fn preflight(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.metadata(record)?;
        Ok(())
    }

    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionResourcePlan> {
        self.metadata(record)?;
        Ok(LocalExecutionResourcePlan::default())
    }

    async fn start(&self, record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            // Production start path: ensure/spawn daemon when autostart allows.
            let client = self.client(true)?;
            let SandboxProcessEnv { durable, ephemeral } = Self::map_env(record)?;
            let workdir = record.workdir.as_deref();
            // Docker names accept UUID form; pin to the Box execution id.
            // Box may have pulled into its own store; the fused daemon has a
            // separate image store — pull only when create reports missing image
            // so hermetic fake daemons (no pull endpoint) stay green.
            let binds = record.volumes.clone();
            let ports = record.port_map.clone();
            // Bridge → fused-engine Docker project network (never MicroVM
            // NetworkStore). Host/None map to Engine API modes. TSI leaves the
            // daemon default (builtin bridge) so minimal smokes stay quiet.
            let (docker_network, ensure_user_network) = match &record.network_mode {
                kbox_core::NetworkMode::Bridge { network } => (Some(network.as_str()), true),
                kbox_core::NetworkMode::Host => (Some("host"), false),
                kbox_core::NetworkMode::None => (Some("none"), false),
                kbox_core::NetworkMode::Tsi => (None, false),
            };
            if ensure_user_network {
                if let Some(network) = docker_network {
                    client.network_ensure(network).map_err(|error| {
                        ExecutionManagerError::Unavailable(format!(
                            "sandbox network ensure failed for {network}: {error}"
                        ))
                    })?;
                }
            }
            let aliases = compose_network_aliases(record);
            let create = |client: &kbox_sandbox_engine::DockerHttpClient| {
                client.container_create_with_network(
                    &record.image,
                    &record.cmd,
                    &durable,
                    workdir,
                    Some(record.id.as_str()),
                    &binds,
                    &ports,
                    docker_network,
                    &aliases,
                )
            };
            let id = match create(&client) {
                Ok(id) => id,
                Err(error)
                    if error
                        .to_string()
                        .to_ascii_lowercase()
                        .contains("no such image") =>
                {
                    client.image_pull(&record.image).map_err(|pull_error| {
                        ExecutionManagerError::Unavailable(format!(
                            "sandbox image pull failed for {}: {pull_error}",
                            record.image
                        ))
                    })?;
                    create(&client).map_err(|retry_error| {
                        ExecutionManagerError::Unavailable(format!(
                            "sandbox container create failed: {retry_error}"
                        ))
                    })?
                }
                Err(error) => {
                    return Err(ExecutionManagerError::Unavailable(format!(
                        "sandbox container create failed: {error}"
                    )));
                }
            };
            if !ephemeral.is_empty() {
                if let Err(error) = client.container_set_ephemeral_env(&id, &ephemeral) {
                    let _ = client.container_remove(&id, true);
                    return Err(ExecutionManagerError::Unavailable(format!(
                        "sandbox ephemeral secret Env attach failed: {error}"
                    )));
                }
            }
            if let Err(error) = client.container_start(&id) {
                let _ = client.container_remove(&id, true);
                return Err(ExecutionManagerError::Unavailable(format!(
                    "sandbox container start failed: {error}"
                )));
            }
            let id_path = self.persist_container_id(record, &id)?;
            let handle = self.handle_for(
                record,
                client.socket_path().to_path_buf(),
                chrono::Utc::now(),
            );
            // Stream container logs into the console path the CLI foreground tails.
            let log_path = handle.console_log.clone();
            let socket = client.socket_path().to_path_buf();
            let log_id = id.clone();
            let _ = std::thread::Builder::new()
                .name("sandbox-logs".into())
                .spawn(move || {
                    let client = kbox_sandbox_engine::DockerHttpClient::new(socket);
                    if let Err(error) = client.follow_logs_to_file(&log_id, &log_path) {
                        tracing::debug!(
                            container = %log_id,
                            path = %log_path.display(),
                            %error,
                            "sandbox log follow ended"
                        );
                    }
                });
            tracing::debug!(
                execution = %record.id,
                container = %id,
                id_path = %id_path.display(),
                "sandbox container started"
            );
            Ok(handle)
        }
    }

    async fn inspect(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionObservation> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            let Some(id) = self.load_container_id(record)? else {
                // Fail closed: do not invent Created/Stopped from absence.
                return Err(ExecutionManagerError::Unavailable(format!(
                    "sandbox container id missing for execution {}",
                    record.id
                )));
            };
            let client = self.client(false)?;
            let inspect = match client.container_inspect(&id) {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(LocalExecutionObservation {
                        state: ExecutionState::Stopped,
                        handle: None,
                        exit_code: None,
                    });
                }
                Err(error) => {
                    return Err(ExecutionManagerError::Unavailable(format!(
                        "sandbox container inspect failed: {error}"
                    )));
                }
            };
            let running = inspect["State"]["Running"].as_bool().unwrap_or(false);
            let paused = inspect["State"]["Paused"].as_bool().unwrap_or(false);
            let exit_code = inspect["State"]["ExitCode"].as_i64().map(|v| v as i32);
            let state = if paused {
                ExecutionState::Paused
            } else if running {
                ExecutionState::Running
            } else {
                ExecutionState::Stopped
            };
            let handle = if matches!(state, ExecutionState::Running | ExecutionState::Paused) {
                Some(self.handle_for(
                    record,
                    client.socket_path().to_path_buf(),
                    record.started_at.unwrap_or_else(chrono::Utc::now),
                ))
            } else {
                None
            };
            Ok(LocalExecutionObservation {
                state,
                handle,
                exit_code,
            })
        }
    }

    async fn pause(
        &self,
        record: &BoxRecord,
        _keep_memory: bool,
    ) -> ExecutionManagerResult<LocalExecutionHandle> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            let id = self.load_container_id(record)?.ok_or_else(|| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox container id missing for execution {}",
                    record.id
                ))
            })?;
            let client = self.client(false)?;
            client.container_pause(&id).map_err(|error| {
                ExecutionManagerError::Unavailable(format!("sandbox pause failed: {error}"))
            })?;
            Ok(self.handle_for(
                record,
                client.socket_path().to_path_buf(),
                record.started_at.unwrap_or_else(chrono::Utc::now),
            ))
        }
    }

    async fn resume(&self, record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            let id = self.load_container_id(record)?.ok_or_else(|| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox container id missing for execution {}",
                    record.id
                ))
            })?;
            let client = self.client(false)?;
            client.container_unpause(&id).map_err(|error| {
                ExecutionManagerError::Unavailable(format!("sandbox resume failed: {error}"))
            })?;
            Ok(self.handle_for(
                record,
                client.socket_path().to_path_buf(),
                record.started_at.unwrap_or_else(chrono::Utc::now),
            ))
        }
    }

    async fn kill(&self, record: &BoxRecord) -> ExecutionManagerResult<KillOutcome> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            let Some(id) = self.load_container_id(record)? else {
                return Ok(KillOutcome::AlreadyStopped);
            };
            let client = self.client(false)?;
            match client.container_kill(&id, "SIGKILL") {
                Ok(()) => {
                    let _ = client.container_remove(&id, true);
                    Ok(KillOutcome::Killed)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let _ = client.container_remove(&id, true);
                    Ok(KillOutcome::AlreadyStopped)
                }
                Err(error) => Err(ExecutionManagerError::Unavailable(format!(
                    "sandbox container kill failed: {error}"
                ))),
            }
        }
    }

    async fn kill_with_status(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<LocalExecutionTermination> {
        let outcome = self.kill(record).await?;
        Ok(LocalExecutionTermination {
            outcome,
            exit_code: None,
        })
    }

    async fn stop_for_restart(
        &self,
        record: &BoxRecord,
        _timeout_secs: Option<u64>,
    ) -> ExecutionManagerResult<KillOutcome> {
        self.kill(record).await
    }

    async fn execute(
        &self,
        record: &BoxRecord,
        request: ExecRequest,
    ) -> ExecutionManagerResult<ExecOutput> {
        #[cfg(not(unix))]
        {
            let _ = (record, request);
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            if request.streaming {
                return Err(ExecutionManagerError::Unavailable(
                    "sandbox captured exec does not support streaming mode".into(),
                ));
            }
            if request.rootfs.as_ref().is_some_and(|path| !path.is_empty()) {
                return Err(ExecutionManagerError::Unavailable(
                    "sandbox exec does not support guest rootfs chroot".into(),
                ));
            }
            // Captured one-shot accepts a finite stdin buffer. Live streaming
            // stdin stays on `start_process` (`kbox exec -i`).
            if request.stdin_streaming {
                return Err(ExecutionManagerError::Unavailable(
                    "sandbox captured exec does not support streaming stdin; \
                     use streaming (`kbox exec -i`) or PTY (`kbox exec -t`)"
                        .into(),
                ));
            }
            if request.cmd.is_empty() {
                return Err(ExecutionManagerError::InvalidRequest(
                    "sandbox exec requires a non-empty command".into(),
                ));
            }
            let stdin = request.stdin.as_deref().unwrap_or(&[]);
            if stdin.len() > kbox_sandbox_engine::DockerHttpClient::MAX_CAPTURED_EXEC_STDIN_BYTES {
                return Err(ExecutionManagerError::InvalidRequest(format!(
                    "sandbox captured exec stdin is {} bytes; max is {}",
                    stdin.len(),
                    kbox_sandbox_engine::DockerHttpClient::MAX_CAPTURED_EXEC_STDIN_BYTES
                )));
            }
            let id = self.load_container_id(record)?.ok_or_else(|| {
                ExecutionManagerError::Unavailable(format!(
                    "sandbox container id missing for execution {}",
                    record.id
                ))
            })?;
            // Fail closed: never spawn a daemon just for exec, and never fall
            // through to the MicroVM guest exec socket.
            let client = self.client(false)?;
            let env = if request.env.is_empty() {
                None
            } else {
                Some(request.env.as_slice())
            };
            let stdin = (!stdin.is_empty()).then_some(stdin);
            let (exit_code, stdout, stderr) = client
                .container_exec(
                    &id,
                    &request.cmd,
                    env,
                    request.user.as_deref(),
                    request.working_dir.as_deref(),
                    stdin,
                )
                .map_err(|error| {
                    ExecutionManagerError::Unavailable(format!(
                        "sandbox container exec failed: {error}"
                    ))
                })?;
            Ok(ExecOutput {
                stdout,
                stderr,
                exit_code,
                truncated: false,
            })
        }
    }

    async fn start_process(
        &self,
        record: &BoxRecord,
        request: ExecRequest,
    ) -> ExecutionManagerResult<ExecutionProcess> {
        super::sandbox_session::start_process(self, record, request).await
    }

    async fn start_pty(
        &self,
        record: &BoxRecord,
        request: PtyRequest,
    ) -> ExecutionManagerResult<ExecutionProcess> {
        super::sandbox_session::start_pty(self, record, request).await
    }

    async fn transfer_file(
        &self,
        record: &BoxRecord,
        request: FileRequest,
    ) -> ExecutionManagerResult<FileResponse> {
        #[cfg(not(unix))]
        {
            let _ = (record, request);
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            // Validate untrusted input before any daemon contact.
            let upload = match request.op {
                FileOp::Upload => Some(PreparedUpload::new(&request)?),
                FileOp::Download => None,
            };
            let id = self.require_container_id(record)?;
            // Fail closed: file I/O never spawns a daemon and never reaches
            // for the MicroVM guest exec socket.
            let client = self.client(false)?;
            run_blocking(move || match upload {
                Some(upload) => sandbox_upload(&client, &id, &upload),
                None => sandbox_download(&client, &id, &request),
            })
            .await
        }
    }

    async fn filesystem(
        &self,
        record: &BoxRecord,
        request: FilesystemRequest,
    ) -> ExecutionManagerResult<FilesystemResponse> {
        #[cfg(not(unix))]
        {
            let _ = (record, request);
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            // Only read-only Stat is wired through the archive API. Mutating
            // ops are refused rather than approximated through exec.
            if request.op != FilesystemOp::Stat {
                return Err(ExecutionManagerError::Unavailable(format!(
                    "sandbox filesystem supports only Stat; {:?} is not implemented",
                    request.op
                )));
            }
            if request.path.is_empty() {
                return Err(ExecutionManagerError::InvalidRequest(
                    "sandbox filesystem stat requires a non-empty path".into(),
                ));
            }
            let id = self.require_container_id(record)?;
            let client = self.client(false)?;
            run_blocking(move || sandbox_stat(&client, &id, &request.path)).await
        }
    }
}

impl SandboxLocalExecutionBackend {
    /// Archive the whole live container rootfs as one tar (`kbox export` /
    /// `kbox commit` for a running Sandbox box).
    ///
    /// Reuses the daemon archive API (`container_get_archive` on `/`): the
    /// guest archives the root as `./…` with `/proc` and `/sys` empty. Sandbox
    /// records have no host PID or guest exec socket, so this never touches
    /// either. The daemon must already be running (no spawn), and the archive
    /// is bounded by [`kbox_sandbox_engine::DockerHttpClient::MAX_ARCHIVE_BYTES`].
    pub async fn export_rootfs_archive(
        &self,
        record: &BoxRecord,
    ) -> ExecutionManagerResult<Vec<u8>> {
        #[cfg(not(unix))]
        {
            let _ = record;
            return Err(ExecutionManagerError::Unavailable(
                "sandbox engine is unavailable on this host".into(),
            ));
        }
        #[cfg(unix)]
        {
            self.metadata(record)?;
            let id = self.require_container_id(record)?;
            let client = self.client(false)?;
            run_blocking(move || sandbox_export_rootfs(&client, &id)).await
        }
    }
}

#[cfg(unix)]
fn sandbox_export_rootfs(
    client: &kbox_sandbox_engine::DockerHttpClient,
    id: &str,
) -> ExecutionManagerResult<Vec<u8>> {
    let limit = kbox_sandbox_engine::DockerHttpClient::MAX_ARCHIVE_BYTES;
    let (stat, archive) = match client.container_get_archive(id, "/", limit) {
        Ok(result) => result,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Err(ExecutionManagerError::Unavailable(format!(
                "sandbox rootfs archive exceeds the {limit}-byte limit or was invalid ({error})"
            )));
        }
        Err(error) => {
            return Err(ExecutionManagerError::Unavailable(format!(
                "sandbox rootfs export failed: {error}"
            )));
        }
    };
    if !stat.is_dir() {
        return Err(ExecutionManagerError::Unavailable(
            "sandbox rootfs export: daemon did not report / as a directory".into(),
        ));
    }
    if archive.is_empty() {
        return Err(ExecutionManagerError::Unavailable(
            "sandbox rootfs archive was empty".into(),
        ));
    }
    Ok(archive)
}

#[cfg(unix)]
impl SandboxLocalExecutionBackend {
    fn require_container_id(&self, record: &BoxRecord) -> ExecutionManagerResult<String> {
        self.load_container_id(record)?.ok_or_else(|| {
            ExecutionManagerError::Unavailable(format!(
                "sandbox container id missing for execution {}",
                record.id
            ))
        })
    }
}

/// Keep blocking Docker-socket I/O off the async executor threads.
#[cfg(unix)]
async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> ExecutionManagerResult<T> + Send + 'static,
) -> ExecutionManagerResult<T> {
    tokio::task::spawn_blocking(work).await.map_err(|error| {
        ExecutionManagerError::Internal(format!("sandbox file task failed: {error}"))
    })?
}

/// Classify an engine I/O failure: path-level refusals become a failed
/// response the CLI can report; transport failures stay typed errors.
#[cfg(unix)]
fn sandbox_io_failure(
    action: &str,
    path: &str,
    error: &std::io::Error,
) -> ExecutionManagerResult<String> {
    match error.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => {
            Ok(format!("{path}: {error}"))
        }
        _ => Err(ExecutionManagerError::Unavailable(format!(
            "sandbox {action} failed: {error}"
        ))),
    }
}

#[cfg(unix)]
fn failed_file_response(error: String) -> FileResponse {
    FileResponse {
        success: false,
        data: None,
        size: 0,
        error: Some(error),
    }
}

/// Split an absolute guest file path into (parent directory, file name).
#[cfg(unix)]
fn split_upload_path(guest_path: &str) -> ExecutionManagerResult<(&str, &str)> {
    let invalid = |reason: &str| {
        ExecutionManagerError::InvalidRequest(format!(
            "sandbox upload path {guest_path:?}: {reason}"
        ))
    };
    if !guest_path.starts_with('/') {
        return Err(invalid("must be absolute"));
    }
    let (parent, name) = guest_path
        .rsplit_once('/')
        .ok_or_else(|| invalid("no name"))?;
    if name.is_empty() || name == "." || name == ".." {
        return Err(invalid("must name a file"));
    }
    Ok((if parent.is_empty() { "/" } else { parent }, name))
}

/// One-entry tar archive holding a regular file named `name`.
#[cfg(unix)]
fn single_file_tar(name: &str, content: &[u8]) -> ExecutionManagerResult<Vec<u8>> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs()),
    );
    let mut builder = tar::Builder::new(Vec::with_capacity(content.len() + 2048));
    builder
        .append_data(&mut header, name, content)
        .and_then(|()| builder.into_inner())
        .map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!("sandbox upload archive: {error}"))
        })
}

/// A validated upload: bounded decoded content already wrapped in a
/// one-entry tar for the archive API.
#[cfg(unix)]
struct PreparedUpload {
    guest_path: String,
    parent: String,
    archive: Vec<u8>,
    size: u64,
}

#[cfg(unix)]
impl PreparedUpload {
    fn new(request: &FileRequest) -> ExecutionManagerResult<Self> {
        use base64::Engine;
        let max = kbox_sandbox_engine::DockerHttpClient::MAX_ARCHIVE_FILE_BYTES;
        if request.user.is_some() {
            // The archive API cannot assign ownership without copyUIDGID;
            // refuse rather than silently create root-owned files.
            return Err(ExecutionManagerError::InvalidRequest(
                "sandbox upload does not support a target user".into(),
            ));
        }
        let (parent, name) = split_upload_path(&request.guest_path)?;
        let encoded = request.data.as_deref().ok_or_else(|| {
            ExecutionManagerError::InvalidRequest("sandbox upload requires file data".into())
        })?;
        if encoded.len() > max.div_ceil(3) * 4 {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "sandbox upload exceeds the {max}-byte limit"
            )));
        }
        let content = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| {
                ExecutionManagerError::InvalidRequest(format!(
                    "sandbox upload data is not base64: {error}"
                ))
            })?;
        if content.len() > max {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "sandbox upload is {} bytes; max is {max}",
                content.len()
            )));
        }
        Ok(Self {
            guest_path: request.guest_path.clone(),
            parent: parent.to_string(),
            archive: single_file_tar(name, &content)?,
            size: content.len() as u64,
        })
    }
}

#[cfg(unix)]
fn sandbox_upload(
    client: &kbox_sandbox_engine::DockerHttpClient,
    id: &str,
    upload: &PreparedUpload,
) -> ExecutionManagerResult<FileResponse> {
    let parent = upload.parent.as_str();
    let mut put = client.container_put_archive(id, parent, &upload.archive);
    if put
        .as_ref()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        // Box file upload creates missing parent directories; the archive API
        // requires the directory to exist.
        let (exit_code, _stdout, stderr) = client
            .container_exec(
                id,
                &["mkdir".into(), "-p".into(), parent.into()],
                None,
                None,
                None,
                None,
            )
            .map_err(|error| {
                ExecutionManagerError::Unavailable(format!("sandbox mkdir failed: {error}"))
            })?;
        if exit_code != 0 {
            return Ok(failed_file_response(format!(
                "cannot create {parent}: {}",
                String::from_utf8_lossy(&stderr).trim()
            )));
        }
        put = client.container_put_archive(id, parent, &upload.archive);
    }
    match put {
        Ok(()) => Ok(FileResponse {
            success: true,
            data: None,
            size: upload.size,
            error: None,
        }),
        Err(error) => Ok(failed_file_response(sandbox_io_failure(
            "file upload",
            &upload.guest_path,
            &error,
        )?)),
    }
}

#[cfg(unix)]
fn sandbox_download(
    client: &kbox_sandbox_engine::DockerHttpClient,
    id: &str,
    request: &FileRequest,
) -> ExecutionManagerResult<FileResponse> {
    use base64::Engine;
    let engine_max = kbox_sandbox_engine::DockerHttpClient::MAX_ARCHIVE_FILE_BYTES as u64;
    let limit = match request.max_bytes {
        Some(max) if max > kbox_core::exec::MAX_BOUNDED_FILE_BYTES => {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "sandbox download max_bytes {max} exceeds {}",
                kbox_core::exec::MAX_BOUNDED_FILE_BYTES
            )));
        }
        Some(max) => max.min(engine_max),
        None => engine_max,
    };
    let path = request.guest_path.as_str();
    if path.is_empty() {
        return Err(ExecutionManagerError::InvalidRequest(
            "sandbox download requires a non-empty path".into(),
        ));
    }
    // Tar headers and padding ride on top of the content bound.
    let archive_limit = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .saturating_add(64 * 1024);
    let (stat, archive) = match client.container_get_archive(id, path, archive_limit) {
        Ok(result) => result,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(failed_file_response(format!(
                "{path}: file exceeds the {limit}-byte download limit or the archive was invalid ({error})"
            )));
        }
        Err(error) => {
            return Ok(failed_file_response(sandbox_io_failure(
                "file download",
                path,
                &error,
            )?));
        }
    };
    if !stat.is_regular() {
        return Ok(failed_file_response(format!(
            "{path}: not a regular file (directory, symlink or special file)"
        )));
    }
    let invalid_archive = |error: std::io::Error| {
        ExecutionManagerError::Unavailable(format!("sandbox download archive is invalid: {error}"))
    };
    let mut archive = tar::Archive::new(std::io::Cursor::new(archive));
    let mut content: Option<Vec<u8>> = None;
    for entry in archive.entries().map_err(invalid_archive)? {
        let mut entry = entry.map_err(invalid_archive)?;
        if content.is_some() || !entry.header().entry_type().is_file() {
            return Ok(failed_file_response(format!(
                "{path}: daemon returned an unexpected archive layout"
            )));
        }
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut std::io::Read::take(&mut entry, limit.saturating_add(1)),
            &mut bytes,
        )
        .map_err(invalid_archive)?;
        if bytes.len() as u64 > limit {
            return Ok(failed_file_response(format!(
                "{path}: file exceeds the {limit}-byte download limit"
            )));
        }
        content = Some(bytes);
    }
    let Some(content) = content else {
        return Ok(failed_file_response(format!(
            "{path}: daemon returned an empty archive"
        )));
    };
    Ok(FileResponse {
        success: true,
        data: Some(base64::engine::general_purpose::STANDARD.encode(&content)),
        size: content.len() as u64,
        error: None,
    })
}

#[cfg(unix)]
fn sandbox_stat(
    client: &kbox_sandbox_engine::DockerHttpClient,
    id: &str,
    path: &str,
) -> ExecutionManagerResult<FilesystemResponse> {
    let stat = match client.container_path_stat(id, path) {
        Ok(stat) => stat,
        Err(error) => {
            return Ok(FilesystemResponse {
                success: false,
                entry: None,
                entries: Vec::new(),
                error: Some(sandbox_io_failure("stat", path, &error)?),
            });
        }
    };
    // Symlinks are not followed (the archive API stats the link itself), so
    // they report Unspecified and callers fail closed instead of guessing.
    let kind = if stat.is_dir() {
        FilesystemEntryKind::Directory
    } else if stat.is_regular() {
        FilesystemEntryKind::File
    } else {
        FilesystemEntryKind::Unspecified
    };
    let mode = stat.unix_mode();
    let type_char = match kind {
        FilesystemEntryKind::Directory => 'd',
        _ if stat.is_symlink() => 'l',
        _ => '-',
    };
    let modified = chrono::DateTime::parse_from_rfc3339(&stat.mtime).ok();
    Ok(FilesystemResponse {
        success: true,
        entry: Some(FilesystemEntry {
            name: stat.name.clone(),
            kind,
            path: path.to_string(),
            size: stat.size,
            mode,
            permissions: permission_string(type_char, mode),
            owner: String::new(),
            group: String::new(),
            modified_seconds: modified.map_or(0, |time| time.timestamp()),
            modified_nanos: modified.map_or(0, |time| {
                i32::try_from(time.timestamp_subsec_nanos()).unwrap_or(0)
            }),
            symlink_target: (!stat.link_target.is_empty()).then(|| stat.link_target.clone()),
            metadata: std::collections::BTreeMap::new(),
        }),
        entries: Vec::new(),
        error: None,
    })
}

#[cfg(unix)]
fn permission_string(type_char: char, mode: u32) -> String {
    let mut out = String::with_capacity(10);
    out.push(type_char);
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        out.push(if bits & 0o4 != 0 { 'r' } else { '-' });
        out.push(if bits & 0o2 != 0 { 'w' } else { '-' });
        out.push(if bits & 0o1 != 0 { 'x' } else { '-' });
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    use async_trait::async_trait;
    use kbox_core::{
        BoxConfig, CreateExecutionRequest, ExecutionId, ExecutionManagerError,
        ExecutionManagerResult, ExecutionState, KillOutcome, OperationId,
    };

    use super::SandboxLocalExecutionBackend;
    use crate::local_execution::record::build_managed_record;
    use crate::local_execution::{
        LocalExecutionBackend, LocalExecutionBackendRouter, LocalExecutionHandle,
        LocalExecutionObservation, LocalExecutionResourcePlan, OciMigrationPolicy,
    };
    use crate::{BoxRecord, ManagedRuntimeRoute};

    #[derive(Default)]
    struct CountingBackend {
        starts: AtomicUsize,
        kills: AtomicUsize,
    }

    impl CountingBackend {
        fn starts(&self) -> usize {
            self.starts.load(Ordering::Relaxed)
        }

        fn kills(&self) -> usize {
            self.kills.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl LocalExecutionBackend for CountingBackend {
        async fn start(&self, _record: &BoxRecord) -> ExecutionManagerResult<LocalExecutionHandle> {
            self.starts.fetch_add(1, Ordering::Relaxed);
            Err(ExecutionManagerError::Unavailable(
                "counting backend must not receive sandbox work".into(),
            ))
        }

        async fn inspect(
            &self,
            _record: &BoxRecord,
        ) -> ExecutionManagerResult<LocalExecutionObservation> {
            Ok(LocalExecutionObservation {
                state: ExecutionState::Stopped,
                handle: None,
                exit_code: None,
            })
        }

        async fn pause(
            &self,
            _record: &BoxRecord,
            _keep_memory: bool,
        ) -> ExecutionManagerResult<LocalExecutionHandle> {
            Err(ExecutionManagerError::Unavailable("pause".into()))
        }

        async fn resume(
            &self,
            _record: &BoxRecord,
        ) -> ExecutionManagerResult<LocalExecutionHandle> {
            Err(ExecutionManagerError::Unavailable("resume".into()))
        }

        async fn kill(&self, _record: &BoxRecord) -> ExecutionManagerResult<KillOutcome> {
            self.kills.fetch_add(1, Ordering::Relaxed);
            Ok(KillOutcome::Killed)
        }

        async fn plan_create_resources(
            &self,
            _record: &BoxRecord,
        ) -> ExecutionManagerResult<LocalExecutionResourcePlan> {
            Ok(LocalExecutionResourcePlan::default())
        }
    }

    fn sandbox_record(home: &Path, suffix: &str) -> BoxRecord {
        let mut record = build_managed_record(
            home,
            &ExecutionId::new(format!("00000000-0000-4000-8000-{suffix:0>12}")).unwrap(),
            OperationId::new(format!("sandbox-{suffix}")).unwrap(),
            CreateExecutionRequest {
                external_sandbox_id: "sandbox-backend".into(),
                config: BoxConfig {
                    image: "alpine:latest".into(),
                    cmd: vec!["echo".into(), "hi".into()],
                    extra_env: vec![("A".into(), "1".into())],
                    ..Default::default()
                },
                labels: Default::default(),
                policy: Default::default(),
                rootfs_snapshot_id: None,
            },
            chrono::Utc::now(),
        )
        .unwrap();
        record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::Sandbox;
        record
    }

    #[tokio::test]
    async fn route_for_create_pins_sandbox() {
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        let record = sandbox_record(temporary.path(), "route");
        assert_eq!(
            backend.route_for_create(&record).unwrap(),
            ManagedRuntimeRoute::Sandbox
        );
    }

    #[tokio::test]
    async fn start_fail_closes_without_downgrade_to_box_or_oci() {
        // Disable spawn so hermetic tests do not wait on a missing binary.
        // SAFETY: test-local env; no concurrent writers in this unit.
        unsafe { std::env::set_var("KBOX_SANDBOX_AUTOSTART", "0") };
        let temporary = tempfile::tempdir().unwrap();
        let legacy = Arc::new(CountingBackend::default());
        let oci = Arc::new(CountingBackend::default());
        let sandbox = Arc::new(SandboxLocalExecutionBackend::new(temporary.path()));
        let router = LocalExecutionBackendRouter::new(
            legacy.clone(),
            oci.clone(),
            OciMigrationPolicy::LegacyOnly,
        )
        .with_sandbox(sandbox);

        let record = sandbox_record(temporary.path(), "nodowngrade");
        let error = router.start(&record).await.unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("daemon")
                || message.contains("not present")
                || message.contains("unreachable")
                || message.contains("autostart"),
            "expected fail-closed daemon error, got: {message}"
        );
        assert_eq!(legacy.starts(), 0, "must not fall back to BoxVm");
        assert_eq!(oci.starts(), 0, "must not fall back to OciSdk");
        assert_eq!(legacy.kills(), 0);
        assert_eq!(oci.kills(), 0);
    }

    #[tokio::test]
    async fn refuses_non_sandbox_pinned_record() {
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        let mut record = sandbox_record(temporary.path(), "wrongroute");
        record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::BoxVm;

        let error = backend.preflight(&record).await.unwrap_err();
        assert!(
            error.to_string().contains("not Sandbox"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn preflight_launch_is_hermetic_without_daemon() {
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        backend.preflight_launch().await.unwrap();
        let caps = kbox_sandbox_engine::discover_capabilities(temporary.path());
        assert!(!caps.daemon_reachable);
        assert_eq!(
            caps.socket_path,
            kbox_sandbox_engine::docker_socket_path(temporary.path())
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn start_and_inspect_against_fake_unix_http_daemon() {
        // Short home path: AF_UNIX sun_path is small on macOS.
        let home = std::env::temp_dir().join(format!("sb{}h", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            // Answer create/start/inspect; ignore log-follow GETs from the
            // production start path without counting them toward completion.
            let mut answered = 0;
            while answered < 3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 16384];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                if request.contains("/logs") {
                    let response =
                        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                    continue;
                }
                let (status, body) = if request.starts_with("POST /containers/create") {
                    assert!(request.contains("\"Image\":\"alpine:latest\""));
                    assert!(request.contains("\"Cmd\":[\"echo\",\"hi\"]"));
                    (201_u16, r#"{"Id":"cafebabe0123"}"#)
                } else if request.starts_with("POST /containers/cafebabe0123/start") {
                    (204_u16, "")
                } else if request.starts_with("GET /containers/cafebabe0123/json") {
                    (
                        200_u16,
                        r#"{"Id":"cafebabe0123","State":{"Running":true,"Paused":false,"ExitCode":0}}"#,
                    )
                } else {
                    (500_u16, r#"{"message":"unexpected request"}"#)
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                answered += 1;
            }
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let record = sandbox_record(&home, "fakedaemon");
        std::fs::create_dir_all(&record.box_dir).unwrap();

        let handle = backend.start(&record).await.unwrap();
        assert_eq!(handle.exec_socket_path, socket);
        assert!(handle.pid.is_none());
        let persisted = kbox_sandbox_engine::read_container_id(&record.box_dir)
            .unwrap()
            .expect("container id file");
        assert_eq!(persisted, "cafebabe0123");

        let observation = backend.inspect(&record).await.unwrap();
        assert_eq!(observation.state, ExecutionState::Running);
        assert!(observation.handle.is_some());

        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn execute_against_fake_unix_http_daemon() {
        fn mux_frame(stream: u8, payload: &[u8]) -> Vec<u8> {
            let mut frame = Vec::with_capacity(8 + payload.len());
            frame.push(stream);
            frame.extend_from_slice(&[0, 0, 0]);
            frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            frame.extend_from_slice(payload);
            frame
        }

        let home = std::env::temp_dir().join(format!("sb{}x", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let mut answered = 0;
            while answered < 3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 16384];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                let (status, body): (u16, Vec<u8>) =
                    if request.starts_with("POST /containers/cafebabeexec/exec") {
                        assert!(request.contains("\"Cmd\":[\"echo\",\"SANDBOX_EXEC_OK\"]"));
                        assert!(request.contains("\"Env\":[\"K=1\"]"));
                        (201, br#"{"Id":"execcafe01"}"#.to_vec())
                    } else if request.starts_with("POST /exec/execcafe01/start") {
                        (200, mux_frame(1, b"SANDBOX_EXEC_OK\n"))
                    } else if request.starts_with("GET /exec/execcafe01/json") {
                        (
                            200,
                            br#"{"ID":"execcafe01","Running":false,"ExitCode":0}"#.to_vec(),
                        )
                    } else {
                        (500, br#"{"message":"unexpected request"}"#.to_vec())
                    };
                let header = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
                answered += 1;
            }
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let record = sandbox_record(&home, "execfake");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "cafebabeexec").unwrap();

        let output = backend
            .execute(
                &record,
                kbox_core::ExecRequest {
                    request_id: Some("hermetic-exec-1".into()),
                    cmd: vec!["echo".into(), "SANDBOX_EXEC_OK".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec!["K=1".into()],
                    working_dir: None,
                    rootfs: None,
                    stdin: None,
                    stdin_streaming: false,
                    user: None,
                    streaming: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, b"SANDBOX_EXEC_OK\n");
        assert!(output.stderr.is_empty());

        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn execute_with_captured_stdin_against_fake_unix_http_daemon() {
        fn mux_frame(stream: u8, payload: &[u8]) -> Vec<u8> {
            let mut frame = Vec::with_capacity(8 + payload.len());
            frame.push(stream);
            frame.extend_from_slice(&[0, 0, 0]);
            frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            frame.extend_from_slice(payload);
            frame
        }

        let home = std::env::temp_dir().join(format!("sb{}c", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/containers/cafebabecap/exec"));
            assert!(request.contains("\"AttachStdin\":true"));
            assert!(request.contains("\"Cmd\":[\"sh\"]"));
            let body = br#"{"Id":"execcapstdin"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execcapstdin/start"));
            assert!(request.contains("Upgrade: tcp"));
            let payload = mux_frame(1, b"CAP_STDIN_OK\n");
            let mut response =
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n"
                    .to_vec();
            response.extend_from_slice(&payload);
            let _ = stream.write_all(&response);
            let mut got = Vec::new();
            let _ = stream.read_to_end(&mut got);
            assert!(
                got.windows(18).any(|w| w == b"echo CAP_STDIN_OK\n"),
                "expected captured stdin, got {got:?}"
            );
            let _ = stream.shutdown(std::net::Shutdown::Write);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execcapstdin/json"));
            let body = br#"{"ID":"execcapstdin","Running":false,"ExitCode":0}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let record = sandbox_record(&home, "capstdin");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "cafebabecap").unwrap();

        let output = backend
            .execute(
                &record,
                kbox_core::ExecRequest {
                    request_id: Some("hermetic-cap-stdin".into()),
                    cmd: vec!["sh".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: Some(b"echo CAP_STDIN_OK\n".to_vec()),
                    stdin_streaming: false,
                    user: None,
                    streaming: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, b"CAP_STDIN_OK\n");
        assert!(output.stderr.is_empty());

        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn execute_rejects_stdin_streaming_flag() {
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        let record = sandbox_record(temporary.path(), "streamflag");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "unused").unwrap();
        let error = backend
            .execute(
                &record,
                kbox_core::ExecRequest {
                    request_id: None,
                    cmd: vec!["true".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: Some(b"x".to_vec()),
                    stdin_streaming: true,
                    user: None,
                    streaming: false,
                },
            )
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("streaming stdin"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn execute_fail_closes_without_daemon() {
        // SAFETY: test-local env; no concurrent writers in this unit.
        unsafe { std::env::set_var("KBOX_SANDBOX_AUTOSTART", "0") };
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        let record = sandbox_record(temporary.path(), "execnod");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "missing").unwrap();
        let error = backend
            .execute(
                &record,
                kbox_core::ExecRequest {
                    request_id: None,
                    cmd: vec!["true".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: None,
                    stdin_streaming: false,
                    user: None,
                    streaming: false,
                },
            )
            .await
            .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("daemon")
                || message.contains("unreachable")
                || message.contains("not present"),
            "expected fail-closed daemon error, got: {message}"
        );
    }

    #[tokio::test]
    async fn start_process_fail_closes_without_daemon() {
        // SAFETY: test-local env; no concurrent writers in this unit.
        unsafe { std::env::set_var("KBOX_SANDBOX_AUTOSTART", "0") };
        let temporary = tempfile::tempdir().unwrap();
        let backend = SandboxLocalExecutionBackend::new(temporary.path());
        let record = sandbox_record(temporary.path(), "nostream");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "missing").unwrap();
        let result = backend
            .start_process(
                &record,
                kbox_core::ExecRequest {
                    request_id: None,
                    cmd: vec!["true".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: None,
                    stdin_streaming: true,
                    user: None,
                    streaming: true,
                },
            )
            .await;
        let message = match result {
            Err(error) => error.to_string(),
            Ok(_) => panic!("expected Unavailable without daemon"),
        };
        assert!(
            message.contains("daemon")
                || message.contains("unreachable")
                || message.contains("not present")
                || message.contains("Unavailable"),
            "expected fail-closed daemon error, got: {message}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn start_process_against_fake_unix_http_daemon() {
        use kbox_core::exec::ExecEvent;
        use std::os::unix::net::UnixListener;

        let home = std::env::temp_dir().join(format!("sb{}s", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/containers/cafebabeatt/exec"));
            assert!(request.contains("\"Tty\":false"));
            assert!(request.contains("\"AttachStdin\":true"));
            let body = br#"{"Id":"execstreamcafe"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execstreamcafe/start"));
            assert!(request.contains("Upgrade: tcp"));
            assert!(request.contains("\"Tty\":false"));
            let mut payload = Vec::new();
            // Docker mux: stream 1 stdout
            payload.push(1);
            payload.extend_from_slice(&[0, 0, 0]);
            let data = b"STREAM_OK\n";
            payload.extend_from_slice(&(data.len() as u32).to_be_bytes());
            payload.extend_from_slice(data);
            let mut response =
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n"
                    .to_vec();
            response.extend_from_slice(&payload);
            let _ = stream.write_all(&response);
            // Keep the attach socket open for stdin while the client drains
            // mux output; after stdin EOF, close our write side so the pump
            // exits and inspect can run.
            let mut got = Vec::new();
            let _ = stream.read_to_end(&mut got);
            assert!(
                got.windows(5).any(|w| w == b"hello"),
                "expected client stdin hello, got {got:?}"
            );
            let _ = stream.shutdown(std::net::Shutdown::Write);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execstreamcafe/json"));
            let body = br#"{"ID":"execstreamcafe","Running":false,"ExitCode":0}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let record = sandbox_record(&home, "streamfake");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "cafebabeatt").unwrap();

        let mut process = backend
            .start_process(
                &record,
                kbox_core::ExecRequest {
                    request_id: None,
                    cmd: vec!["sh".into()],
                    timeout_ns: 5_000_000_000,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: Some(b"hello".to_vec()),
                    stdin_streaming: true,
                    user: None,
                    streaming: true,
                },
            )
            .await
            .map_err(|error| error.to_string())
            .expect("start_process");

        // Half-close stdin so the fake daemon's read_to_end returns and the
        // inspect round-trip can complete.
        process
            .input()
            .close_stdin()
            .await
            .map_err(|error| error.to_string())
            .expect("close_stdin");

        let mut saw_ok = false;
        let mut exit = None;
        while let Some(event) = process
            .next_event()
            .await
            .map_err(|error| error.to_string())
            .expect("next_event")
        {
            match event {
                ExecEvent::Chunk(chunk) => {
                    if chunk.data.windows(9).any(|w| w == b"STREAM_OK") {
                        saw_ok = true;
                    }
                }
                ExecEvent::Exit(status) => {
                    exit = Some(status.exit_code);
                    break;
                }
                ExecEvent::FlushAck => {}
            }
        }
        assert!(saw_ok, "expected STREAM_OK chunk");
        assert_eq!(exit, Some(0));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn start_pty_against_fake_unix_http_daemon() {
        use kbox_core::exec::ExecEvent;
        use kbox_core::pty::PtyRequest;
        use std::os::unix::net::UnixListener;

        let home = std::env::temp_dir().join(format!("sb{}p", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            // create
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/containers/cafebabepyt/exec"));
            assert!(request.contains("\"Tty\":true"));
            let body = br#"{"Id":"execptycafe"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);

            // start upgrade
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execptycafe/start"));
            assert!(request.contains("Upgrade: tcp"));
            let _ = stream.write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\nPTY_OK\n",
            );
            // Drop write so the reader sees EOF after PTY_OK.
            let _ = stream.shutdown(std::net::Shutdown::Write);

            // inspect exit
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execptycafe/json"));
            let body = br#"{"ID":"execptycafe","Running":false,"ExitCode":0}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let record = sandbox_record(&home, "ptyfake");
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "cafebabepyt").unwrap();

        let mut process = backend
            .start_pty(
                &record,
                PtyRequest {
                    cmd: vec!["sh".into()],
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    user: None,
                    cols: 80,
                    rows: 24,
                },
            )
            .await
            .map_err(|error| error.to_string())
            .expect("start_pty");

        let mut saw_ok = false;
        let mut exit = None;
        while let Some(event) = process
            .next_event()
            .await
            .map_err(|error| error.to_string())
            .expect("next_event")
        {
            match event {
                ExecEvent::Chunk(chunk) => {
                    if chunk.data.windows(6).any(|w| w == b"PTY_OK") {
                        saw_ok = true;
                    }
                }
                ExecEvent::Exit(status) => {
                    exit = Some(status.exit_code);
                    break;
                }
                ExecEvent::FlushAck => {}
            }
        }
        assert!(saw_ok, "expected PTY_OK chunk");
        assert_eq!(exit, Some(0));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn map_env_expands_secret_manifest_from_readonly_binds() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().unwrap();
        let secret_path = temporary.path().join("000.secret");
        std::fs::write(&secret_path, b"sandbox-secret-fixture").unwrap();
        std::fs::set_permissions(&secret_path, std::fs::Permissions::from_mode(0o400)).unwrap();

        let digest = "c".repeat(64);
        let guest = format!("/.kbox-secrets/{digest}/000.secret");
        let binding = kbox_core::secret::SecretEnvironmentBinding {
            variable: "API_TOKEN".into(),
            path: guest.clone(),
        };
        let manifest = serde_json::to_string(&vec![binding]).unwrap();
        let mut record = sandbox_record(temporary.path(), "secretenv");
        record.env.insert(
            kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.into(),
            manifest,
        );
        record.env.insert("PLAIN".into(), "visible".into());
        record.volumes = vec![format!("{}:{guest}:ro", secret_path.display())];

        let mapped = SandboxLocalExecutionBackend::map_env(&record).expect("map_env");
        assert!(
            mapped
                .durable
                .iter()
                .all(|(key, _)| key != kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST),
            "manifest must be consumed before engine Env"
        );
        assert!(
            mapped.durable.iter().all(|(key, _)| key != "API_TOKEN"),
            "Secret values must not land in durable create Env"
        );
        assert_eq!(
            mapped
                .durable
                .iter()
                .find(|(key, _)| key == "PLAIN")
                .map(|(_, value)| value.as_str()),
            Some("visible")
        );
        let token = mapped
            .ephemeral
            .iter()
            .find(|(key, _)| key == "API_TOKEN")
            .map(|(_, value)| value.as_str());
        assert_eq!(token, Some("sandbox-secret-fixture"));
    }

    #[cfg(unix)]
    #[test]
    fn map_env_refuses_missing_secret_bind() {
        let temporary = tempfile::tempdir().unwrap();
        let digest = "d".repeat(64);
        let guest = format!("/.kbox-secrets/{digest}/000.secret");
        let binding = kbox_core::secret::SecretEnvironmentBinding {
            variable: "API_TOKEN".into(),
            path: guest,
        };
        let mut record = sandbox_record(temporary.path(), "secretmiss");
        record.env.insert(
            kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.into(),
            serde_json::to_string(&vec![binding]).unwrap(),
        );
        let error = SandboxLocalExecutionBackend::map_env(&record).expect_err("missing bind");
        assert!(
            error.to_string().contains("missing or ambiguous"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn start_attaches_secret_env_via_ephemeral_not_create() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::{Arc, Mutex};

        let home = std::env::temp_dir().join(format!("sb{}se", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let secret_path = home.join("000.secret");
        std::fs::write(&secret_path, b"sandbox-secret-fixture").unwrap();
        std::fs::set_permissions(&secret_path, std::fs::Permissions::from_mode(0o400)).unwrap();

        let digest = "e".repeat(64);
        let guest = format!("/.kbox-secrets/{digest}/000.secret");
        let binding = kbox_core::secret::SecretEnvironmentBinding {
            variable: "API_TOKEN".into(),
            path: guest.clone(),
        };
        let mut record = sandbox_record(&home, "ephemera");
        record.env.insert(
            kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.into(),
            serde_json::to_string(&vec![binding]).unwrap(),
        );
        record.volumes = vec![format!("{}:{guest}:ro", secret_path.display())];
        std::fs::create_dir_all(&record.box_dir).unwrap();

        let socket = kbox_sandbox_engine::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_server = Arc::clone(&seen);
        let server = thread::spawn(move || {
            let mut answered = 0;
            while answered < 3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 65536];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                if request.contains("/logs") {
                    let response =
                        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                    continue;
                }
                seen_server.lock().unwrap().push(request.clone());
                let (status, body) = if request.starts_with("POST /containers/create") {
                    assert!(
                        !request.contains("sandbox-secret-fixture"),
                        "create must not carry Secret plaintext"
                    );
                    assert!(!request.contains("API_TOKEN="));
                    (201_u16, r#"{"Id":"cafeephem0123"}"#)
                } else if request.starts_with("POST /kbox/containers/cafeephem0123/ephemeral-env") {
                    assert!(request.contains("API_TOKEN=sandbox-secret-fixture"));
                    (204_u16, "")
                } else if request.starts_with("POST /containers/cafeephem0123/start") {
                    (204_u16, "")
                } else {
                    (500_u16, r#"{"message":"unexpected request"}"#)
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                answered += 1;
            }
        });

        let backend = SandboxLocalExecutionBackend::new(&home);
        let handle = backend.start(&record).await.unwrap();
        assert_eq!(handle.exec_socket_path, socket);
        server.join().unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 3, "create + ephemeral-env + start");
        let _ = std::fs::remove_dir_all(&home);
    }

    // ---- Spec050 file transfer / stat through the daemon archive API ----

    type FakeReply = (u16, String, Vec<u8>);

    /// Fake daemon answering exactly `requests` connections. The handler gets
    /// the request head and body and returns (status, extra headers, body).
    /// The thread returns every (head, body) it saw, in order.
    fn spawn_fake_daemon(
        home: &Path,
        requests: usize,
        mut handler: impl FnMut(&str, &[u8]) -> FakeReply + Send + 'static,
    ) -> thread::JoinHandle<Vec<(String, Vec<u8>)>> {
        let socket = kbox_sandbox_engine::docker_socket_path(home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        thread::spawn(move || {
            let mut seen = Vec::new();
            while seen.len() < requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buf = [0_u8; 8192];
                let (head_end, content_length) = loop {
                    let n = stream.read(&mut buf).unwrap();
                    if n == 0 {
                        break (0, 0);
                    }
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(pos) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&raw[..pos]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (pos + 4, length);
                    }
                };
                if head_end == 0 {
                    continue;
                }
                while raw.len() < head_end + content_length {
                    let n = stream.read(&mut buf).unwrap();
                    raw.extend_from_slice(&buf[..n]);
                }
                let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
                let body = raw[head_end..].to_vec();
                let (status, extra, reply) = handler(&head, &body);
                let mut out = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
                    reply.len()
                )
                .into_bytes();
                out.extend_from_slice(&reply);
                let _ = stream.write_all(&out);
                seen.push((head, body));
            }
            seen
        })
    }

    fn path_stat_header(name: &str, size: u64, mode: u64) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let json = format!(
            r#"{{"name":"{name}","size":{size},"mode":{mode},"mtime":"2026-01-02T03:04:05Z","linkTarget":""}}"#
        );
        let mut encoded = String::new();
        for chunk in json.as_bytes().chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0_u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
            for i in 0..=chunk.len() {
                encoded.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
        }
        format!("X-Docker-Container-Path-Stat: {encoded}\r\n")
    }

    fn tar_with_file(name: &str, content: &[u8]) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        let mut builder = tar::Builder::new(Vec::new());
        builder.append_data(&mut header, name, content).unwrap();
        builder.into_inner().unwrap()
    }

    fn sandbox_with_container(home_tag: &str) -> (std::path::PathBuf, BoxRecord) {
        let home =
            std::env::temp_dir().join(format!("sb{}{home_tag}", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let record = sandbox_record(&home, home_tag);
        std::fs::create_dir_all(&record.box_dir).unwrap();
        kbox_sandbox_engine::write_container_id(&record.box_dir, "cafebabefile").unwrap();
        (home, record)
    }

    fn download_request(path: &str) -> kbox_core::FileRequest {
        kbox_core::FileRequest {
            op: kbox_core::FileOp::Download,
            guest_path: path.into(),
            data: None,
            user: None,
            max_bytes: None,
            request_id: None,
        }
    }

    fn upload_request(path: &str, content: &[u8]) -> kbox_core::FileRequest {
        use base64::Engine;
        kbox_core::FileRequest {
            op: kbox_core::FileOp::Upload,
            guest_path: path.into(),
            data: Some(base64::engine::general_purpose::STANDARD.encode(content)),
            user: None,
            max_bytes: None,
            request_id: Some("hermetic-upload-1".into()),
        }
    }

    fn stat_request(path: &str) -> kbox_core::FilesystemRequest {
        kbox_core::FilesystemRequest {
            op: kbox_core::FilesystemOp::Stat,
            path: path.into(),
            destination: None,
            depth: 0,
            user: None,
            request_id: None,
        }
    }

    #[test]
    fn split_upload_path_requires_absolute_file_path() {
        assert_eq!(
            super::split_upload_path("/etc/hosts").unwrap(),
            ("/etc", "hosts")
        );
        assert_eq!(super::split_upload_path("/top").unwrap(), ("/", "top"));
        for bad in ["relative.txt", "/dir/", "/dir/..", "/dir/.", "/"] {
            assert!(
                matches!(
                    super::split_upload_path(bad),
                    Err(ExecutionManagerError::InvalidRequest(_))
                ),
                "{bad} must be refused"
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn download_against_fake_unix_http_daemon() {
        use base64::Engine;
        let (home, record) = sandbox_with_container("dl");
        let archive = tar_with_file("note.txt", b"hello from the box\n");
        let header = path_stat_header("note.txt", 19, 0o644);
        let server = spawn_fake_daemon(&home, 1, move |head, _| {
            assert!(
                head.starts_with("GET /containers/cafebabefile/archive?path=%2Ftmp%2Fnote.txt ")
            );
            (200, header.clone(), archive.clone())
        });

        let response = SandboxLocalExecutionBackend::new(&home)
            .transfer_file(&record, download_request("/tmp/note.txt"))
            .await
            .unwrap();
        server.join().unwrap();

        assert!(response.success, "{:?}", response.error);
        assert_eq!(response.size, 19);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(response.data.unwrap())
            .unwrap();
        assert_eq!(decoded, b"hello from the box\n");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn download_of_directory_and_missing_path_report_failed_responses() {
        let (home, record) = sandbox_with_container("dm");
        let dir_header = path_stat_header("etc", 4096, (1 << 31) | 0o755);
        let server = spawn_fake_daemon(&home, 2, move |head, _| {
            if head.contains("path=%2Fetc ") {
                (200, dir_header.clone(), tar_with_file("etc/x", b"x"))
            } else {
                (
                    404,
                    String::new(),
                    br#"{"message":"no such file"}"#.to_vec(),
                )
            }
        });
        let backend = SandboxLocalExecutionBackend::new(&home);

        let directory = backend
            .transfer_file(&record, download_request("/etc"))
            .await
            .unwrap();
        assert!(!directory.success);
        assert!(directory.error.unwrap().contains("not a regular file"));

        let missing = backend
            .transfer_file(&record, download_request("/nope"))
            .await
            .unwrap();
        assert!(!missing.success);
        assert!(missing.error.unwrap().contains("no such file"));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn download_enforces_the_requested_byte_ceiling() {
        let (home, record) = sandbox_with_container("db");
        let archive = tar_with_file("big.bin", &[9_u8; 4096]);
        let header = path_stat_header("big.bin", 4096, 0o644);
        let server =
            spawn_fake_daemon(&home, 1, move |_, _| (200, header.clone(), archive.clone()));
        let mut request = download_request("/big.bin");
        request.max_bytes = Some(1024);

        let response = SandboxLocalExecutionBackend::new(&home)
            .transfer_file(&record, request)
            .await
            .unwrap();
        server.join().unwrap();

        assert!(!response.success);
        assert!(response.error.unwrap().contains("1024-byte download limit"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn export_rootfs_archive_gets_the_root_from_the_fake_daemon() {
        let (home, record) = sandbox_with_container("xr");
        let archive = tar_with_file("etc/hostname", b"box\n");
        let expected = archive.clone();
        let header = path_stat_header("/", 4096, (1 << 31) | 0o755);
        let server = spawn_fake_daemon(&home, 1, move |head, _| {
            assert!(head.starts_with("GET /containers/cafebabefile/archive?path=%2F "));
            (200, header.clone(), archive.clone())
        });

        let tar = SandboxLocalExecutionBackend::new(&home)
            .export_rootfs_archive(&record)
            .await
            .unwrap();
        server.join().unwrap();

        assert_eq!(tar, expected);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn export_rootfs_archive_fails_closed_without_daemon_or_on_bad_replies() {
        let (home, record) = sandbox_with_container("xf");
        let backend = SandboxLocalExecutionBackend::new(&home);
        let error = backend.export_rootfs_archive(&record).await.unwrap_err();
        assert!(
            matches!(&error, ExecutionManagerError::Unavailable(m) if m.contains("daemon unreachable")),
            "{error:?}"
        );

        let file_header = path_stat_header("f", 1, 0o644);
        let dir_header = path_stat_header("/", 4096, (1 << 31) | 0o755);
        let mut calls = 0;
        let server = spawn_fake_daemon(&home, 2, move |_, _| {
            calls += 1;
            if calls == 1 {
                (200, file_header.clone(), tar_with_file("f", b"x"))
            } else {
                (200, dir_header.clone(), Vec::new())
            }
        });
        let not_dir = backend.export_rootfs_archive(&record).await.unwrap_err();
        assert!(
            not_dir.to_string().contains("not report / as a directory"),
            "{not_dir}"
        );
        let empty = backend.export_rootfs_archive(&record).await.unwrap_err();
        assert!(empty.to_string().contains("was empty"), "{empty}");
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn export_rootfs_archive_refuses_a_non_sandbox_record() {
        let (home, mut record) = sandbox_with_container("xn");
        record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::BoxVm;

        let error = SandboxLocalExecutionBackend::new(&home)
            .export_rootfs_archive(&record)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("not Sandbox"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_against_fake_unix_http_daemon_sends_one_file_tar() {
        let (home, record) = sandbox_with_container("up");
        let server = spawn_fake_daemon(&home, 1, |head, _| {
            assert!(head.starts_with("PUT /containers/cafebabefile/archive?path=%2Fsrv%2Fdata "));
            (200, String::new(), Vec::new())
        });
        let payload = b"host bytes \x00\xff binary".to_vec();

        let response = SandboxLocalExecutionBackend::new(&home)
            .transfer_file(&record, upload_request("/srv/data/out.bin", &payload))
            .await
            .unwrap();
        let seen = server.join().unwrap();

        assert!(response.success, "{:?}", response.error);
        assert_eq!(response.size, payload.len() as u64);
        let mut archive = tar::Archive::new(std::io::Cursor::new(seen[0].1.clone()));
        let mut entries = archive.entries().unwrap();
        let mut entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.path().unwrap().to_str(), Some("out.bin"));
        assert!(entry.header().entry_type().is_file());
        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        assert_eq!(content, payload);
        assert!(entries.next().is_none(), "exactly one archive entry");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_creates_missing_parent_then_retries() {
        fn mux_frame(stream: u8, payload: &[u8]) -> Vec<u8> {
            let mut frame = vec![stream, 0, 0, 0];
            frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            frame.extend_from_slice(payload);
            frame
        }
        let (home, record) = sandbox_with_container("um");
        let mut puts = 0;
        let server = spawn_fake_daemon(&home, 5, move |head, _| {
            if head.starts_with("PUT /containers/cafebabefile/archive") {
                puts += 1;
                if puts == 1 {
                    return (
                        404,
                        String::new(),
                        br#"{"message":"no such directory"}"#.to_vec(),
                    );
                }
                (200, String::new(), Vec::new())
            } else if head.starts_with("POST /containers/cafebabefile/exec") {
                (201, String::new(), br#"{"Id":"execmkdir01"}"#.to_vec())
            } else if head.starts_with("POST /exec/execmkdir01/start") {
                (200, String::new(), mux_frame(1, b""))
            } else if head.starts_with("GET /exec/execmkdir01/json") {
                (
                    200,
                    String::new(),
                    br#"{"ID":"execmkdir01","Running":false,"ExitCode":0}"#.to_vec(),
                )
            } else {
                (500, String::new(), br#"{"message":"unexpected"}"#.to_vec())
            }
        });

        let response = SandboxLocalExecutionBackend::new(&home)
            .transfer_file(&record, upload_request("/new/dir/f.txt", b"x"))
            .await
            .unwrap();
        let seen = server.join().unwrap();

        assert!(response.success, "{:?}", response.error);
        let exec_create = &seen[1];
        assert!(exec_create
            .0
            .starts_with("POST /containers/cafebabefile/exec"));
        let create_body = String::from_utf8_lossy(&exec_create.1);
        assert!(create_body.contains(r#""Cmd":["mkdir","-p","/new/dir"]"#));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_refuses_target_user_and_invalid_payload_before_the_daemon() {
        let (home, record) = sandbox_with_container("ur");
        // No daemon socket exists: these must fail on validation alone.
        let backend = SandboxLocalExecutionBackend::new(&home);

        let mut with_user = upload_request("/x", b"x");
        with_user.user = Some("1000".into());
        assert!(matches!(
            backend.transfer_file(&record, with_user).await,
            Err(ExecutionManagerError::InvalidRequest(_))
        ));

        let mut not_base64 = upload_request("/x", b"x");
        not_base64.data = Some("***".into());
        assert!(matches!(
            backend.transfer_file(&record, not_base64).await,
            Err(ExecutionManagerError::InvalidRequest(_))
        ));

        let mut no_data = upload_request("/x", b"x");
        no_data.data = None;
        assert!(matches!(
            backend.transfer_file(&record, no_data).await,
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn transfer_file_fail_closes_without_daemon() {
        let (home, record) = sandbox_with_container("nd");
        let backend = SandboxLocalExecutionBackend::new(&home);

        let error = backend
            .transfer_file(&record, download_request("/etc/hosts"))
            .await
            .unwrap_err();

        assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
        assert!(error.to_string().contains("daemon"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn filesystem_stat_reports_file_and_directory_kinds() {
        use kbox_core::FilesystemEntryKind;
        let (home, record) = sandbox_with_container("fs");
        let file_header = path_stat_header("hosts", 12, 0o640);
        let dir_header = path_stat_header("etc", 4096, (1 << 31) | 0o755);
        let server = spawn_fake_daemon(&home, 3, move |head, _| {
            assert!(head.starts_with("HEAD /containers/cafebabefile/archive?path="));
            if head.contains("path=%2Fetc%2Fhosts ") {
                (200, file_header.clone(), Vec::new())
            } else if head.contains("path=%2Fetc ") {
                (200, dir_header.clone(), Vec::new())
            } else {
                (404, String::new(), Vec::new())
            }
        });
        let backend = SandboxLocalExecutionBackend::new(&home);

        let file = backend
            .filesystem(&record, stat_request("/etc/hosts"))
            .await
            .unwrap();
        let entry = file.entry.unwrap();
        assert!(file.success);
        assert_eq!(entry.kind, FilesystemEntryKind::File);
        assert_eq!(entry.size, 12);
        assert_eq!(entry.mode, 0o640);
        assert_eq!(entry.permissions, "-rw-r-----");
        assert_eq!(entry.modified_seconds, 1_767_323_045);

        let dir = backend
            .filesystem(&record, stat_request("/etc"))
            .await
            .unwrap();
        assert_eq!(dir.entry.unwrap().kind, FilesystemEntryKind::Directory);

        let missing = backend
            .filesystem(&record, stat_request("/missing"))
            .await
            .unwrap();
        assert!(!missing.success);
        assert!(missing.entry.is_none());
        assert!(missing.error.unwrap().contains("/missing"));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn filesystem_refuses_mutating_operations_without_a_daemon_call() {
        let (home, record) = sandbox_with_container("fm");
        let backend = SandboxLocalExecutionBackend::new(&home);
        let mut request = stat_request("/tmp/x");
        request.op = kbox_core::FilesystemOp::Remove;

        let error = backend.filesystem(&record, request).await.unwrap_err();

        assert!(matches!(error, ExecutionManagerError::Unavailable(_)));
        assert!(error.to_string().contains("supports only Stat"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
