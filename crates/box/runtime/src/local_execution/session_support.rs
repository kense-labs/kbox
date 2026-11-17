//! Shared policy for legacy socket and OCI SDK process sessions.

use std::collections::{BTreeMap, HashMap};

use kbox_core::{ExecutionGeneration, ExecutionId};
#[cfg(unix)]
use kbox_core::{ExecutionManagerError, ExecutionManagerResult, SecurityConfig};

use crate::BoxRecord;

pub(super) fn has_oci_runtime(record: &BoxRecord) -> bool {
    record
        .managed_execution
        .as_ref()
        .and_then(|metadata| metadata.oci_runtime.as_ref())
        .is_some()
}

/// Sessions owned by a LocalExecutionBackend (OCI SDK or Spec050 Sandbox).
///
/// Sandbox records have no guest exec socket; routing them through the MicroVM
/// ExecClient path times out as "guest may be wedged". Never silent-downgrade.
pub(super) fn uses_backend_session(record: &BoxRecord) -> bool {
    if has_oci_runtime(record) {
        return true;
    }
    record
        .managed_execution
        .as_ref()
        .is_some_and(|metadata| metadata.runtime_route == crate::ManagedRuntimeRoute::Sandbox)
}

/// Merge the environment fixed at Sandbox creation into an exec/PTY request.
///
/// The guest normally inherits these values from guest-init, but the execution
/// contract must not depend on which user a later request selects. Per-request
/// values remain authoritative, matching OCI/Docker exec environment semantics.
pub(super) fn inherit_container_environment(
    container: &HashMap<String, String>,
    request: &mut Vec<String>,
) {
    let mut merged: BTreeMap<String, String> = container
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let mut malformed = Vec::new();
    for entry in std::mem::take(request) {
        if let Some((key, value)) = entry.split_once('=') {
            merged.insert(key.to_string(), value.to_string());
        } else {
            malformed.push(entry);
        }
    }
    request.extend(
        merged
            .into_iter()
            .map(|(key, value)| format!("{key}={value}")),
    );
    request.extend(malformed);
}

/// Fused-engine Sandbox expands Secret env at container create. Durable
/// `record.env` still carries the non-sensitive manifest for restart/audit;
/// re-injecting it into exec/PTY would expose the control key without values.
/// OCI backend sessions keep the manifest for agent-side materialization.
pub(super) fn strip_sandbox_secret_manifest(record: &BoxRecord, request_env: &mut Vec<String>) {
    let is_sandbox = record
        .managed_execution
        .as_ref()
        .is_some_and(|metadata| metadata.runtime_route == crate::ManagedRuntimeRoute::Sandbox);
    if !is_sandbox {
        return;
    }
    let prefix = format!("{}=", kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST);
    request_env.retain(|entry| !entry.starts_with(&prefix));
}

/// Apply the creation-time security policy to a legacy guest exec/PTY request.
///
/// A MicroVM's guest-init control plane is intentionally more privileged than
/// its workloads. Every later process must therefore receive the same private
/// `K_SEC_*` controls as the main process. Request and container environment
/// entries cannot override these runtime-owned values, and the guest filters
/// them before the workload observes its environment.
#[cfg(unix)]
pub(super) fn inherit_execution_security_environment(
    record: &BoxRecord,
    request: &mut Vec<String>,
) -> ExecutionManagerResult<()> {
    let security = if let Some(metadata) = record.managed_execution.as_ref() {
        let config = &metadata.request.config;
        SecurityConfig::from_options(
            &config.security_opt,
            &config.cap_add,
            &config.cap_drop,
            config.privileged,
        )
    } else {
        SecurityConfig::from_options(
            &record.security_opt,
            &record.cap_add,
            &record.cap_drop,
            record.privileged,
        )
    };
    security
        .validate()
        .map_err(ExecutionManagerError::InvalidRequest)?;

    let mut merged = BTreeMap::new();
    let mut malformed = Vec::new();
    for entry in std::mem::take(request) {
        if let Some((key, value)) = entry.split_once('=') {
            if !key.starts_with("K_SEC_") {
                merged.insert(key.to_string(), value.to_string());
            }
        } else {
            malformed.push(entry);
        }
    }
    for (key, value) in security.to_env_vars() {
        merged.insert(key, value);
    }
    request.extend(
        merged
            .into_iter()
            .map(|(key, value)| format!("{key}={value}")),
    );
    request.extend(malformed);
    Ok(())
}

/// Record only environment key names at the final host-to-runtime boundary.
pub(super) fn debug_session_environment(
    execution_id: &ExecutionId,
    generation: ExecutionGeneration,
    operation: &str,
    container: &HashMap<String, String>,
    request: &[String],
) {
    let mut container_keys: Vec<&str> = container.keys().map(String::as_str).collect();
    container_keys.sort_unstable();
    let mut request_keys: Vec<&str> = request
        .iter()
        .filter_map(|entry| entry.split_once('=').map(|(key, _)| key))
        .collect();
    request_keys.sort_unstable();
    request_keys.dedup();
    let malformed_request_entries = request.iter().filter(|entry| !entry.contains('=')).count();

    tracing::debug!(
        %execution_id,
        generation = generation.get(),
        operation,
        container_env_count = container.len(),
        container_env_keys = ?container_keys,
        merged_request_env_count = request.len(),
        merged_request_env_keys = ?request_keys,
        malformed_request_entries,
        "Prepared managed execution session environment"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        inherit_container_environment, strip_sandbox_secret_manifest, uses_backend_session,
    };
    use crate::local_execution::record::build_managed_record;
    use crate::ManagedRuntimeRoute;
    use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionId, OperationId};
    use std::collections::HashMap;

    #[test]
    fn sandbox_route_uses_backend_session_not_guest_exec() {
        let home = tempfile::tempdir().unwrap();
        let mut record = build_managed_record(
            home.path(),
            &ExecutionId::new("00000000-0000-4000-8000-000000000001").unwrap(),
            OperationId::new("session-support-op").unwrap(),
            CreateExecutionRequest {
                external_sandbox_id: "ext".into(),
                config: BoxConfig {
                    image: "alpine:3.20".into(),
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
        assert!(uses_backend_session(&record));
        record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::BoxVm;
        assert!(!uses_backend_session(&record));
    }

    #[test]
    fn request_environment_overrides_inherited_container_values() {
        let container = HashMap::from([
            ("ALPHA".to_string(), "container".to_string()),
            ("BETA".to_string(), "container".to_string()),
        ]);
        let mut request = vec!["BETA=request".to_string(), "GAMMA=request".to_string()];

        inherit_container_environment(&container, &mut request);

        assert_eq!(
            request,
            ["ALPHA=container", "BETA=request", "GAMMA=request"]
        );
    }

    #[test]
    fn malformed_request_entries_are_preserved_after_inherited_values() {
        let container = HashMap::from([("ALPHA".to_string(), "container".to_string())]);
        let mut request = vec!["MALFORMED".to_string()];

        inherit_container_environment(&container, &mut request);

        assert_eq!(request, ["ALPHA=container", "MALFORMED"]);
    }

    #[test]
    fn sandbox_exec_strips_secret_manifest_control_key() {
        let home = tempfile::tempdir().unwrap();
        let mut record = build_managed_record(
            home.path(),
            &ExecutionId::new("00000000-0000-4000-8000-000000000002").unwrap(),
            OperationId::new("strip-manifest-op").unwrap(),
            CreateExecutionRequest {
                external_sandbox_id: "ext".into(),
                config: BoxConfig {
                    image: "alpine:3.20".into(),
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
        let manifest = format!("{}={{}}", kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST);
        let mut env = vec![
            "API_TOKEN=from-container".into(),
            format!("{manifest}"),
            "OTHER=1".into(),
        ];
        strip_sandbox_secret_manifest(&record, &mut env);
        assert_eq!(env, ["API_TOKEN=from-container", "OTHER=1"]);

        record.managed_execution.as_mut().unwrap().runtime_route = ManagedRuntimeRoute::OciSdk;
        let mut oci_env = vec![format!("{manifest}"), "OTHER=1".into()];
        strip_sandbox_secret_manifest(&record, &mut oci_env);
        assert_eq!(oci_env, [manifest, "OTHER=1".into()]);
    }
}
