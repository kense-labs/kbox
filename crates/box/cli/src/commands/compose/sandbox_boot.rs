//! Spec050: compose services with `ExecutionBoundary::Sandbox` boot through
//! `LocalExecutionManager` — never silent MicroVM via `VmManager`.

use std::collections::BTreeMap;

use kbox_core::config::BoxConfig;
use kbox_core::network::NetworkMode;
use kbox_core::{
    CreateExecutionRequest, ExecutionManager, ExecutionRecordPolicy, ExecutionRestartPolicy,
    OperationId,
};
use kbox_runtime::LocalExecutionManager;

use crate::state::{BoxRecord, HealthCheck, StateFile};

/// Admit sandbox compose networking.
///
/// Bridge is passed through: the Sandbox backend creates a fused-engine Docker
/// bridge by that name (not MicroVM NetworkStore). Never silent-remap to TSI
/// and never fall back to MicroVM. TSI / Host / None pass through unchanged.
pub(super) fn sandbox_compose_network(mode: NetworkMode) -> Result<NetworkMode, String> {
    match mode {
        NetworkMode::Bridge { .. } | NetworkMode::Tsi | NetworkMode::Host | NetworkMode::None => {
            Ok(mode)
        }
    }
}

pub(super) fn execution_restart_policy(value: &str) -> Result<ExecutionRestartPolicy, String> {
    match value {
        "no" => Ok(ExecutionRestartPolicy::No),
        "always" => Ok(ExecutionRestartPolicy::Always),
        "on-failure" => Ok(ExecutionRestartPolicy::OnFailure),
        "unless-stopped" => Ok(ExecutionRestartPolicy::UnlessStopped),
        other => Err(format!("Invalid normalized restart policy: {other}")),
    }
}

pub(super) struct SandboxComposeBoot<'a> {
    pub home: &'a std::path::Path,
    pub box_config: BoxConfig,
    pub box_name: String,
    pub labels: BTreeMap<String, String>,
    pub volume_names: Vec<String>,
    pub restart_policy: String,
    pub max_restart_count: u32,
    pub health_check: Option<HealthCheck>,
    pub healthcheck_disabled: bool,
    pub stop_signal: Option<String>,
}

/// Create+start one compose service on the Sandbox LocalExecutionManager path.
pub(super) async fn boot_sandbox_compose_service(
    args: SandboxComposeBoot<'_>,
) -> Result<BoxRecord, Box<dyn std::error::Error>> {
    let mut config = args.box_config;
    config.network = sandbox_compose_network(config.network)?;
    kbox_core::resolve_execution(&config)?;

    let manager =
        LocalExecutionManager::with_configured_backend(args.home.join("boxes.json"), args.home)
            .await?;
    manager.preflight_launch().await?;

    let restart = execution_restart_policy(&args.restart_policy)?;
    let operation_id = OperationId::new(format!("compose-sandbox-{}", uuid::Uuid::new_v4()))?;
    let request = CreateExecutionRequest {
        external_sandbox_id: operation_id.as_str().to_string(),
        config,
        labels: args.labels,
        policy: ExecutionRecordPolicy {
            name: Some(args.box_name),
            auto_remove: false,
            restart_policy: restart,
            max_restart_count: args.max_restart_count,
            health_check: args.health_check,
            healthcheck_disabled: args.healthcheck_disabled,
            volume_names: args.volume_names,
            stop_signal: args.stop_signal,
            ..Default::default()
        },
        rootfs_snapshot_id: None,
    };

    let reservation = manager.create(request, &operation_id).await?;
    let execution_id = reservation.execution_id.clone();
    let box_id = execution_id.to_string();
    if let Err(error) = manager.start(&execution_id, reservation.generation).await {
        let _ = manager
            .remove_execution(&execution_id, reservation.generation)
            .await;
        return Err(format!("Failed to start sandbox compose service: {error}").into());
    }

    let record = StateFile::load_readonly()?
        .find_by_id(&box_id)
        .cloned()
        .ok_or_else(|| format!("sandbox compose service {box_id} disappeared after startup"))?;

    let route = record
        .managed_execution
        .as_ref()
        .map(|meta| meta.runtime_route);
    if route != Some(kbox_runtime::ManagedRuntimeRoute::Sandbox) {
        let _ = manager
            .remove_execution(&execution_id, reservation.generation)
            .await;
        return Err(format!(
            "sandbox compose refused: expected ManagedRuntimeRoute::Sandbox, got {route:?}"
        )
        .into());
    }

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_compose_network_admits_bridge() {
        let mode = NetworkMode::Bridge {
            network: "proj_default".into(),
        };
        assert_eq!(
            sandbox_compose_network(mode.clone()).unwrap(),
            mode,
            "bridge must pass through for fused-engine docker networks"
        );
    }

    #[test]
    fn sandbox_compose_network_keeps_tsi() {
        assert_eq!(
            sandbox_compose_network(NetworkMode::Tsi).unwrap(),
            NetworkMode::Tsi
        );
    }

    #[test]
    fn sandbox_compose_network_keeps_none_and_host() {
        assert_eq!(
            sandbox_compose_network(NetworkMode::None).unwrap(),
            NetworkMode::None
        );
        assert_eq!(
            sandbox_compose_network(NetworkMode::Host).unwrap(),
            NetworkMode::Host
        );
    }
}
