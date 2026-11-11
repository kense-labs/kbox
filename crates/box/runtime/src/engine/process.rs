use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use kbox_contract::contract::RuntimeUnitState;
use kbox_core::exec::ExecExit;
use kbox_core::ExecutionProcess;
use tokio::net::UnixStream;
use tokio::sync::oneshot;

use super::{
    EngineAttachToken, EngineError, EngineErrorCode, EngineKey, EnginePrimaryInfo,
    EnginePrimaryOpen, EnginePrimaryTicket, EngineResult, LocalEngine,
};

#[derive(Default)]
pub(super) struct PrimaryConnections {
    active: BTreeSet<EngineKey>,
    waiting: BTreeMap<EngineAttachToken, Option<oneshot::Sender<UnixStream>>>,
}

pub(super) struct PendingPrimary {
    pub ticket: EnginePrimaryTicket,
    pub request: EnginePrimaryOpen,
    pub data: oneshot::Receiver<UnixStream>,
    pub owner: PrimaryOwner,
}

pub(super) struct PrimaryOwner {
    engine: Arc<LocalEngine>,
    request_id: EngineKey,
    token: EngineAttachToken,
}

impl Drop for PrimaryOwner {
    fn drop(&mut self) {
        let mut connections = self.engine.connections.lock();
        connections.active.remove(&self.request_id);
        connections.waiting.remove(&self.token);
        self.engine.wake.notify_one();
    }
}

impl LocalEngine {
    pub(super) async fn reserve_primary(
        self: &Arc<Self>,
        request: EnginePrimaryOpen,
    ) -> EngineResult<PendingPrimary> {
        request.validate()?;
        let (unit_id, existing) = {
            let journal = self.journal.lock().await;
            let existing = match journal.primary(&request.request_id) {
                Ok(_) => true,
                Err(error) if error.code == EngineErrorCode::NotFound => false,
                Err(error) => return Err(error),
            };
            (journal.unit_id(&request.instance_id), existing)
        };
        let lease = self.state.acquire_operation_lease(&unit_id).await?;
        if !existing {
            let unit = self.state.load(&unit_id).await?;
            if unit.spec.generation != request.generation {
                return Err(EngineError::new(
                    EngineErrorCode::GenerationMismatch,
                    "requested Runtime generation changed",
                ));
            }
            if unit.observation.state != RuntimeUnitState::Running {
                return Err(EngineError::new(
                    EngineErrorCode::CleanupPending,
                    "requested environment is not ready",
                ));
            }
            let journal = self.journal.lock().await;
            journal.require_assembly(&unit, journal.current(&request.instance_id)?)?;
        }
        let engine = self.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            let mut journal = engine.journal.blocking_lock();
            let info = journal.reserve_primary(&request)?;
            let token =
                EngineAttachToken(EngineKey::parse(uuid::Uuid::new_v4().simple().to_string())?);
            let (sender, data) = oneshot::channel();
            // Publish the connection owner before releasing the journal lock.
            let owner = PrimaryOwner {
                engine: engine.clone(),
                request_id: request.request_id.clone(),
                token: token.clone(),
            };
            let mut connections = engine.connections.lock();
            connections.active.insert(request.request_id.clone());
            connections.waiting.insert(token.clone(), Some(sender));
            drop(connections);
            Ok(PendingPrimary {
                ticket: EnginePrimaryTicket {
                    process: info,
                    attach_token: token,
                },
                request,
                data,
                owner,
            })
        })
        .await
        .map_err(|_| {
            EngineError::new(EngineErrorCode::Internal, "process admission worker failed")
        })?
    }

    pub(super) fn take_primary_attachment(
        &self,
        token: &EngineAttachToken,
    ) -> EngineResult<oneshot::Sender<UnixStream>> {
        self.connections
            .lock()
            .waiting
            .get_mut(token)
            .ok_or_else(|| {
                EngineError::new(
                    EngineErrorCode::StreamLost,
                    "process attachment is unavailable",
                )
            })?
            .take()
            .ok_or_else(|| {
                EngineError::new(
                    EngineErrorCode::AlreadyConnected,
                    "primary process is already attached",
                )
            })
    }

    pub(super) async fn start_primary(
        &self,
        request: &EnginePrimaryOpen,
        bootstrap: &[u8],
    ) -> EngineResult<(EnginePrimaryInfo, ExecutionProcess)> {
        let unit_id = {
            let journal = self.journal.lock().await;
            journal.require_current_primary(&request.request_id)?;
            journal.unit_id(&request.instance_id)
        };
        let lease = self.state.acquire_operation_lease(&unit_id).await?;
        self.journal
            .lock()
            .await
            .require_current_primary(&request.request_id)?;
        let unit = self.state.load(&unit_id).await?;
        if unit.spec.generation != request.generation {
            return Err(EngineError::new(
                EngineErrorCode::GenerationMismatch,
                "Runtime generation changed before process launch",
            ));
        }
        let assembly_binding = {
            let journal = self.journal.lock().await;
            let record = journal.current(&request.instance_id)?;
            journal.require_assembly(&unit, record)?;
            journal
                .data
                .assemblies
                .get(&record.runtime_request_id)
                .map(|receipt| (receipt.execution_id.clone(), receipt.execution_generation))
        };
        let (execution_id, generation, process) = self
            .driver
            .start_runtime_process_bound(
                &unit,
                request.exec_request()?,
                assembly_binding
                    .as_ref()
                    .map(|(id, generation)| (id.as_str(), *generation)),
            )
            .await?;
        if !bootstrap.is_empty() {
            let input = process.input();
            tokio::time::timeout(super::transport::CONTROL_TIMEOUT, async {
                for bytes in bootstrap.chunks(super::process_transport::MAX_PROCESS_DATA_BYTES) {
                    input
                        .write_stdin(bytes)
                        .await
                        .map_err(|_| bootstrap_lost())?;
                }
                Ok::<(), EngineError>(())
            })
            .await
            .map_err(|_| bootstrap_lost())??;
        }
        let journal = self.journal.clone();
        let id = request.request_id.clone();
        let info = tokio::task::spawn_blocking(move || {
            let _lease = lease;
            journal.blocking_lock().primary_started(
                &id,
                execution_id.as_str().into(),
                generation.get(),
            )
        })
        .await
        .map_err(|_| {
            EngineError::new(
                EngineErrorCode::Internal,
                "process launch receipt worker failed",
            )
        })??;
        Ok((info, process))
    }

    pub(super) async fn finish_primary(
        &self,
        request_id: &EngineKey,
        exit: Option<ExecExit>,
    ) -> EngineResult<()> {
        let unit_id = {
            let journal = self.journal.lock().await;
            journal.unit_id(&journal.primary(request_id)?.info.instance_id)
        };
        let lease = self.state.acquire_operation_lease(&unit_id).await?;
        let journal = self.journal.clone();
        let request_id = request_id.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            journal.blocking_lock().finish_primary(&request_id, exit)
        })
        .await
        .map_err(|_| {
            EngineError::new(
                EngineErrorCode::Internal,
                "process completion worker failed",
            )
        })??;
        self.wake.notify_one();
        Ok(())
    }

    pub(super) async fn orphaned_primary_instances(&self) -> EngineResult<BTreeSet<EngineKey>> {
        let journal = self.journal.lock().await;
        let unfinished = journal.unfinished_primaries()?;
        let connections = self.connections.lock();
        Ok(unfinished
            .into_iter()
            .filter(|info| !connections.active.contains(&info.request_id))
            .map(|info| info.instance_id)
            .collect())
    }

    pub(super) async fn unfinished_primary_instances(&self) -> EngineResult<BTreeSet<EngineKey>> {
        Ok(self
            .journal
            .lock()
            .await
            .unfinished_primaries()?
            .into_iter()
            .map(|info| info.instance_id)
            .collect())
    }

    pub(super) async fn close_primary_instances(
        &self,
        instances: BTreeSet<EngineKey>,
    ) -> EngineResult<()> {
        for instance in instances {
            self.retire_orphaned_primaries(&instance).await?;
            let (record, unit) = self.current(&instance).await?;
            if record.intent.desired_state == super::DesiredState::Up {
                self.retire_previous(&record, &unit).await?;
            } else {
                self.reconcile(&record).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn retire_orphaned_primaries(
        &self,
        instance_id: &EngineKey,
    ) -> EngineResult<()> {
        let orphaned = {
            let journal = self.journal.lock().await;
            let unfinished = journal.unfinished_primaries()?;
            let connections = self.connections.lock();
            unfinished
                .into_iter()
                .filter(|info| {
                    info.instance_id == *instance_id
                        && !connections.active.contains(&info.request_id)
                })
                .map(|info| info.request_id)
                .collect::<Vec<_>>()
        };
        for id in orphaned {
            self.finish_primary(&id, None).await?;
        }
        Ok(())
    }
}

fn bootstrap_lost() -> EngineError {
    EngineError::new(
        EngineErrorCode::StreamLost,
        "primary bootstrap was not delivered",
    )
}
