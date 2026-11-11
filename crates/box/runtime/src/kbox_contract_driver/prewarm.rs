//! Optional clean environment preparation over the canonical execution manager.
use std::sync::Arc;

use async_trait::async_trait;
use kbox_contract::contract::{IsolationLevel, RestartPolicy, RuntimeUnitClass, RuntimeUnitSpec};
use kbox_contract::{RuntimeError, RuntimeResult};
use kbox_core::{ExecutionLease, ExecutionManager, ExecutionReservation, OperationId};

use crate::{BoxRecord, ManagedExecutionState};

use super::mapping::{attach_host_services, creation_request_for, validate_creation_spec};
use super::metadata::map_execution_error;
use super::{BoxRuntimeDriver, NATIVE_GUEST_PROFILE_DIGEST};

#[async_trait]
pub(crate) trait BoxPrewarmScopePort: Send + Sync {
    async fn scope(&self, spec: &RuntimeUnitSpec) -> RuntimeResult<Option<String>>;
    fn owns_scope(&self, scope: &str) -> bool;
}

pub(super) enum PrewarmScope {
    Fixed(String),
    Port(Arc<dyn BoxPrewarmScopePort>),
}

pub(crate) fn eligible(spec: &RuntimeUnitSpec) -> bool {
    spec.class == RuntimeUnitClass::Service
        && spec.isolation == IsolationLevel::Sandbox
        && spec.semantics_profile_digest.as_deref() == Some(NATIVE_GUEST_PROFILE_DIGEST)
        && spec.process.command == ["/bin/sleep"]
        && spec.process.args == ["infinity"]
        && spec.process.environment.is_empty()
        && spec.secrets.is_empty()
        && spec.health.is_none()
        && spec.restart == RestartPolicy::Never
}

impl BoxRuntimeDriver {
    /// The caller partitions trust and provider-build identity. Ordinary drivers remain unchanged.
    pub fn with_environment_prewarm(mut self, scope: impl Into<String>) -> RuntimeResult<Self> {
        let scope = scope.into();
        if scope.is_empty()
            || scope.len() > 128
            || !scope
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return Err(RuntimeError::InvalidRequest(
                "invalid environment prewarm scope".into(),
            ));
        }
        self.prewarm_scope = Some(PrewarmScope::Fixed(scope));
        Ok(self)
    }

    pub(crate) fn with_prewarm_scope_port(mut self, port: Arc<dyn BoxPrewarmScopePort>) -> Self {
        self.prewarm_scope = Some(PrewarmScope::Port(port));
        self
    }

    pub(crate) async fn environment_scope(
        &self,
        spec: &RuntimeUnitSpec,
    ) -> RuntimeResult<Option<String>> {
        if !eligible(spec) {
            return Ok(None);
        }
        match &self.prewarm_scope {
            Some(PrewarmScope::Fixed(scope)) => Ok(Some(scope.clone())),
            Some(PrewarmScope::Port(port)) => port.scope(spec).await,
            None => Ok(None),
        }
    }

    /// Boot a clean compatible environment; no package, user process or credential enters it.
    pub async fn prepare_environment(
        &self,
        spec: &RuntimeUnitSpec,
        owner: &str,
    ) -> RuntimeResult<ExecutionLease> {
        let scope = self.environment_scope(spec).await?.ok_or_else(|| {
            RuntimeError::UnsupportedCapabilities(vec!["environment-prewarm.v1".into()])
        })?;
        let reserved = self.reserve_environment(spec, owner, &scope).await?;
        self.manager
            .start_prewarmed(&reserved.execution_id, reserved.generation)
            .await
            .map_err(|error| map_execution_error(&spec.unit_id, error))
    }

    pub(crate) async fn reserve_environment(
        &self,
        spec: &RuntimeUnitSpec,
        owner: &str,
        scope: &str,
    ) -> RuntimeResult<ExecutionReservation> {
        validate_creation_spec(
            spec,
            self.execution_boundary,
            self.sev_snp_config(),
            &self.config.secret_root,
        )?;
        let host_services = self.host_services.resolve(spec).await?;
        let storage = self.artifact_storage.prepare_plan(spec).await?;
        let mut request = creation_request_for(
            spec,
            self.execution_boundary,
            self.sev_snp_config(),
            &self.config.secret_root,
            &storage,
            None,
        )?;
        attach_host_services(spec, &mut request, &host_services)?;
        request.external_sandbox_id = format!("prewarm:{owner}");
        request.labels.clear();
        let operation = OperationId::new(format!("kbox-prewarm-v1:{}", uuid::Uuid::new_v4()))
            .map_err(|error| map_execution_error(&spec.unit_id, error))?;
        self.manager
            .ensure_prewarmed_reservation(request, &operation, scope, owner)
            .await
            .map_err(|error| map_execution_error(&spec.unit_id, error))
    }

    /// Retire only this caller's unclaimed slots; claimed workloads retain their normal owner.
    pub async fn retire_unclaimed_environments(&self, owner: &str) -> RuntimeResult<()> {
        self.retire_unclaimed_environments_except(owner, None).await
    }

    pub(crate) async fn unclaimed_environments(
        &self,
        owner: &str,
    ) -> RuntimeResult<Vec<BoxRecord>> {
        let Some(resolver) = &self.prewarm_scope else {
            return Ok(Vec::new());
        };
        let records = self
            .manager
            .managed_records()
            .await
            .map_err(|error| map_execution_error(owner, error))?;
        Ok(records
            .into_iter()
            .filter(|record| {
                let Some(metadata) = record.managed_execution.as_ref() else {
                    return false;
                };
                let Some(warm) = metadata.prewarm.as_ref() else {
                    return false;
                };
                let owned = match resolver {
                    PrewarmScope::Fixed(scope) => warm.scope() == scope,
                    PrewarmScope::Port(port) => port.owns_scope(warm.scope()),
                };
                owned && warm.owner() == owner && warm.is_unclaimed(metadata)
            })
            .collect())
    }

    pub(crate) async fn retire_unclaimed_environments_except(
        &self,
        owner: &str,
        keep_scope: Option<&str>,
    ) -> RuntimeResult<()> {
        for record in self.unclaimed_environments(owner).await? {
            let Some(metadata) = record.managed_execution.as_ref() else {
                continue;
            };
            let Some(warm) = metadata.prewarm.as_ref() else {
                continue;
            };
            let state = record
                .managed_state()
                .map_err(|error| RuntimeError::Protocol(error.to_string()))?;
            if keep_scope == Some(warm.scope())
                && matches!(
                    state,
                    Some(
                        ManagedExecutionState::Created
                            | ManagedExecutionState::Starting
                            | ManagedExecutionState::Running
                    )
                )
            {
                continue;
            }
            let id = kbox_core::ExecutionId::new(record.id)
                .map_err(|error| map_execution_error(owner, error))?;
            if state != Some(ManagedExecutionState::Removing) {
                if let Err(error) = self.manager.kill(&id, metadata.generation).await {
                    // A concurrent claim advances generation and transfers teardown responsibility.
                    let current = self
                        .manager
                        .managed_record(&id)
                        .await
                        .map_err(|error| map_execution_error(owner, error))?;
                    if current
                        .as_ref()
                        .and_then(|record| record.managed_execution.as_ref())
                        .is_none_or(|current| current.generation != metadata.generation)
                    {
                        continue;
                    }
                    return Err(map_execution_error(owner, error));
                }
            }
            self.manager
                .remove(&id, metadata.generation)
                .await
                .map_err(|error| map_execution_error(owner, error))?;
        }
        Ok(())
    }
}
