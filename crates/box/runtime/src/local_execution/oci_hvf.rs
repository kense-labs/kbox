//! Explicit Box composition with the independently owned macOS Host Service.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kbox_core::{ExecutionManagerError, ExecutionManagerResult};

use super::{LocalExecutionManager, OciMigrationPolicy, OciRuntimeEndpoint};

pub const OCI_HVF_ENDPOINT_ENV: &str = "K_BOX_OCI_HVF_ENDPOINT";

/// Connection to the public same-UID Host Service; Box does not own its process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacosHvfOciMigrationConfig {
    runtime_root: PathBuf,
    endpoint: OciRuntimeEndpoint,
    creation_policy: OciMigrationPolicy,
}

impl MacosHvfOciMigrationConfig {
    pub fn new(
        runtime_root: impl Into<PathBuf>,
        endpoint: impl Into<PathBuf>,
    ) -> ExecutionManagerResult<Self> {
        let runtime_root = runtime_root.into();
        let endpoint = endpoint.into();
        super::oci_migration::validate_absolute_normalized(&runtime_root, "HVF runtime root")?;
        super::oci_migration::validate_absolute_normalized(&endpoint, "HVF endpoint")?;
        let service = endpoint
            .parent()
            .ok_or_else(|| invalid("HVF endpoint has no parent"))?;
        if endpoint
            .file_name()
            .is_none_or(|name| name != "runtime.sock")
            || runtime_root != service.join("runtime")
        {
            return Err(invalid(
                "HVF endpoint/runtime root must use the Host Service's runtime.sock/runtime layout",
            ));
        }
        Ok(Self {
            runtime_root,
            endpoint: OciRuntimeEndpoint::unix_socket(endpoint)?,
            creation_policy: OciMigrationPolicy::MicrovmViaOci,
        })
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }
    pub fn endpoint(&self) -> &OciRuntimeEndpoint {
        &self.endpoint
    }

    /// Change new reservations only, keeping both retained routes available.
    pub fn with_creation_policy(mut self, policy: OciMigrationPolicy) -> Self {
        self.creation_policy = policy;
        self
    }

    pub(super) fn from_environment() -> ExecutionManagerResult<Option<Self>> {
        parse(
            std::env::var_os(super::oci_migration::OCI_MIGRATION_ENV),
            std::env::var_os(super::oci_migration::OCI_HOST_ROOT_ENV),
            std::env::var_os(OCI_HVF_ENDPOINT_ENV),
        )
    }
}

fn invalid(message: &str) -> ExecutionManagerError {
    ExecutionManagerError::InvalidRequest(message.into())
}

fn parse(
    mode: Option<OsString>,
    root: Option<OsString>,
    endpoint: Option<OsString>,
) -> ExecutionManagerResult<Option<MacosHvfOciMigrationConfig>> {
    let mode = mode
        .as_deref()
        .map(|value| {
            value
                .to_str()
                .ok_or_else(|| invalid("OCI migration mode must be UTF-8"))
        })
        .transpose()?;
    let policy = match mode.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        None | Some("") => None,
        Some("0" | "false" | "off" | "disabled" | "legacy") => Some(OciMigrationPolicy::LegacyOnly),
        Some("microvm" | "microvm-via-oci" | "hvf" | "all" | "all-via-oci") => Some(OciMigrationPolicy::MicrovmViaOci),
        _ => return Err(invalid("HVF migration requires off or explicit MicroVM DedicatedVm; shared-kernel migration is unsupported")),
    };
    let Some(endpoint) = endpoint.filter(|path| !path.is_empty()) else {
        if root.is_some() || policy.is_some_and(|value| value != OciMigrationPolicy::LegacyOnly) {
            return Err(invalid(
                "explicit HVF migration requires K_BOX_OCI_HVF_ENDPOINT; no BoxVm fallback",
            ));
        }
        return Ok(None);
    };
    let endpoint = PathBuf::from(endpoint);
    let runtime = root
        .map(PathBuf::from)
        .unwrap_or_else(|| endpoint.parent().unwrap_or(Path::new("")).join("runtime"));
    Ok(Some(
        MacosHvfOciMigrationConfig::new(runtime, endpoint)?
            .with_creation_policy(policy.unwrap_or(OciMigrationPolicy::MicrovmViaOci)),
    ))
}

impl LocalExecutionManager {
    /// Explicit migration through the public SDK. The caller owns service startup.
    pub async fn with_macos_hvf_oci_qualification(
        state_path: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
        config: MacosHvfOciMigrationConfig,
    ) -> ExecutionManagerResult<Self> {
        Self::with_macos_hvf_oci_and_pull_progress(state_path.into(), home_dir.into(), config, None)
            .await
    }

    pub(super) async fn with_macos_hvf_oci_and_pull_progress(
        state_path: PathBuf,
        home_dir: PathBuf,
        config: MacosHvfOciMigrationConfig,
        progress: Option<crate::PullProgressFn>,
    ) -> ExecutionManagerResult<Self> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            use koci_sdk::{
                DriverKind, IsolationClass, RUNTIME_BUNDLE_HANDOFF_EXTENSION,
                RUNTIME_BUNDLE_HANDOFF_EXTERNAL_STORAGE_VERSION,
            };
            use std::sync::Arc;
            let adapter = super::OciLifecycleAdapter::connect(config.endpoint.clone()).await?;
            let info = adapter
                .require_isolation(kbox_core::ExecutionBoundary::Microvm)
                .await?;
            if !info.drivers.drivers.iter().any(|driver| {
                driver.driver == DriverKind::LibkHvf
                    && driver.can_launch()
                    && driver
                        .isolation_classes
                        .contains(&IsolationClass::DedicatedVm)
            }) || !info.attachments.supports_extension(
                RUNTIME_BUNDLE_HANDOFF_EXTENSION,
                RUNTIME_BUNDLE_HANDOFF_EXTERNAL_STORAGE_VERSION,
            ) {
                return Err(ExecutionManagerError::Unavailable("endpoint does not advertise the launch-ready HVF DedicatedVm bundle-handoff contract".into()));
            }
            let mut provider =
                super::MacosHvfOciBundleProvider::new(home_dir.clone(), config.runtime_root);
            if let Some(progress) = &progress {
                provider = provider.with_pull_progress_fn(progress.clone());
            }
            let oci = super::OciLocalExecutionBackend::from_client(
                config.endpoint,
                adapter.client(),
                Arc::new(provider),
            )?;
            Ok(Self::with_oci_migration_backend_and_pull_progress(
                state_path,
                home_dir,
                Arc::new(oci),
                config.creation_policy,
                progress,
            ))
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            let _ = (state_path, home_dir, config, progress);
            Err(ExecutionManagerError::Unavailable(
                "Box/HVF OCI qualification requires macOS arm64".into(),
            ))
        }
    }
}

#[cfg(test)]
#[path = "oci_hvf_tests.rs"]
mod tests;
