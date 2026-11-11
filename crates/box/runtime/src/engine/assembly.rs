//! MateHarness package preparation through the existing generation-fenced process authority.
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use kbox_contract::contract::RuntimeUnitState;
use kbox_contract::RuntimeUnitRecord;
use kbox_core::exec::{ExecEvent, ExecRequest, StreamType, UNLIMITED_STREAM_TIMEOUT_NS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use super::protocol::AcceptedIntent;
use super::{
    AcceptedOperation, DesiredState, EngineError, EngineErrorCode, EngineIntent, EngineKey,
    EngineResult, EngineWorkload, LocalEngine,
};

const MAX_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_RECEIPT_BYTES: usize = 16 * 1024 * 1024;
const INSTALLER: &str = "/opt/krun/bin/krun";
const PACKAGE_ROOT: &str = "/opt/krun/packages";
pub(super) const GUEST_PROTOCOLS_CAPABILITY: &str = "harness-assembly.guest-protocols.v1";
pub(super) const PACKAGES_CAPABILITY: &str = "harness-assembly.capability-packages.v1";

struct ArchiveCopyGuard(Arc<AtomicBool>);
impl Drop for ArchiveCopyGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineAssemblyPlan {
    pub archive: PathBuf,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_packages: Vec<EngineAssemblyPackage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prewarm_bindings_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_guest_protocol: Option<EngineKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_guest_protocols: Vec<EngineKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineAssemblyPackage {
    pub archive: PathBuf,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineInstalledPackage {
    pub digest: String,
    pub root: String,
}

impl EngineAssemblyPlan {
    pub(super) fn validate(&self, intent: &EngineIntent) -> EngineResult<()> {
        let mut protocols = std::collections::BTreeSet::new();
        if self
            .required_protocols()
            .any(|protocol| !protocols.insert(protocol))
        {
            return Err(invalid());
        }
        let mut digests = std::collections::BTreeSet::from([self.digest.as_str()]);
        if self.capability_packages.iter().any(|package| {
            !package.archive.is_absolute()
                || !valid_digest(&package.digest)
                || !digests.insert(package.digest.as_str())
        }) || (!self.capability_packages.is_empty()
            && !self
                .required_protocols()
                .any(|protocol| protocol.as_str() == "capability-packages.v1"))
        {
            return Err(invalid());
        }
        if intent.desired_state != DesiredState::Up
            || intent.workload != EngineWorkload::AcpHarness
            || !self.archive.is_absolute()
            || self.archive.as_os_str().is_empty()
            || !valid_digest(&self.digest)
            || self
                .prewarm_bindings_digest
                .as_deref()
                .is_some_and(|digest| !valid_digest(digest))
        {
            return Err(invalid());
        }
        let service = intent.service.as_ref().ok_or_else(invalid)?;
        if service.process.command != ["/bin/sleep"]
            || service.process.args.len() != 1
            || !matches!(service.process.args[0].as_str(), "infinity" | "2147483647")
        {
            return Err(invalid());
        }
        if self.prewarm_bindings_digest.is_some() {
            let mut spec = service.bind("engine-validation".into(), 1)?;
            spec.semantics_profile_digest =
                Some(crate::kbox_contract_driver::NATIVE_GUEST_PROFILE_DIGEST.into());
            if !crate::kbox_contract_driver::prewarm::eligible(&spec) {
                return Err(invalid());
            }
        }
        for mount in &service.mounts {
            let target = Path::new(&mount.target);
            if target.starts_with("/opt/krun") || Path::new("/opt/krun").starts_with(target) {
                return Err(invalid());
            }
        }
        for secret in &service.secrets {
            if let kbox_contract::contract::SecretTarget::File { path, .. } = &secret.target {
                if Path::new(path).starts_with("/opt/krun")
                    || Path::new("/opt/krun").starts_with(path)
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }

    pub(super) fn root(&self) -> String {
        package_root(&self.digest)
    }

    pub(super) fn installed_capabilities(&self) -> Vec<EngineInstalledPackage> {
        self.capability_packages
            .iter()
            .map(|package| EngineInstalledPackage {
                digest: package.digest.clone(),
                root: package_root(&package.digest),
            })
            .collect()
    }

    fn required_protocols(&self) -> impl Iterator<Item = &EngineKey> {
        self.required_guest_protocol
            .iter()
            .chain(&self.required_guest_protocols)
    }
}

fn package_root(digest: &str) -> String {
    format!(
        "{PACKAGE_ROOT}/{}",
        digest.strip_prefix("sha256:").unwrap_or("")
    )
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineAssemblyReceipt {
    pub schema: String,
    pub accepted: AcceptedOperation,
    pub runtime_request_id: String,
    pub generation: u64,
    pub spec_digest: String,
    pub execution_id: String,
    pub execution_generation: u64,
    pub package_digest: String,
    pub root: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_packages: Vec<EngineInstalledPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineAssembly {
    pub accepted: AcceptedOperation,
    pub target_generation: u64,
    pub receipt: Option<EngineAssemblyReceipt>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuestInstallation {
    schema: String,
    digest: String,
    root: String,
    #[serde(default)]
    protocols: Vec<EngineKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuestInstallationFailure {
    schema: String,
    digest: String,
    protocol: EngineKey,
    code: EngineErrorCode,
}

impl GuestInstallationFailure {
    fn decode(stdout: &[u8], digest: &str, plan: &EngineAssemblyPlan) -> Option<EngineError> {
        let failure: Self = serde_json::from_slice(stdout).ok()?;
        if failure.schema != "krun.guest-installation-error.v1"
            || failure.digest != digest
            || !plan
                .required_protocols()
                .any(|protocol| protocol == &failure.protocol)
        {
            return None;
        }
        let message = match failure.code {
            EngineErrorCode::RuntimeRequirementsUnmet => {
                "the guest image does not satisfy the package runtime requirements"
            }
            EngineErrorCode::UnsupportedCapability => {
                "the guest image does not provide the required assembly protocol"
            }
            _ => return None,
        };
        Some(EngineError::new(failure.code, message))
    }
}

impl GuestInstallation {
    fn validate(&self, plan: &EngineAssemblyPlan) -> EngineResult<()> {
        self.validate_package(&plan.digest, false, plan)
    }

    fn validate_package(
        &self,
        digest: &str,
        capability: bool,
        plan: &EngineAssemblyPlan,
    ) -> EngineResult<()> {
        let schema = if capability {
            "krun.guest-capability-installation.v1"
        } else {
            "krun.guest-installation.v1"
        };
        if self.schema != schema || self.digest != digest || self.root != package_root(digest) {
            return Err(failed());
        }
        if plan
            .required_protocols()
            .any(|protocol| !self.protocols.contains(protocol))
        {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedCapability,
                "guest image does not provide the required launch protocol",
            ));
        }
        Ok(())
    }
}

impl LocalEngine {
    pub(super) async fn inspect_assembly(
        &self,
        instance: &EngineKey,
    ) -> EngineResult<EngineAssembly> {
        let (current, unit_id) = self.current(instance).await?;
        let receipt = if current.intent.desired_state == DesiredState::Up {
            self.journal
                .lock()
                .await
                .data
                .assemblies
                .get(&current.runtime_request_id)
                .cloned()
        } else {
            None
        };
        if let Some(receipt) = &receipt {
            let unit = self.state.load(&unit_id).await?;
            require_running(&unit, &current)?;
            let (id, generation) = self.driver.running_execution(&unit).await?;
            if id.as_str() != receipt.execution_id
                || generation.get() != receipt.execution_generation
            {
                return Err(EngineError::new(
                    EngineErrorCode::GenerationMismatch,
                    "assembly belongs to a different Box execution",
                ));
            }
        }
        Ok(EngineAssembly {
            accepted: current.accepted,
            target_generation: current.generation,
            receipt,
        })
    }

    pub(super) async fn assemble(
        &self,
        record: &AcceptedIntent,
        unit_id: &str,
    ) -> EngineResult<()> {
        let Some(plan) = &record.intent.assembly else {
            return Ok(());
        };
        let lease = self.state.acquire_operation_lease(unit_id).await?;
        self.journal.lock().await.require_assembly_intent(record)?;
        let unit = self.state.load(unit_id).await?;
        require_running(&unit, record)?;
        if self.journal.lock().await.assembly_matches(&unit, record)? {
            return Ok(());
        }
        drop(lease);
        let (execution_id, execution_generation) = self
            .install_package(
                record,
                unit_id,
                &EngineAssemblyPackage {
                    archive: plan.archive.clone(),
                    digest: plan.digest.clone(),
                },
                false,
            )
            .await?;
        for package in &plan.capability_packages {
            let installed = self.install_package(record, unit_id, package, true).await?;
            if installed != (execution_id.clone(), execution_generation) {
                return Err(EngineError::new(
                    EngineErrorCode::GenerationMismatch,
                    "packages were installed into different Box executions",
                ));
            }
        }
        let lease = self.state.acquire_operation_lease(unit_id).await?;
        let latest = self.state.load(unit_id).await?;
        require_running(&latest, record)?;
        let receipt = EngineAssemblyReceipt {
            schema: "kbox.engine.assembly.v1".into(),
            accepted: record.accepted.clone(),
            runtime_request_id: record.runtime_request_id.clone(),
            generation: record.generation,
            spec_digest: latest.spec.digest().map_err(|_| invalid())?,
            execution_id,
            execution_generation,
            package_digest: plan.digest.clone(),
            root: plan.root(),
            capability_packages: plan.installed_capabilities(),
        };
        if latest.observation.provider_resource_id.as_deref() != Some(receipt.execution_id.as_str())
        {
            return Err(failed());
        }
        let journal = self.journal.clone();
        let record = record.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            journal.blocking_lock().publish_assembly(&record, receipt)
        })
        .await
        .map_err(|_| failed())?
    }

    async fn install_package(
        &self,
        record: &AcceptedIntent,
        unit_id: &str,
        package: &EngineAssemblyPackage,
        capability: bool,
    ) -> EngineResult<(String, u64)> {
        let plan = record.intent.assembly.as_ref().ok_or_else(invalid)?;
        let package_copy = package.clone();
        // Verify an immutable anonymous copy before any archive byte crosses into the guest.
        let source = {
            let cancelled = Arc::new(AtomicBool::new(false));
            let _cancel = ArchiveCopyGuard(cancelled.clone());
            let stage = tokio::task::spawn_blocking(move || {
                verified_archive(&package_copy.archive, &package_copy.digest, &cancelled)
            });
            tokio::select! {
                result=stage=>result.map_err(|_| failed())??,
                _=self.wait_assembly_superseded(record)=>return Err(EngineError::new(EngineErrorCode::StaleIntent,"archive copy intent changed")),
            }
        };
        let lease = self.state.acquire_operation_lease(unit_id).await?;
        self.journal.lock().await.require_assembly_intent(record)?;
        let unit = self.state.load(unit_id).await?;
        require_running(&unit, record)?;
        let (execution_id, execution_generation, mut process) = self
            .driver
            .start_runtime_process(
                &unit,
                ExecRequest {
                    request_id: None,
                    cmd: vec![
                        INSTALLER.into(),
                        if capability {
                            "_guest-install-capability"
                        } else {
                            "_guest-install"
                        }
                        .into(),
                        "--digest".into(),
                        package.digest.clone(),
                    ],
                    timeout_ns: UNLIMITED_STREAM_TIMEOUT_NS,
                    env: Vec::new(),
                    working_dir: Some("/".into()),
                    rootfs: None,
                    stdin: None,
                    stdin_streaming: true,
                    user: None,
                    streaming: true,
                },
            )
            .await?;
        drop(lease);
        let input = process.input();
        let transfer = async {
            let send = async {
                let mut source = tokio::fs::File::from_std(source);
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    let count = source.read(&mut buffer).await.map_err(|_| failed())?;
                    if count == 0 {
                        break;
                    }
                    input
                        .write_stdin(&buffer[..count])
                        .await
                        .map_err(|_| failed())?;
                }
                input.close_stdin().await.map_err(|_| failed())
            };
            let receive = async {
                let mut stdout = Vec::new();
                let mut total = 0usize;
                let mut exit = None;
                while let Some(event) = process.next_event().await.map_err(|_| failed())? {
                    match event {
                        ExecEvent::Chunk(chunk) if exit.is_none() => {
                            total = total.checked_add(chunk.data.len()).ok_or_else(failed)?;
                            if total > MAX_RECEIPT_BYTES {
                                return Err(failed());
                            }
                            if chunk.stream == StreamType::Stdout {
                                stdout.extend(chunk.data);
                            }
                        }
                        ExecEvent::Exit(result) if exit.is_none() => exit = Some(result),
                        ExecEvent::FlushAck => {}
                        _ => return Err(failed()),
                    }
                }
                let exit = exit.ok_or_else(failed)?;
                if exit.oom_killed {
                    return Err(failed());
                }
                if exit.exit_code != 0 {
                    return Err(
                        GuestInstallationFailure::decode(&stdout, &package.digest, plan)
                            .unwrap_or_else(failed),
                    );
                }
                let reply: GuestInstallation =
                    serde_json::from_slice(&stdout).map_err(|_| failed())?;
                if capability {
                    reply.validate_package(&package.digest, true, plan)
                } else {
                    reply.validate(plan)
                }
            };
            tokio::try_join!(send, receive).map(|_| ())
        };
        let result = tokio::select! {
            result = transfer => result,
            _ = self.wait_assembly_superseded(record) => Err(EngineError::new(EngineErrorCode::StaleIntent,"assembly intent changed")),
        };
        if let Err(error) = result {
            let _ = tokio::time::timeout(Duration::from_secs(5), input.cancel()).await;
            return Err(error);
        }
        Ok((execution_id.as_str().into(), execution_generation.get()))
    }

    async fn wait_assembly_superseded(&self, record: &AcceptedIntent) {
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        loop {
            tick.tick().await;
            if self
                .journal
                .lock()
                .await
                .require_assembly_intent(record)
                .is_err()
            {
                return;
            }
        }
    }
}

fn require_running(unit: &RuntimeUnitRecord, record: &AcceptedIntent) -> EngineResult<()> {
    if unit.spec
        != record
            .intent
            .bind_service(unit.spec.unit_id.clone(), record.generation)?
        || unit.removed_at_ms.is_some()
        || unit.observation.state != RuntimeUnitState::Running
    {
        return Err(EngineError::new(
            EngineErrorCode::CleanupPending,
            "assembly requires the current running environment",
        ));
    }
    Ok(())
}

fn verified_archive(archive: &Path, digest: &str, cancelled: &AtomicBool) -> EngineResult<File> {
    let canonical = archive.canonicalize()?;
    if canonical != archive {
        return Err(invalid());
    }
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(archive)?;
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(invalid());
    }
    let mut copy = tempfile::tempfile()?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(EngineError::new(
                EngineErrorCode::StaleIntent,
                "archive copy was cancelled",
            ));
        }
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_ARCHIVE_BYTES {
            return Err(invalid());
        }
        hash.update(&buffer[..count]);
        copy.write_all(&buffer[..count])?;
    }
    if format!("sha256:{:x}", hash.finalize()) != digest {
        return Err(invalid());
    }
    copy.rewind()?;
    Ok(copy)
}

fn invalid() -> EngineError {
    EngineError::new(
        EngineErrorCode::InvalidRequest,
        "invalid Harness package assembly plan",
    )
}
fn failed() -> EngineError {
    EngineError::new(
        EngineErrorCode::AssemblyFailed,
        "guest package installation did not produce a valid receipt",
    )
}

#[cfg(test)]
#[path = "assembly_tests.rs"]
mod tests;
