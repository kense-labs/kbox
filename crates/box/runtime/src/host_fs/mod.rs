//! Host filesystem helpers for MicroVM (id-mapping + leftover mount cleanup).
//!
//! Attachment aliases may still live under the legacy layout dir named
//! `sandbox/`; destroy/reap tears them down.

mod id_mapping;
mod mount_alias;
pub mod rootfs;

pub use id_mapping::{
    map_container_gid, map_container_uid, unmap_host_gid, unmap_host_uid, IdMapping,
    IdMappingPlan,
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
            let memory_limit = (config.resources.memory_mb as i64)
                .checked_mul(1024 * 1024)
                .ok_or_else(|| BoxError::ConfigError("memory limit overflow".into()))?;
            let cpu_period = 100_000u64;
            let cpu_quota = (config.resources.vcpus as i64)
                .checked_mul(cpu_period as i64)
                .ok_or_else(|| BoxError::ConfigError("CPU quota overflow".into()))?;
            Ok(Self {
                memory_limit,
                memory_reservation: None,
                memory_swap: None,
                cpu_shares: None,
                cpu_quota,
                cpu_period,
                cpuset_cpus: None,
                pids_limit: 4096,
            })
        }
    }
}
