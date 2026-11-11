use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kbox_contract::contract::{RuntimeUnitSpec, RuntimeUnitState};
use kbox_contract::{RuntimeError, RuntimeResult};
use kbox_core::{ExecutionGeneration, ExecutionId, ExecutionManager, ExecutionState};
use sha2::{Digest, Sha256};
use tokio::sync::{watch, Mutex};

use crate::kbox_contract_driver::prewarm::BoxPrewarmScopePort;
use crate::{BoxRuntimeDriver, BoxRuntimeDriverConfig};

use super::protocol::AcceptedIntent;
use super::{
    DesiredState, EngineError, EngineErrorCode, EngineJournal, EngineKey, EngineResult, LocalEngine,
};

pub(super) struct EnginePrewarm {
    journal: Arc<Mutex<EngineJournal>>,
    authority: String,
    executable: PathBuf,
    home_dir: PathBuf,
}

impl EngineJournal {
    pub(super) fn has_prewarm_history(&self, instance: &EngineKey) -> bool {
        self.data.requests.values().any(|previous| {
            previous.intent.instance_id == *instance
                && previous
                    .intent
                    .assembly
                    .as_ref()
                    .is_some_and(|plan| plan.prewarm_bindings_digest.is_some())
        })
    }
}

impl EnginePrewarm {
    pub(super) fn new(journal: Arc<Mutex<EngineJournal>>, home_dir: PathBuf) -> EngineResult<Self> {
        let id = journal.blocking_lock().engine_id().clone();
        let digest = format!("{:x}", Sha256::digest(id.as_str().as_bytes()));
        Ok(Self {
            journal,
            authority: format!("engine-{}:", &digest[..32]),
            executable: std::env::current_exe()?,
            home_dir,
        })
    }
}

#[async_trait]
impl BoxPrewarmScopePort for EnginePrewarm {
    async fn scope(&self, spec: &RuntimeUnitSpec) -> RuntimeResult<Option<String>> {
        let (bindings, grants) = {
            let journal = self.journal.lock().await;
            journal.ensure_usable().map_err(|_| unavailable())?;
            let prefix = format!("engine.{}.", journal.engine_id().as_str());
            let Some(record) = spec
                .unit_id
                .strip_prefix(&prefix)
                .and_then(|id| super::EngineKey::parse(id).ok())
                .and_then(|id| journal.current(&id).ok())
                .filter(|record| {
                    record.generation == spec.generation
                        && record.intent.desired_state == DesiredState::Up
                })
            else {
                return Ok(None);
            };
            let Some(bindings) = record
                .intent
                .assembly
                .as_ref()
                .and_then(|assembly| assembly.prewarm_bindings_digest.clone())
            else {
                return Ok(None);
            };
            if record
                .intent
                .bind_service(spec.unit_id.clone(), spec.generation)
                .ok()
                .as_ref()
                != Some(spec)
            {
                return Err(RuntimeError::Protocol(
                    "prewarm specification differs from accepted intent".into(),
                ));
            }
            (bindings, record.intent.directories.clone())
        };
        let spec = spec.clone();
        let authority = self.authority.clone();
        let executable = self.executable.clone();
        let home_dir = self.home_dir.clone();
        tokio::task::spawn_blocking(move || {
            let mut assets = Sha256::new();
            hash_host_assets(&mut assets, &executable, &home_dir)?;
            compatible_scope(&authority, spec, &bindings, grants, &assets.finalize()).map(Some)
        })
        .await
        .map_err(|_| unavailable())?
    }

    fn owns_scope(&self, scope: &str) -> bool {
        scope.starts_with(&self.authority)
    }
}

fn compatible_scope(
    authority: &str,
    mut spec: RuntimeUnitSpec,
    bindings: &str,
    mut grants: Vec<super::EngineDirectoryGrant>,
    assets: &[u8],
) -> RuntimeResult<String> {
    spec.unit_id = "prewarm".into();
    spec.generation = 1;
    let template = spec.digest().map_err(RuntimeError::InvalidRequest)?;
    let mut hash = Sha256::new();
    for value in [
        "kbox.engine.prewarm.v1",
        authority,
        std::env::consts::OS,
        std::env::consts::ARCH,
        &template,
        bindings,
    ] {
        hash_part(&mut hash, OsStr::new(value));
    }
    hash.update(crate::host_sockets::current_unix_uid().to_be_bytes());
    hash.update(assets);
    grants.sort_by(|a, b| a.reference.cmp(&b.reference));
    for grant in grants {
        let metadata = std::fs::metadata(&grant.source).map_err(|_| unavailable())?;
        if !metadata.is_dir() {
            return Err(RuntimeError::InvalidRequest(
                "prewarm directory grant is no longer a directory".into(),
            ));
        }
        hash_part(&mut hash, grant.source.as_os_str());
        hash.update(metadata.dev().to_be_bytes());
        hash.update(metadata.ino().to_be_bytes());
        hash.update([u8::from(grant.read_only)]);
    }
    Ok(format!("{authority}{:x}", hash.finalize()))
}

fn unavailable() -> RuntimeError {
    RuntimeError::ProviderUnavailable("prewarm compatibility facts are unavailable".into())
}

fn hash_part(hash: &mut Sha256, value: &OsStr) {
    hash.update((value.as_bytes().len() as u64).to_be_bytes());
    hash.update(value.as_bytes());
}

fn hash_file_generation(hash: &mut Sha256, path: &Path) -> RuntimeResult<()> {
    let path = path.canonicalize().map_err(|_| unavailable())?;
    let metadata = path.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(unavailable());
    }
    hash_part(hash, path.as_os_str());
    for value in [
        metadata.dev(),
        metadata.ino(),
        metadata.size(),
        metadata.mtime() as u64,
        metadata.mtime_nsec() as u64,
        metadata.ctime() as u64,
        metadata.ctime_nsec() as u64,
    ] {
        hash.update(value.to_be_bytes());
    }
    Ok(())
}

fn hash_host_assets(hash: &mut Sha256, executable: &Path, home_dir: &Path) -> RuntimeResult<()> {
    let shim = crate::vmm::VmController::find_shim().map_err(|_| unavailable())?;
    let init = crate::vm::VmManager::find_guest_init().map_err(|_| unavailable())?;
    for path in [executable, shim.as_path(), init.as_path()] {
        hash_file_generation(hash, path)?;
    }
    let mut library_dirs = BTreeSet::from([home_dir.join("lib")]);
    if let Some(prefix) = shim.parent().and_then(Path::parent) {
        library_dirs.insert(prefix.join("lib"));
    }
    for name in ["DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            hash_part(hash, &value);
            library_dirs.extend(std::env::split_paths(&value));
        }
    }
    #[cfg(target_os = "macos")]
    library_dirs.insert(PathBuf::from("/opt/homebrew/lib"));
    let mut libraries = BTreeSet::new();
    for dir in library_dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(unavailable()),
        };
        for entry in entries {
            let entry = entry.map_err(|_| unavailable())?;
            if entry.file_name().as_bytes().starts_with(b"libkrun") && entry.path().is_file() {
                libraries.insert(entry.path());
            }
        }
    }
    for path in libraries {
        hash_file_generation(hash, &path)?;
    }
    Ok(())
}

/// Run a reserved bootstrap under the canonical execution lock, independently of the engine process.
pub async fn start_reserved_environment(
    config: BoxRuntimeDriverConfig,
    execution_id: &str,
    generation: u64,
) -> EngineResult<()> {
    let id = ExecutionId::new(execution_id).map_err(execution_error)?;
    let generation = ExecutionGeneration::new(generation).map_err(execution_error)?;
    BoxRuntimeDriver::new(config)?
        .manager
        .start_prewarmed(&id, generation)
        .await
        .map_err(execution_error)?;
    Ok(())
}

fn execution_error(error: kbox_core::ExecutionManagerError) -> EngineError {
    RuntimeError::ProviderUnavailable(error.to_string()).into()
}

impl LocalEngine {
    pub(super) async fn run_prewarm(
        self: Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> EngineResult<()> {
        let Some(worker) = &self.prewarm else {
            return Ok(());
        };
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => return Ok(()),
                _ = tick.tick() => {},
            }
            let records = self.journal.lock().await.current_requests()?;
            for record in records {
                if !record
                    .intent
                    .assembly
                    .as_ref()
                    .is_some_and(|plan| plan.prewarm_bindings_digest.is_some())
                {
                    continue;
                }
                if *shutdown.borrow() {
                    return Ok(());
                }
                let result = self
                    .replenish_environment(&record, worker, &mut shutdown)
                    .await;
                if let Err(error) = result {
                    tracing::warn!(instance_id = %record.intent.instance_id.as_str(), code = ?error.code, "environment prewarm will retry");
                }
            }
        }
    }

    async fn replenish_environment(
        &self,
        record: &AcceptedIntent,
        worker: &EnginePrewarm,
        shutdown: &mut watch::Receiver<bool>,
    ) -> EngineResult<()> {
        let owner = &record.intent.instance_id;
        let (current, unit_id) = self.current(owner).await?;
        if current.runtime_request_id != record.runtime_request_id {
            return Ok(());
        }
        let spec = record
            .intent
            .bind_service(unit_id.clone(), record.generation)?;
        let scope = self.driver.environment_scope(&spec).await?;
        tokio::select! {
            biased;
            _ = shutdown.changed() => return Ok(()),
            result = self.driver.retire_unclaimed_environments_except(owner.as_str(), scope.as_deref()) => result?,
        }
        let Some(scope) = scope else {
            return Ok(());
        };
        let lease = self.state.acquire_operation_lease(&unit_id).await?;
        let (current, _) = self.current(owner).await?;
        if current.runtime_request_id != record.runtime_request_id {
            return Ok(());
        }
        let unit = match self.state.load(&unit_id).await {
            Ok(unit) => unit,
            Err(RuntimeError::NotFound { .. }) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if unit.spec != spec
            || unit.observation.state != RuntimeUnitState::Running
            || unit.removed_at_ms.is_some()
            || !self.journal.lock().await.assembly_matches(&unit, record)?
        {
            return Ok(());
        }
        let reserved = self
            .driver
            .reserve_environment(&spec, owner.as_str(), &scope)
            .await?;
        drop(lease);
        let status = tokio::select! {
            biased;
            _ = shutdown.changed() => return Ok(()),
            result = self.driver.manager.inspect(&reserved.execution_id) => result.map_err(execution_error)?,
        };
        if status.generation != reserved.generation || status.state == ExecutionState::Running {
            return Ok(());
        }
        if status.state != ExecutionState::Created {
            return Err(EngineError::new(
                EngineErrorCode::CleanupPending,
                "prewarm reservation is not startable",
            ));
        }
        // The process can finish a durable start after engine exit. A late worker is fenced by ID and generation.
        let mut command = tokio::process::Command::new(&worker.executable);
        command
            .args([
                "engine",
                "start-reserved-environment",
                reserved.execution_id.as_str(),
            ])
            .arg(reserved.generation.get().to_string())
            .env("K_HOME", &worker.home_dir)
            .env_remove("K_SNAPSHOT_MEM_FILE")
            .env_remove("K_SNAPSHOT_SOCK")
            .env_remove("K_RESTORE_FROM")
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        let status = tokio::select! {
            biased;
            _ = shutdown.changed() => return Ok(()),
            result = command.status() => result?,
        };
        if !status.success() {
            return Err(EngineError::new(
                EngineErrorCode::Unknown,
                "reserved environment startup failed",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "prewarm_tests.rs"]
mod tests;
