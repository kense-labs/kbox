use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use kbox_contract::contract::{
    RuntimeActionRequest, RuntimeApplyRequest, RuntimeInspection, RuntimeLogQuery,
};
use kbox_contract::{
    FileRuntimeStateStore, ManagedRuntimeClient, RuntimeClient, RuntimeDriver, RuntimeError,
    RuntimeStateStore,
};
use tokio::sync::{watch, Mutex, Notify};
use tokio::task::JoinSet;

use crate::{BoxRuntimeDriver, BoxRuntimeDriverConfig};

use super::prewarm::EnginePrewarm;
use super::process::PrimaryConnections;
use super::protocol::AcceptedIntent;
use super::{
    AcceptedOperation, DesiredState, EngineCommand, EngineDiscovery, EngineError, EngineErrorCode,
    EngineExecution, EngineIntent, EngineJournal, EngineKey, EngineOperation, EngineReplyValue,
    EngineRequest, EngineResult, EngineRevisionGuard,
};

/// Accepted intent and delivery only. Runtime receipts and ExecutionManager remain authoritative.
pub struct LocalEngine {
    pub(super) journal: Arc<Mutex<EngineJournal>>,
    pub(super) state: Arc<dyn RuntimeStateStore>,
    runtime: Arc<ManagedRuntimeClient>,
    pub(super) driver: Arc<BoxRuntimeDriver>,
    pub(super) prewarm: Option<Arc<EnginePrewarm>>,
    pub(super) wake: Notify,
    pub(super) connections: parking_lot::Mutex<PrimaryConnections>,
    errors: Mutex<BTreeMap<EngineKey, EngineErrorCode>>,
}

impl std::fmt::Debug for LocalEngine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalEngine")
            .finish_non_exhaustive()
    }
}

impl LocalEngine {
    /// Performs filesystem I/O; call outside async executor threads.
    pub fn open(config: BoxRuntimeDriverConfig) -> EngineResult<Self> {
        let directory = config.home_dir.join("engine");
        let journal = Self::open_journal(&directory)?;
        let prewarm = Arc::new(EnginePrewarm::new(
            journal.clone(),
            config.home_dir.clone(),
        )?);
        let driver = Arc::new(
            BoxRuntimeDriver::new(config)?
                .with_directory_port(Arc::new(super::directory::EngineDirectories(
                    journal.clone(),
                )))
                .with_prewarm_scope_port(prewarm.clone()),
        );
        Self::with_journal(directory, driver, journal, Some(prewarm))
    }

    #[cfg(test)]
    pub(crate) fn with_runtime(
        directory: std::path::PathBuf,
        driver: Arc<BoxRuntimeDriver>,
    ) -> EngineResult<Self> {
        let journal = Self::open_journal(&directory)?;
        Self::with_journal(directory, driver, journal, None)
    }

    fn open_journal(directory: &std::path::Path) -> EngineResult<Arc<Mutex<EngineJournal>>> {
        Ok(Arc::new(Mutex::new(EngineJournal::open(
            directory.join("intent.json"),
        )?)))
    }

    fn with_journal(
        directory: std::path::PathBuf,
        driver: Arc<BoxRuntimeDriver>,
        journal: Arc<Mutex<EngineJournal>>,
        prewarm: Option<Arc<EnginePrewarm>>,
    ) -> EngineResult<Self> {
        let state: Arc<dyn RuntimeStateStore> =
            Arc::new(FileRuntimeStateStore::new(directory.join("runtime")));
        let runtime = Arc::new(
            ManagedRuntimeClient::new(state.clone(), driver.clone())
                .with_admission_guard(Arc::new(EngineRevisionGuard::new(journal.clone()))),
        );
        Ok(Self {
            journal,
            state,
            runtime,
            driver,
            prewarm,
            wake: Notify::new(),
            connections: parking_lot::Mutex::new(PrimaryConnections::default()),
            errors: Mutex::new(BTreeMap::new()),
        })
    }

    pub async fn dispatch(&self, request: EngineRequest) -> EngineResult<EngineReplyValue> {
        request.validate()?;
        match request.command {
            EngineCommand::Discover => {
                let mut protocols = vec![
                    "kbox.engine.v1".into(),
                    "process-stream.primary.v1".into(),
                    super::process_protocol::BOOTSTRAP_PROTOCOL.into(),
                    "harness-assembly.v1".into(),
                    super::assembly::GUEST_PROTOCOLS_CAPABILITY.into(),
                ];
                if self.prewarm.is_some() {
                    protocols.push("environment-prewarm.v1".into());
                }
                if self
                    .driver
                    .capabilities()
                    .await?
                    .mount_kinds
                    .contains(&kbox_contract::contract::MountKind::Directory)
                {
                    protocols.push("directory-grants.v1".into());
                }
                Ok(EngineReplyValue::Discovery(EngineDiscovery {
                    schema: "kbox.engine.discovery.v1".into(),
                    engine_id: self.journal.lock().await.engine_id().clone(),
                    protocols,
                    platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
                    max_control_frame_bytes: super::transport::MAX_CONTROL_FRAME_BYTES as u32,
                }))
            }
            EngineCommand::ApplyIntent { intent } => {
                self.accept(*intent).await.map(EngineReplyValue::Accepted)
            }
            EngineCommand::InspectExecution { instance_id } => self
                .inspect_execution(&instance_id)
                .await
                .map(EngineReplyValue::Execution),
            EngineCommand::InspectOperation { request_id } => self
                .inspect_operation(&request_id)
                .await
                .map(|operation| EngineReplyValue::Operation(Box::new(operation))),
            EngineCommand::InspectAssembly { instance_id } => self
                .inspect_assembly(&instance_id)
                .await
                .map(EngineReplyValue::Assembly),
            EngineCommand::ReadLogs {
                instance_id,
                cursor,
            } => {
                let (record, unit) = self.current(&instance_id).await?;
                let query = RuntimeLogQuery {
                    schema: RuntimeLogQuery::SCHEMA.into(),
                    unit_id: unit,
                    generation: record.generation,
                    cursor,
                    limit: 128,
                    stream: None,
                };
                Ok(EngineReplyValue::Logs(self.runtime.logs(&query).await?))
            }
            EngineCommand::InspectPrimaryProcess { request_id } => {
                Ok(EngineReplyValue::PrimaryProcess(
                    self.journal.lock().await.primary(&request_id)?.info.clone(),
                ))
            }
            EngineCommand::OpenPrimaryProcess { .. }
            | EngineCommand::AttachPrimaryProcess { .. } => Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "process streams require the stream transport",
            )),
        }
    }

    pub async fn accept(&self, intent: EngineIntent) -> EngineResult<AcceptedOperation> {
        intent.validate()?;
        if self.prewarm.is_none()
            && intent
                .assembly
                .as_ref()
                .is_some_and(|plan| plan.prewarm_bindings_digest.is_some())
        {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedCapability,
                "engine has no environment prewarm worker",
            ));
        }
        let unit_id = self.journal.lock().await.unit_id(&intent.instance_id);
        // Serialize acceptance with provider effects, using the existing cross-process unit lease.
        let lease = self.state.acquire_operation_lease(&unit_id).await?;
        let journal = self.journal.clone();
        let accepted = tokio::task::spawn_blocking(move || {
            let _lease = lease;
            journal.blocking_lock().accept(intent)
        })
        .await
        .map_err(join_error)??;
        self.wake.notify_one();
        Ok(accepted)
    }

    pub(super) async fn current(
        &self,
        instance_id: &EngineKey,
    ) -> EngineResult<(AcceptedIntent, String)> {
        let journal = self.journal.lock().await;
        Ok((
            journal.current(instance_id)?.clone(),
            journal.unit_id(instance_id),
        ))
    }

    async fn inspect_execution(&self, instance_id: &EngineKey) -> EngineResult<EngineExecution> {
        let (record, unit) = self.current(instance_id).await?;
        let runtime = match self.runtime.inspect(&unit).await {
            Ok(inspection) => Some(inspection),
            Err(RuntimeError::NotFound { .. }) => None,
            Err(error) => return Err(error.into()),
        };
        let mut last_error = self
            .errors
            .lock()
            .await
            .get(&record.intent.request_id)
            .copied();
        if record.intent.desired_state == DesiredState::Down
            && matches!(&runtime, Some(RuntimeInspection::NotFound { .. }))
            && last_error.is_none()
        {
            let had_prewarm = self.journal.lock().await.has_prewarm_history(instance_id);
            if had_prewarm
                && !self
                    .driver
                    .unclaimed_environments(instance_id.as_str())
                    .await?
                    .is_empty()
            {
                last_error = Some(EngineErrorCode::CleanupPending);
            }
        }
        Ok(EngineExecution {
            last_error,
            accepted: record.accepted,
            target_generation: record.generation,
            desired_state: record.intent.desired_state,
            runtime,
        })
    }

    async fn inspect_operation(&self, request_id: &EngineKey) -> EngineResult<EngineOperation> {
        let (record, unit, superseded) = {
            let journal = self.journal.lock().await;
            let record = journal.request(request_id)?.clone();
            let superseded = journal
                .current(&record.intent.instance_id)?
                .intent
                .request_id
                != *request_id;
            let unit = journal.unit_id(&record.intent.instance_id);
            (record, unit, superseded)
        };
        let id = if record.intent.desired_state == DesiredState::Down {
            format!("{}-remove", record.runtime_request_id)
        } else {
            record.runtime_request_id.clone()
        };
        let runtime = match self.state.load_request(&unit, &id).await {
            Ok(receipt) => Some(receipt),
            Err(RuntimeError::RequestNotFound { .. } | RuntimeError::NotFound { .. }) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(EngineOperation {
            accepted: record.accepted,
            target_generation: record.generation,
            superseded,
            runtime,
            last_error: self.errors.lock().await.get(request_id).copied(),
        })
    }

    pub(super) async fn reconcile(&self, record: &AcceptedIntent) -> EngineResult<()> {
        self.retire_orphaned_primaries(&record.intent.instance_id)
            .await?;
        let (current, unit_id) = self.current(&record.intent.instance_id).await?;
        if current.runtime_request_id != record.runtime_request_id {
            return Ok(());
        }
        if self.prewarm.is_some()
            && !record
                .intent
                .assembly
                .as_ref()
                .is_some_and(|plan| plan.prewarm_bindings_digest.is_some())
        {
            let had_prewarm = self
                .journal
                .lock()
                .await
                .has_prewarm_history(&record.intent.instance_id);
            if had_prewarm {
                self.driver
                    .retire_unclaimed_environments(record.intent.instance_id.as_str())
                    .await?;
            }
        }
        match record.intent.desired_state {
            DesiredState::Up => {
                self.retire_previous(record, &unit_id).await?;
                let request = RuntimeApplyRequest {
                    schema: RuntimeApplyRequest::SCHEMA.into(),
                    request_id: record.runtime_request_id.clone(),
                    deadline_at_ms: None,
                    spec: record
                        .intent
                        .bind_service(unit_id.clone(), record.generation)?,
                    restore_from_snapshot_operation_id: None,
                };
                self.runtime.apply(&request).await?;
                self.assemble(record, &unit_id).await?;
            }
            DesiredState::Down => {
                let unit = match self.state.load(&unit_id).await {
                    Ok(unit) => unit,
                    Err(RuntimeError::NotFound { .. }) => return Ok(()),
                    Err(error) => return Err(error.into()),
                };
                if unit.removed_at_ms.is_some() {
                    return Ok(());
                }
                let mut action = RuntimeActionRequest {
                    schema: RuntimeActionRequest::SCHEMA.into(),
                    request_id: format!("{}-stop", record.runtime_request_id),
                    unit_id,
                    generation: unit.spec.generation,
                    deadline_at_ms: None,
                };
                self.runtime.stop(&action).await?;
                action.request_id = format!("{}-remove", record.runtime_request_id);
                self.runtime.remove(&action).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn retire_previous(
        &self,
        record: &AcceptedIntent,
        unit_id: &str,
    ) -> EngineResult<()> {
        match self.state.load(unit_id).await {
            Ok(previous)
                if previous.spec.generation < record.generation
                    && previous.removed_at_ms.is_none() =>
            {
                let mut action = RuntimeActionRequest {
                    schema: RuntimeActionRequest::SCHEMA.into(),
                    request_id: format!("{}-replace-stop", record.runtime_request_id),
                    unit_id: unit_id.into(),
                    generation: previous.spec.generation,
                    deadline_at_ms: None,
                };
                self.runtime.stop(&action).await?;
                action.request_id = format!("{}-replace-remove", record.runtime_request_id);
                self.runtime.remove(&action).await?;
            }
            Ok(_) | Err(RuntimeError::NotFound { .. }) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    pub(super) async fn run(
        self: Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> EngineResult<()> {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut jobs = JoinSet::new();
        let mut active = BTreeSet::new();
        let mut settled = BTreeMap::new();
        let mut retry_at = BTreeMap::new();
        loop {
            tokio::select! {
                biased;
                result = shutdown.changed() => { if result.is_err() || *shutdown.borrow() { break; } },
                Some(result) = jobs.join_next(), if !jobs.is_empty() => {
                    let (record, result): (AcceptedIntent, EngineResult<()>) = result.map_err(join_error)?;
                    active.remove(&record.intent.instance_id);
                    let mut errors = self.errors.lock().await;
                    match result {
                        Ok(()) => {
                            errors.remove(&record.intent.request_id);
                            settled.insert(record.intent.instance_id.clone(), record.runtime_request_id.clone());
                        }
                        Err(error) => {
                            retry_at.insert(record.intent.request_id.clone(), tokio::time::Instant::now() + Duration::from_secs(1));
                            errors.insert(record.intent.request_id, error.code);
                        }
                    }
                    self.wake.notify_one();
                }
                _ = self.wake.notified() => {},
                _ = tick.tick() => {},
            }
            let records = self.journal.lock().await.current_requests()?;
            let orphaned = self.orphaned_primary_instances().await?;
            for record in records {
                if (settled.get(&record.intent.instance_id) == Some(&record.runtime_request_id)
                    && !orphaned.contains(&record.intent.instance_id))
                    || retry_at
                        .get(&record.intent.request_id)
                        .is_some_and(|at| *at > tokio::time::Instant::now())
                {
                    continue;
                }
                if active.insert(record.intent.instance_id.clone()) {
                    let engine = self.clone();
                    jobs.spawn(async move {
                        let result = engine.reconcile(&record).await;
                        (record, result)
                    });
                }
            }
        }
        jobs.abort_all();
        while jobs.join_next().await.is_some() {}
        Ok(())
    }
}

fn join_error(_: tokio::task::JoinError) -> EngineError {
    EngineError::new(EngineErrorCode::Internal, "engine worker failed")
}
