//! Kbox Box Runtime - MicroVM runtime implementation.
//!
//! This module provides the actual runtime implementation for Kbox Box,
//! including VM management, OCI image handling, rootfs building, and gRPC health checks.
//!
//! # Feature Flags
//!
//! - `pool` — Warm VM pool with autoscaling (enabled by default)
//! - `scale` — Multi-node scale manager and instance registry (enabled by default)
//! - `compose` — Multi-container compose orchestration (enabled by default)
//! - `operator` — Kubernetes CRD autoscaler controller (enabled by default)
//! - `build` — Dockerfile/Containerfile build engine (enabled by default)
//! - `runtime-provider-qualification` — Explicit downstream real-process
//!   qualification seam for the Linux Kbox Runtime provider (disabled by
//!   default; never a production capability-probe fallback)

#![allow(clippy::result_large_err)]

// -- Core modules (always compiled) --
pub mod audit;
pub mod box_record;
pub mod box_state;
pub mod cache;
#[cfg(all(feature = "pool", any(target_os = "linux", target_os = "macos")))]
pub mod engine;
pub(crate) mod file_lock;
pub mod fs;
pub mod grpc;
pub mod hardware_vm;
pub mod host_check;
pub mod host_fs;
pub mod host_sockets;
#[cfg(all(feature = "vm", any(target_os = "linux", target_os = "macos")))]
pub mod kbox_contract_driver;
pub mod local_execution;
pub mod log;
pub mod managed_execution_store;
pub mod network;
pub mod oci;
pub mod oci_portable_spec;
pub mod process;
mod process_path;
pub mod prom;
pub mod resize;
mod resolved_image;
pub mod rootfs;
pub mod snapshot;
mod store_io;
#[cfg(unix)]
pub mod tee;
#[cfg(feature = "vm")]
pub mod vm;
#[cfg(feature = "vm")]
pub mod vmm;
pub mod volume;

// -- Optional modules (feature-gated) --
#[cfg(feature = "compose")]
pub mod compose;
#[cfg(feature = "operator")]
pub mod operator;
#[cfg(feature = "pool")]
pub mod pool;
#[cfg(feature = "scale")]
pub mod scale;

// ── Core re-exports (used by CLI, CRI, SDK, shim) ──

// Audit
pub use audit::{read_audit_log, AuditLog, AuditQuery};

#[cfg(all(feature = "vm", any(target_os = "linux", target_os = "macos")))]
pub use kbox_contract_driver::{
    BoxArtifactPort, BoxArtifactPortError, BoxDirectoryPort, BoxDirectoryPortError,
    BoxHostServiceResolver, BoxRegistryCredential, BoxRuntimeDriver, BoxRuntimeDriverConfig,
    BoxRuntimeSevSnpConfig, BoxSecretEnvironmentProjection, BoxSecretMaterial,
    BoxSecretMaterializationError, BoxSecretMaterializer, BoxTransientSecretStore,
};

// Canonical local execution metadata
pub use box_record::{
    BoxRecord, HealthCheck, ManagedExecutionMetadata, ManagedExecutionOperation,
    ManagedExecutionState, ManagedResourceUpdateCompletion, ManagedRestartCompletion,
    ManagedRestartOutcome, ManagedRuntimeRoute,
};
pub use box_state::BoxStateStore;
pub use koci_sdk::{IO_READ_BYTES_METRIC, IO_WRITE_BYTES_METRIC};
pub use local_execution::{
    acquire_execution_lifecycle_lock, ExecutionLifecycleLock, FreshOwnerAdmission,
    LocalExecutionBackend, LocalExecutionBackendRouter, LocalExecutionHandle,
    LocalExecutionManager, LocalExecutionObservation, LocalExecutionOwnerFacts,
    LocalExecutionResourcePlan, LocalExecutionTermination, ManagedPrewarm,
    OciBundlePreparationContext, OciBundleProvider, OciLifecycleAdapter, OciLocalExecutionBackend,
    OciMigrationPolicy, OciPreparedExecution, OciRuntimeBinding, OciRuntimeEndpoint,
    OciRuntimeLaunch, OwnedLifecycleOutcome, OwnedLifecycleRequest,
    OCI_RUNTIME_BINDING_SCHEMA_VERSION,
};

/// Detach MicroVM `:ro` virtio-fs host RO-bind aliases under `box_dir/.filemounts`.
/// Absent aliases are success. Present mount detach failures retain the aliases
/// for retry (same contract as managed execution path cleanup).
pub fn cleanup_microvm_virtiofs_ro_shares(
    box_dir: &std::path::Path,
) -> kbox_core::error::Result<()> {
    #[cfg(feature = "vm")]
    {
        vm::cleanup_virtiofs_ro_shares(box_dir)
    }
    #[cfg(not(feature = "vm"))]
    {
        let _ = box_dir;
        Ok(())
    }
}

/// Tear down a leftover host-netdevice lease under `boxes/{id}/sandbox/`
/// (legacy layout dir named `sandbox`). Absent lease is success. Present
/// decode / DNAT / veth teardown failures fail closed.
pub fn cleanup_host_netdevice_lease(
    home_dir: &std::path::Path,
    box_id: &str,
) -> kbox_core::ExecutionManagerResult<()> {
    #[cfg(all(feature = "vm", target_os = "linux"))]
    {
        local_execution::oci_host_netdevice::teardown_lease(home_dir, box_id)
    }
    #[cfg(not(all(feature = "vm", target_os = "linux")))]
    {
        let _ = (home_dir, box_id);
        Ok(())
    }
}

#[cfg(feature = "vm")]
pub use local_execution::{
    LinuxKvmOciBundleProvider, LinuxKvmOciMigrationConfig, VmLocalExecutionBackend,
    WindowsWhpxOciBundleProvider, WindowsWhpxOciMigrationConfig,
};
pub use managed_execution_store::{
    ManagedExecutionReservation, ManagedExecutionStore, ManagedExecutionStoreError,
    ManagedExecutionStoreResult,
};
pub use process::{
    is_process_alive, is_process_alive_with_identity, is_process_running_with_identity,
    pid_start_time,
};

// gRPC clients
#[cfg(unix)]
pub use grpc::{
    AttestationClient, PtyClient, RaTlsAttestationClient, StreamingPty, StreamingPtyInput,
};
pub use grpc::{ExecClient, StreamingExec, StreamingExecInput};
#[cfg(unix)]
pub use grpc::{SealClient, SecretEntry, SecretInjector};

// Host checks
pub use host_check::check_virtualization_support;

// Network
pub use network::NetworkStore;

// OCI images
pub use kbox_core::{ExecutionBoundary, StoredImage};
pub use oci::{
    prune_stale_pull_temp_dirs, ImagePuller, ImageReference, ImageStore, PullProgress,
    PullProgressEventFn, PullProgressState, PullTempPruneResult, RegistryAuth, RegistryPullPolicy,
};
pub use oci::{CredentialStore, PushResult, RegistryProtocol, RegistryPusher};
pub use oci::{OciImage, SignResult, SignaturePolicy};

// Metrics
pub use prom::RuntimeMetrics;

// Snapshot
pub use resolved_image::{load_resolved_image_config, RESOLVED_IMAGE_CONFIG_FILE};
pub use snapshot::{
    RestoredSnapshotRootfs, SnapshotRootfsFormat, SnapshotStore, SNAPSHOT_ROOTFS_SCHEMA,
};

// TEE
#[cfg(unix)]
pub use tee::{seal, unseal};
#[cfg(unix)]
pub use tee::{
    verify_attestation, verify_attestation_with_time, AmdKdsClient, AttestationPolicy,
    MinTcbPolicy, PolicyResult, VerificationResult,
};
#[cfg(unix)]
pub use tee::{AttestationReport, AttestationRequest, PlatformInfo};

// VM
#[cfg(feature = "vm")]
pub use vm::{archive_stopped_guest_native_rootfs, BoxState, PullProgressFn, VmManager};
#[cfg(all(feature = "vm", target_os = "windows"))]
pub use vm::{
    clear_windows_guest_stop_request, finalize_box_terminal_rootfs_metadata,
    stage_windows_guest_stop_request, wait_windows_guest_stop_delivered,
    WINDOWS_GUEST_FINALIZATION_TIMEOUT_MS, WINDOWS_STOP_DELIVERY_TIMEOUT_MS,
};
#[cfg(feature = "vm")]
pub use vmm::{
    Entrypoint, FsMount, InstanceSpec, NetworkInstanceConfig, RawBlockDevice, ShimHandler,
    TeeInstanceConfig, VmController, VmHandler, VmMetrics, VmmProvider,
};

// Resize
pub use resize::{is_valid_cpuset, validate_update, ResizeResult, ResourceUpdate};

// Volume
pub use volume::VolumeStore;

// ── Feature-gated re-exports ──

#[cfg(feature = "build")]
pub use oci::{
    assemble_recorded_build_outputs, cancel_recorded_build_plan, execute_recorded_build_plan,
    hydrate_recorded_build_cache, inspect_recorded_build_plan, inspect_recorded_build_status,
    remove_recorded_build_plan, start_recorded_build_plan, BoxBuildOptions, BoxBuildPlan,
    BoxBuildPlanError, BuildAssemblyError, BuildCachePolicy, BuildCacheReceipt,
    BuildCancellationOutcome, BuildConfig, BuildNetworkPolicy, BuildOperationIdentity,
    BuildOutputAssembly, BuildOutputAssemblyInput, BuildOutputDescriptor, BuildOutputReceipt,
    BuildPlanExecutionError, BuildReceiptError, BuildReceiptOutput, BuildResult,
    BuildRunPoolConfig, Dockerfile, Instruction, MultiPlatformBuildResult, RecordedBuildCache,
    RecordedBuildResult, RecordedBuildStatus, BUILD_CACHE_ARTIFACT_MEDIA_TYPE,
    BUILD_CACHE_CONFIG_MEDIA_TYPE, OCI_IMAGE_INDEX_MEDIA_TYPE, OCI_IMAGE_MANIFEST_MEDIA_TYPE,
};

#[cfg(feature = "compose")]
#[allow(deprecated)]
pub use compose::{ComposeProject, ComposeRuntimePlan, HealthCheckSpec};

#[cfg(feature = "operator")]
pub use operator::AutoscalerController;

#[cfg(feature = "pool")]
pub use pool::WarmPool;

#[cfg(feature = "scale")]
pub use scale::{
    serve_scale_api, DurableScaleAuthority, LocalScaleReconciler, ScaleApiState,
    ScaleAuthorityError, ScaleCatalogError, ScaleEndpointConfig, ScaleEndpointConfigError,
    ScaleManager, ScaleReconcileError, ScaleReconcileObservation, ScaleReconcileReport,
    ScaleServiceCatalog, SharedScaleAuthority,
};

// ── Constants ──

/// Kbox Box Runtime version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use kbox_core::{ATTEST_VSOCK_PORT, EXEC_VSOCK_PORT, PORT_FWD_VSOCK_PORT, PTY_VSOCK_PORT};

/// Default maximum image cache size: 10 GB.
pub const DEFAULT_IMAGE_CACHE_SIZE: u64 = 10 * 1024 * 1024 * 1024;
