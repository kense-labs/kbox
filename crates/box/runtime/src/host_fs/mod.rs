//! Host filesystem helpers for MicroVM (id-mapping + leftover mount cleanup).
//!
//! Attachment aliases may still live under the legacy layout dir named
//! `sandbox/`; destroy/reap tears them down.

mod id_mapping;
mod mount_alias;
pub mod rootfs;

pub use id_mapping::{
    map_container_gid, map_container_uid, unmap_host_gid, unmap_host_uid, IdMapping, IdMappingPlan,
};
pub(crate) use mount_alias::cleanup_mount_aliases;
#[cfg(target_os = "linux")]
pub(crate) use mount_alias::mount_alias_root;
pub use rootfs::{
    inspect_rootfs_identity_requirements, mapped_root_ids, prepare_managed_mount_source,
    prepare_managed_secret_mount_source, prepare_rootfs_ownership, validate_external_mount_access,
    RootfsIdentityRequirements,
};

/// Minimal resource compiler retained for OCI live-update validation on MicroVM routes.
pub mod oci {
    use kbox_core::config::BoxConfig;
    use kbox_core::error::{BoxError, Result};
    use kbox_core::ExecutionResourceUpdate;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OciResources {
        pub memory_limit: i64,
        pub memory_reservation: Option<i64>,
        pub memory_swap: Option<i64>,
        pub cpu_shares: Option<u64>,
        pub cpu_quota: i64,
        pub cpu_period: u64,
        pub cpuset_cpus: Option<String>,
        pub pids_limit: i64,
    }

    impl OciResources {
        pub fn from_box_config(config: &BoxConfig) -> Result<Self> {
            if config.resources.memory_mb == 0 {
                return Err(BoxError::ConfigError(
                    "memory limit must be greater than zero".into(),
                ));
            }
            if config.resources.vcpus == 0 {
                return Err(BoxError::ConfigError(
                    "CPU limit must be greater than zero".into(),
                ));
            }
            let limits = &config.resource_limits;
            let update = ExecutionResourceUpdate {
                memory_reservation: limits.memory_reservation,
                memory_swap: limits.memory_swap,
                pids_limit: limits.pids_limit,
                cpu_shares: limits.cpu_shares,
                cpu_quota: limits.cpu_quota,
                cpu_period: limits.cpu_period,
                cpuset_cpus: limits.cpuset_cpus.clone(),
            };
            if !update.is_empty() {
                update
                    .validate()
                    .map_err(|error| BoxError::ConfigError(error.to_string()))?;
            }
            let memory_limit = i64::try_from(
                limits
                    .sandbox_memory_limit_bytes
                    .unwrap_or(u64::from(config.resources.memory_mb) * 1024 * 1024),
            )
            .map_err(|_| BoxError::ConfigError("memory limit overflow".into()))?;
            if memory_limit == 0 {
                return Err(BoxError::ConfigError(
                    "memory limit must be greater than zero".into(),
                ));
            }
            let cpu_period = limits.cpu_period.unwrap_or(100_000);
            let cpu_quota = match limits.cpu_quota {
                Some(quota) => quota,
                None => i64::try_from(cpu_period)
                    .ok()
                    .and_then(|period| i64::from(config.resources.vcpus).checked_mul(period))
                    .ok_or_else(|| BoxError::ConfigError("CPU quota overflow".into()))?,
            };
            let memory_reservation = limits
                .memory_reservation
                .map(i64::try_from)
                .transpose()
                .map_err(|_| BoxError::ConfigError("memory reservation overflow".into()))?;
            if memory_reservation.is_some_and(|value| value > memory_limit) {
                return Err(BoxError::ConfigError(
                    "memory reservation exceeds the hard limit".into(),
                ));
            }
            if limits
                .memory_swap
                .is_some_and(|value| value > 0 && value < memory_limit)
            {
                return Err(BoxError::ConfigError(
                    "memory plus swap limit is smaller than the hard limit".into(),
                ));
            }
            let pids_limit = i64::try_from(limits.pids_limit.unwrap_or(4096))
                .map_err(|_| BoxError::ConfigError("PID limit overflow".into()))?;
            Ok(Self {
                memory_limit,
                memory_reservation,
                memory_swap: limits.memory_swap,
                cpu_shares: limits.cpu_shares,
                cpu_quota,
                cpu_period,
                cpuset_cpus: limits.cpuset_cpus.clone(),
                pids_limit,
            })
        }
    }
}

#[cfg(test)]
#[path = "resource_tests.rs"]
mod resource_tests;
