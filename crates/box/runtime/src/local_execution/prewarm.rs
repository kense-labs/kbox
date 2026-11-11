//! Clean environment reservations and atomic handoff in the canonical execution store.
use kbox_core::{
    BoxError, CreateExecutionRequest, ExecutionGeneration, ExecutionId, ExecutionLease,
    ExecutionManagerError, ExecutionManagerResult, ExecutionReservation, ExecutionState,
    OperationId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::record::{build_managed_record, reservation_from_record};
use super::store::run_store;
use super::support::managed_state;
use super::LocalExecutionManager;
use crate::{
    BoxRecord, BoxStateStore, ManagedExecutionMetadata, ManagedExecutionState,
    ManagedExecutionStoreError,
};

/// Provenance, not a second resource state. Only the manager can mint a reservation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedPrewarm {
    scope: String,
    owner: String,
    compatibility_digest: String,
    operation_id: OperationId,
    generation: ExecutionGeneration,
}

impl ManagedPrewarm {
    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn compatibility_digest(&self) -> &str {
        &self.compatibility_digest
    }
    pub(crate) fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    pub fn is_unclaimed(&self, metadata: &ManagedExecutionMetadata) -> bool {
        self.operation_id == metadata.operation_id
    }

    pub(crate) fn validate(&self, metadata: &ManagedExecutionMetadata) -> kbox_core::Result<()> {
        let valid = valid_identity(&self.scope)
            && valid_identity(&self.owner)
            && self
                .compatibility_digest
                .strip_prefix("sha256:")
                .is_some_and(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                });
        let consistent = if self.is_unclaimed(metadata) {
            metadata.generation == self.generation
                && self.generation == ExecutionGeneration::INITIAL
                && validate_bootstrap(&metadata.request).is_ok()
                && compatibility(&metadata.request)
                    .is_ok_and(|key| key == self.compatibility_digest)
        } else {
            metadata.generation.get() > self.generation.get()
        };
        if !valid || !consistent {
            return Err(BoxError::StateError(
                "invalid prewarm reservation provenance".into(),
            ));
        }
        Ok(())
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

fn validate_bootstrap(request: &CreateExecutionRequest) -> ExecutionManagerResult<()> {
    let config = &request.config;
    let pinned = config
        .image
        .rsplit_once("@sha256:")
        .is_some_and(|(_, digest)| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
    if !pinned
        || !config.extra_env.is_empty()
        || config.entrypoint_override.as_deref() != Some(&["/bin/sleep".to_string()][..])
        || config.cmd != ["infinity"]
        || config.tee != kbox_core::config::TeeConfig::None
        || config.deferred_main
        || config.snapshot_mem_file.is_some()
        || config.snapshot_sock.is_some()
        || config.restore_from.is_some()
        || request.rootfs_snapshot_id.is_some()
        || request.policy.managed_secret_root.is_some()
        || request.policy.health_check.is_some()
        || request.policy.auto_remove
        || request.policy.restart_policy != kbox_core::ExecutionRestartPolicy::No
    {
        return Err(ExecutionManagerError::InvalidRequest(
            "prewarm requires a secret-free idle bootstrap".into(),
        ));
    }
    Ok(())
}

/// The complete boot policy, including mounts and resource limits, excludes caller identities.
fn compatibility(request: &CreateExecutionRequest) -> ExecutionManagerResult<String> {
    let mut value = serde_json::to_value(request).map_err(|_| invalid_encoding())?;
    let object = value.as_object_mut().ok_or_else(invalid_encoding)?;
    object.remove("external_sandbox_id");
    object.remove("labels");
    object
        .get_mut("policy")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(invalid_encoding)?
        .remove("name");
    let bytes = serde_json::to_vec(&value).map_err(|_| invalid_encoding())?;
    let mut hash = Sha256::new();
    hash.update(b"kbox.environment-prewarm.v1\0");
    hash.update(bytes);
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn invalid_encoding() -> ExecutionManagerError {
    ExecutionManagerError::InvalidRequest("cannot encode prewarm compatibility".into())
}

fn metadata(record: &BoxRecord) -> ExecutionManagerResult<&ManagedExecutionMetadata> {
    record.managed_execution.as_ref().ok_or_else(|| {
        ExecutionManagerError::Internal("prewarm record lost its canonical metadata".into())
    })
}

impl LocalExecutionManager {
    /// The worker owns the canonical lifecycle lock until startup is recorded.
    pub(crate) async fn start_prewarmed(
        &self,
        execution_id: &ExecutionId,
        generation: ExecutionGeneration,
    ) -> ExecutionManagerResult<ExecutionLease> {
        let _lifecycle =
            super::lifecycle_lock::acquire(&self.home_dir, execution_id.as_str()).await?;
        let record = self
            .get(execution_id)
            .await?
            .ok_or_else(|| ExecutionManagerError::NotFound(execution_id.clone()))?;
        super::support::require_generation(&record, execution_id, generation)?;
        if !record.is_unclaimed_prewarm() {
            return Err(ExecutionManagerError::Conflict {
                execution_id: execution_id.clone(),
                message: "execution is not an unclaimed prewarm reservation".into(),
            });
        }
        self.ensure_started(record).await
    }

    /// Reserve at most one clean slot for this owner and exact boot policy. Start uses the normal API.
    pub async fn ensure_prewarmed_reservation(
        &self,
        request: CreateExecutionRequest,
        operation: &OperationId,
        scope: &str,
        owner: &str,
    ) -> ExecutionManagerResult<ExecutionReservation> {
        validate_bootstrap(&request)?;
        if !valid_identity(scope) || !valid_identity(owner) {
            return Err(ExecutionManagerError::InvalidRequest(
                "invalid prewarm scope or owner".into(),
            ));
        }
        let key = compatibility(&request)?;
        let mut incoming = self.prepare_creation_record(request, operation).await?;
        incoming
            .managed_execution
            .as_mut()
            .ok_or_else(invalid_encoding)?
            .prewarm = Some(ManagedPrewarm {
            scope: scope.into(),
            owner: owner.into(),
            compatibility_digest: key,
            operation_id: operation.clone(),
            generation: ExecutionGeneration::INITIAL,
        });
        let path = self.store.path().to_path_buf();
        let selected = run_store(move || {
            BoxStateStore::transact(&path, |store| {
                let candidate = incoming.managed_execution.as_ref().ok_or_else(|| {
                    ManagedExecutionStoreError::InvalidRecord("missing incoming metadata".into())
                })?;
                let warm = candidate.prewarm.as_ref().ok_or_else(|| {
                    ManagedExecutionStoreError::InvalidRecord("missing prewarm provenance".into())
                })?;
                if let Some(existing) = store.find_by_operation_id(&candidate.operation_id) {
                    let existing_meta = existing.managed_execution.as_ref().ok_or_else(|| {
                        ManagedExecutionStoreError::InvalidRecord(
                            "missing existing metadata".into(),
                        )
                    })?;
                    if existing_meta.prewarm.as_ref() != Some(warm)
                        || existing_meta.operation_id != candidate.operation_id
                        || existing_meta.creation_intent_digest != candidate.creation_intent_digest
                    {
                        return Err(ManagedExecutionStoreError::Conflict {
                            execution_id: ExecutionId::new(existing.id.clone()).map_err(|e| {
                                ManagedExecutionStoreError::InvalidRecord(e.to_string())
                            })?,
                            message: "prewarm operation was already used".into(),
                        });
                    }
                    return Ok(existing.clone());
                }
                for record in store.records() {
                    if let Some(existing) = record
                        .managed_execution
                        .as_ref()
                        .and_then(|m| m.prewarm.as_ref().filter(|p| p.is_unclaimed(m)))
                    {
                        if existing.scope == warm.scope
                            && existing.owner == warm.owner
                            && existing.compatibility_digest == warm.compatibility_digest
                            && matches!(
                                record.managed_state().map_err(|e| {
                                    ManagedExecutionStoreError::InvalidRecord(e.to_string())
                                })?,
                                Some(
                                    ManagedExecutionState::Created
                                        | ManagedExecutionState::Starting
                                        | ManagedExecutionState::Running
                                )
                            )
                        {
                            return Ok(record.clone());
                        }
                    }
                }
                store.records_mut().push(incoming.clone());
                Ok(incoming)
            })
        })
        .await?;
        reservation_from_record(&selected)
    }

    /// Claim an authenticated running VM once, advancing the canonical generation without rebooting.
    pub async fn claim_prewarmed(
        &self,
        request: CreateExecutionRequest,
        operation: &OperationId,
        scope: &str,
    ) -> ExecutionManagerResult<Option<ExecutionReservation>> {
        validate_bootstrap(&request)?;
        if !valid_identity(scope) {
            return Err(ExecutionManagerError::InvalidRequest(
                "invalid prewarm scope".into(),
            ));
        }
        let key = compatibility(&request)?;
        let incoming = self.prepare_creation_record(request, operation).await?;
        if let Some(existing) = self.get_by_operation(operation).await? {
            require_same_claim(&existing, &incoming, scope, &key)?;
            return reservation_from_record(&existing).map(Some);
        }
        let mut candidates = self.managed_records().await?;
        candidates.sort_by(|a, b| a.id.cmp(&b.id));
        for observed in candidates {
            let Some(warm) = metadata(&observed)?.prewarm.as_ref() else {
                continue;
            };
            if warm.scope != scope
                || warm.compatibility_digest != key
                || !warm.is_unclaimed(metadata(&observed)?)
                || managed_state(&observed)? != ManagedExecutionState::Running
            {
                continue;
            }
            let id = ExecutionId::new(observed.id.clone())?;
            let _lifecycle = super::lifecycle_lock::acquire(&self.home_dir, id.as_str()).await?;
            let Some(bound) = self.get(&id).await? else {
                continue;
            };
            let current = metadata(&bound)?;
            if current.prewarm.as_ref() != Some(warm)
                || !warm.is_unclaimed(current)
                || current.generation != warm.generation
                || managed_state(&bound)? != ManagedExecutionState::Running
                || current.runtime_route != metadata(&incoming)?.runtime_route
            {
                continue;
            }
            let observation = match self.backend.inspect(&bound).await {
                Err(ExecutionManagerError::NotFound(_)) => continue,
                result => result?,
            };
            observation.validate(&id)?;
            let Some(handle) = observation.handle else {
                continue;
            };
            if observation.state != ExecutionState::Running
                || handle.pid != bound.pid
                || handle.pid_start_time != bound.pid_start_time
                || handle.exec_socket_path != bound.exec_socket_path
                || handle.oci_runtime != current.oci_runtime
            {
                continue;
            }
            let mut replacement = build_managed_record(
                &self.home_dir,
                &id,
                operation.clone(),
                metadata(&incoming)?.request.clone(),
                bound.created_at,
            )?;
            let next = ExecutionGeneration::new(
                current
                    .generation
                    .get()
                    .checked_add(1)
                    .ok_or_else(invalid_encoding)?,
            )?;
            let replaced = replacement
                .managed_execution
                .as_mut()
                .ok_or_else(invalid_encoding)?;
            replaced.generation = next;
            replaced.runtime_route = current.runtime_route;
            replaced.prewarm = Some(warm.clone());
            replacement.status = ManagedExecutionState::Running.as_status().into();
            super::record::apply_handle(&mut replacement, &handle);
            replacement.started_at = bound.started_at;
            replacement.anonymous_volumes = bound.anonymous_volumes.clone();
            metadata(&replacement)?
                .validate()
                .map_err(|e| ExecutionManagerError::Internal(e.to_string()))?;
            let path = self.store.path().to_path_buf();
            let scope = scope.to_string();
            let key = key.clone();
            let incoming = incoming.clone();
            let committed = run_store(move || {
                BoxStateStore::transact(&path, |store| {
                    if let Some(existing) =
                        store.find_by_operation_id(&metadata_store(&replacement)?.operation_id)
                    {
                        if metadata_store(existing)?.creation_intent_digest
                            != metadata_store(&incoming)?.creation_intent_digest
                            || metadata_store(existing)?.operation_id
                                != metadata_store(&incoming)?.operation_id
                            || !metadata_store(existing)?
                                .prewarm
                                .as_ref()
                                .is_some_and(|warm| {
                                    warm.scope == scope && warm.compatibility_digest == key
                                })
                        {
                            return Err(ManagedExecutionStoreError::Conflict {
                                execution_id: id.clone(),
                                message: "claim operation already has different intent".into(),
                            });
                        }
                        return Ok(Some(existing.clone()));
                    }
                    let Some(current) = store.find_by_id_mut(id.as_str()) else {
                        return Ok(None);
                    };
                    let before = serde_json::to_value(&bound)
                        .map_err(|e| ManagedExecutionStoreError::InvalidRecord(e.to_string()))?;
                    let now = serde_json::to_value(&current)
                        .map_err(|e| ManagedExecutionStoreError::InvalidRecord(e.to_string()))?;
                    if before != now {
                        return Ok(None);
                    }
                    *current = replacement.clone();
                    Ok(Some(replacement))
                })
            })
            .await?;
            if let Some(record) = committed {
                return reservation_from_record(&record).map(Some);
            }
        }
        Ok(None)
    }
}

fn metadata_store(
    record: &BoxRecord,
) -> Result<&ManagedExecutionMetadata, ManagedExecutionStoreError> {
    record
        .managed_execution
        .as_ref()
        .ok_or_else(|| ManagedExecutionStoreError::InvalidRecord("missing prewarm metadata".into()))
}

fn require_same_claim(
    existing: &BoxRecord,
    incoming: &BoxRecord,
    scope: &str,
    key: &str,
) -> ExecutionManagerResult<()> {
    if metadata(existing)?.operation_id != metadata(incoming)?.operation_id
        || metadata(existing)?.creation_intent_digest != metadata(incoming)?.creation_intent_digest
        || !metadata(existing)?
            .prewarm
            .as_ref()
            .is_some_and(|warm| warm.scope == scope && warm.compatibility_digest == key)
    {
        return Err(ExecutionManagerError::Conflict {
            execution_id: ExecutionId::new(existing.id.clone())?,
            message: "claim operation already has different intent".into(),
        });
    }
    existing.require_claimed()
}
