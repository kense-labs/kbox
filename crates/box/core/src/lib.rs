//! Kbox Box Core - Foundational Types and Abstractions
//!
//! This module provides the foundational types, traits, and abstractions
//! used across the Kbox Box MicroVM runtime.

pub mod audit;
pub mod compose;
pub mod config;
pub mod dns;
pub mod env;
pub mod error;
pub mod event;
pub mod exec;
pub mod execution;
pub mod fs_atomic;
pub mod guest_exec;
pub mod host_service;
pub mod lifecycle_profile;
pub mod log;
pub mod network;
pub mod operator;
pub mod platform;
pub mod port;
pub mod pty;
pub mod rootfs_baseline;
pub mod rootfs_metadata;
pub mod scale;
pub mod secret;
pub mod security;
pub mod snapshot;
pub mod tee;
pub mod traits;
pub mod vmm;
pub mod volume;
#[cfg(windows)]
pub mod windows_file;
#[cfg(windows)]
pub mod windows_symlink;

// Re-export commonly used types
pub use audit::{AuditAction, AuditConfig, AuditEvent, AuditOutcome};
pub use compose::ComposeConfig;
pub use config::{BoxConfig, ExecutionIsolation, ResourceConfig, ResourceLimits};
pub use error::{BoxError, Result};
pub use event::{BoxEvent, EventEmitter};
pub use exec::{ExecChunk, ExecEvent, ExecExit, ExecMetrics, StreamType};
pub use exec::{ExecOutput, ExecRequest};
pub use exec::{
    FileOp, FileRequest, FileResponse, FilesystemEntry, FilesystemEntryKind, FilesystemOp,
    FilesystemRequest, FilesystemResponse, GuestSessionRequest, MAX_BOUNDED_FILE_BYTES,
};
pub use exec::{EXEC_VSOCK_PORT, PORT_FWD_VSOCK_PORT};
pub use execution::{
    resolve_execution, validate_microvm_compatibility, ExecutionBackend, IsolationClass,
    ResolvedExecutionPlan,
};
pub use host_service::{validate_host_service_sockets, HostServiceSocket};
pub use network::{
    ipv4_in_prefix, load_network_egress_allowed_names, load_network_egress_rules, lookup_network_a,
    parse_ipv4_cidr, EgressMatchRule, IsolationMode, NetworkConfig, NetworkEndpoint, NetworkMode,
    NetworkPolicy, PolicyAction,
};
pub use operator::{BoxAutoscaler, BoxAutoscalerSpec, BoxAutoscalerStatus, MetricType};
pub use platform::{
    BridgeNetworkBackend, HostFamily, HostGuestChannel, Platform, PlatformCapabilities, VmBackend,
};
pub use port::{
    normalize_and_resolve_port_maps, normalize_port_maps, parse_port_mapping,
    resolve_auto_host_port, PortMapping, PortProtocol,
};
pub use pty::PTY_VSOCK_PORT;
pub use scale::{
    InstanceDeregistration, InstanceEvent, InstanceHealth, InstanceInfo, InstanceRegistration,
    InstanceState, ScaleConfig, ScaleDirection, ScaleObservation, ScaleOperationConflict,
    ScaleOperationRequest, ScaleOperationResponse, ScaleRequest, ScaleResponse,
    SCALE_OPERATION_SCHEMA_VERSION,
};
pub use security::{SeccompMode, SecurityConfig};
pub use snapshot::{
    SnapshotConfig, SnapshotImageConfig, SnapshotImageHealthCheck, SnapshotMetadata,
    MANAGED_SANDBOX_OCI_CAPTURE_LABEL, MANAGED_SANDBOX_OCI_CAPTURE_VALUE,
};
pub use tee::ATTEST_VSOCK_PORT;
pub use tee::{detect_tee, is_tee_available, TeeCapability, TeeType};
pub use traits::{
    AuditSink, CacheBackend, CacheEntry, CacheStats, CreateExecutionRequest, CredentialProvider,
    EventBus, ExecutionCpuStats, ExecutionEventBatch, ExecutionEventKind, ExecutionEventsRequest,
    ExecutionGeneration, ExecutionHealthCheck, ExecutionId, ExecutionLease, ExecutionManager,
    ExecutionManagerError, ExecutionManagerResult, ExecutionMemoryStats, ExecutionPortConnector,
    ExecutionPortIo, ExecutionPortStream, ExecutionProcess, ExecutionProcessInfo,
    ExecutionProcessInput, ExecutionProcessInventory, ExecutionProcessSignal,
    ExecutionProcessStream, ExecutionRecordPolicy, ExecutionReservation, ExecutionResourceUpdate,
    ExecutionRestartPolicy, ExecutionRuntimeEvent, ExecutionSessionManager, ExecutionSnapshot,
    ExecutionSnapshotId, ExecutionState, ExecutionStats, ExecutionStatus, ExecutionUdpPort,
    ExecutionUdpPortIo, ImageRegistry, ImageStoreBackend, KillExecutionOptions, KillOutcome,
    MetricsCollector, NetworkStoreBackend, NoopMetrics, OperationId, PulledImage, ReconcileOutcome,
    RestartExecutionOptions, SnapshotStoreBackend, StoredImage, VolumeStoreBackend,
    MAX_EXECUTION_EVENT_BATCH_ITEMS,
};
pub use vmm::{
    Entrypoint, FsMount, InstanceSpec, NetworkInstanceConfig, RawBlockDevice, TeeInstanceConfig,
    VmHandler, VmMetrics, VmmProvider, DEFAULT_SHUTDOWN_TIMEOUT_MS,
};
pub use volume::VolumeConfig;

/// Kbox Box version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the Kbox home directory.
///
/// Resolution order:
/// 1. `K_HOME` environment variable (if set)
/// 2. `~/.kbox` (via `dirs::home_dir()`)
/// 3. Fallback to `.kbox` in the current directory
pub fn dirs_home() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("K_HOME") {
        return std::path::PathBuf::from(home);
    }
    dirs::home_dir()
        .map(|h| h.join(".kbox"))
        .unwrap_or_else(|| std::path::PathBuf::from(".kbox"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn dirs_home_prefers_khome_environment_variable() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("K_HOME");
        let tmp = tempfile::tempdir().unwrap();

        std::env::set_var("K_HOME", tmp.path());
        assert_eq!(dirs_home(), tmp.path());

        match previous {
            Some(value) => std::env::set_var("K_HOME", value),
            None => std::env::remove_var("K_HOME"),
        }
    }

    #[test]
    fn dirs_home_defaults_to_dot_kunder_user_home_when_env_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("K_HOME");

        std::env::remove_var("K_HOME");
        let home = dirs_home();

        assert!(home.ends_with(".kbox"));
        if let Some(user_home) = dirs::home_dir() {
            assert_eq!(home, user_home.join(".kbox"));
        }

        match previous {
            Some(value) => std::env::set_var("K_HOME", value),
            None => std::env::remove_var("K_HOME"),
        }
    }
}
