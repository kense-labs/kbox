//! Explicit production composition for MicroVM DedicatedVm OCI qualification.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use kbox_core::{ExecutionManagerError, ExecutionManagerResult};
#[cfg(target_os = "linux")]
use sha2::{Digest, Sha256};

use super::LocalExecutionManager;
#[cfg(target_os = "linux")]
use super::{LinuxKvmOciBundleProvider, OciLocalExecutionBackend, OciMigrationPolicy};

pub const OCI_MIGRATION_ENV: &str = "K_BOX_OCI_MIGRATION";
pub const OCI_HOST_ROOT_ENV: &str = "K_BOX_OCI_HOST_ROOT";
pub const OCI_WHPX_ENDPOINT_ENV: &str = "K_BOX_OCI_WHPX_ENDPOINT";
pub const OCI_KVM_ENDPOINT_ENV: &str = "K_BOX_OCI_KVM_ENDPOINT";
pub const OCI_KVM_BOX_OWNED_ENV: &str = "K_BOX_KVM_OCI_BOX_OWNED";
pub const OCI_KVM_SERVICE_ROOT_ENV: &str = "K_BOX_KVM_OCI_SERVICE_ROOT";
pub const OCI_KVM_SERVICE_BIN_ENV: &str = "K_BOX_KVM_OCI_SERVICE_BIN";
pub const OCI_KVM_SERVICE_SHIM_ENV: &str = "K_BOX_KVM_OCI_SERVICE_SHIM";
pub const OCI_KVM_SERVICE_MANIFEST_ENV: &str = "K_BOX_KVM_OCI_SERVICE_MANIFEST";
pub const OCI_WHPX_BOX_OWNED_ENV: &str = "K_BOX_WHPX_OCI_BOX_OWNED";
pub const OCI_WHPX_SERVICE_ROOT_ENV: &str = "K_BOX_WHPX_OCI_SERVICE_ROOT";
pub const OCI_WHPX_SERVICE_BIN_ENV: &str = "K_BOX_WHPX_OCI_SERVICE_BIN";
pub const OCI_WHPX_SERVICE_SHIM_ENV: &str = "K_BOX_WHPX_OCI_SERVICE_SHIM";
pub const OCI_WHPX_SERVICE_VM_ROOTFS_ENV: &str = "K_BOX_WHPX_OCI_SERVICE_VM_ROOTFS";
pub const OCI_WHPX_SERVICE_MANIFEST_ENV: &str = "K_BOX_WHPX_OCI_SERVICE_MANIFEST";
#[cfg(test)]
const DEFAULT_OCI_WHPX_ENDPOINT: &str = r"\\.\pipe\koci-box-qualification";

/// Explicit connection to an externally owned qualification-only WHPX service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsWhpxOciMigrationConfig {
    runtime_root: PathBuf,
    endpoint: super::OciRuntimeEndpoint,
    box_owned_owner: Option<WindowsWhpxBoxOwnedOwner>,
}

/// Optional Box-owned Host ensure inputs for Windows WHPX qualification.
///
/// When set, construction identity-fences and (re)spawns
/// `box-whpx-qualification-service` under `service_root`. External-only connect
/// remains available when this is absent so existing operator-launched Hosts
/// keep working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsWhpxBoxOwnedOwner {
    service_root: PathBuf,
    runtime_path: PathBuf,
    shim_path: PathBuf,
    vm_rootfs: PathBuf,
    system_image_manifest: PathBuf,
}

impl WindowsWhpxBoxOwnedOwner {
    pub fn service_root(&self) -> &Path {
        &self.service_root
    }

    pub fn runtime_path(&self) -> &Path {
        &self.runtime_path
    }

    pub fn shim_path(&self) -> &Path {
        &self.shim_path
    }

    pub fn vm_rootfs(&self) -> &Path {
        &self.vm_rootfs
    }

    pub fn system_image_manifest(&self) -> &Path {
        &self.system_image_manifest
    }
}

/// Optional Box-owned Host ensure inputs for Linux KVM qualification.
///
/// When set, construction identity-fences and (re)spawns
/// `box-kvm-qualification-service` under `service_root`. External-only connect
/// remains available when this is absent so existing operator-launched Hosts
/// keep working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxKvmBoxOwnedOwner {
    service_root: PathBuf,
    runtime_path: PathBuf,
    shim_path: PathBuf,
    system_image_manifest: PathBuf,
}

impl LinuxKvmBoxOwnedOwner {
    pub fn service_root(&self) -> &Path {
        &self.service_root
    }

    pub fn runtime_path(&self) -> &Path {
        &self.runtime_path
    }

    pub fn shim_path(&self) -> &Path {
        &self.shim_path
    }

    pub fn system_image_manifest(&self) -> &Path {
        &self.system_image_manifest
    }
}

/// Explicit connection to a qualification-only Linux KVM service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxKvmOciMigrationConfig {
    runtime_root: PathBuf,
    endpoint: super::OciRuntimeEndpoint,
    box_owned_owner: Option<LinuxKvmBoxOwnedOwner>,
}

impl LinuxKvmOciMigrationConfig {
    pub fn new(
        runtime_root: impl Into<PathBuf>,
        endpoint: impl Into<PathBuf>,
    ) -> ExecutionManagerResult<Self> {
        let config = Self {
            runtime_root: runtime_root.into(),
            endpoint: super::OciRuntimeEndpoint::unix_socket(endpoint)?,
            box_owned_owner: None,
        };
        config.validate()?;
        Ok(config)
    }

    /// Attach Box-owned Host ensure/recovery for the qualification service.
    ///
    /// The configured endpoint must be `{service_root}/runtime.sock`. This does
    /// not select production MicroVM routing.
    pub fn with_box_owned_owner(
        mut self,
        service_root: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
        shim_path: impl Into<PathBuf>,
        system_image_manifest: impl Into<PathBuf>,
    ) -> ExecutionManagerResult<Self> {
        self.box_owned_owner = Some(LinuxKvmBoxOwnedOwner {
            service_root: service_root.into(),
            runtime_path: runtime_path.into(),
            shim_path: shim_path.into(),
            system_image_manifest: system_image_manifest.into(),
        });
        self.validate()?;
        Ok(self)
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    pub fn endpoint(&self) -> &super::OciRuntimeEndpoint {
        &self.endpoint
    }

    pub fn box_owned_owner(&self) -> Option<&LinuxKvmBoxOwnedOwner> {
        self.box_owned_owner.as_ref()
    }

    pub fn from_environment(home_dir: &Path) -> ExecutionManagerResult<Option<Self>> {
        parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: std::env::var_os(OCI_MIGRATION_ENV),
                runtime_root: std::env::var_os(OCI_HOST_ROOT_ENV),
                endpoint: std::env::var_os(OCI_KVM_ENDPOINT_ENV),
                box_owned: std::env::var_os(OCI_KVM_BOX_OWNED_ENV),
                service_root: std::env::var_os(OCI_KVM_SERVICE_ROOT_ENV),
                service_bin: std::env::var_os(OCI_KVM_SERVICE_BIN_ENV),
                service_shim: std::env::var_os(OCI_KVM_SERVICE_SHIM_ENV),
                service_manifest: std::env::var_os(OCI_KVM_SERVICE_MANIFEST_ENV),
            },
            home_dir,
        )
    }

    fn validate(&self) -> ExecutionManagerResult<()> {
        validate_absolute_normalized(&self.runtime_root, "KVM OCI runtime root")?;
        match &self.endpoint {
            super::OciRuntimeEndpoint::UnixSocket { path } => {
                if let Some(owner) = &self.box_owned_owner {
                    validate_absolute_normalized(&owner.service_root, "KVM OCI service root")?;
                    validate_absolute_normalized(&owner.runtime_path, "KVM OCI runtime binary")?;
                    validate_absolute_normalized(&owner.shim_path, "KVM OCI shim")?;
                    validate_absolute_normalized(
                        &owner.system_image_manifest,
                        "KVM OCI system-image manifest",
                    )?;
                    let expected = owner.service_root.join("runtime.sock");
                    if path != &expected {
                        return Err(ExecutionManagerError::InvalidRequest(format!(
                            "Box-owned KVM OCI endpoint must be {} (got {})",
                            expected.display(),
                            path.display()
                        )));
                    }
                }
                Ok(())
            }
            super::OciRuntimeEndpoint::WindowsNamedPipe { .. } => {
                Err(ExecutionManagerError::InvalidRequest(
                    "KVM OCI qualification requires a Unix-domain socket endpoint".to_string(),
                ))
            }
        }
    }
}

impl WindowsWhpxOciMigrationConfig {
    pub fn new(
        runtime_root: impl Into<PathBuf>,
        endpoint: impl Into<String>,
    ) -> ExecutionManagerResult<Self> {
        let config = Self {
            runtime_root: runtime_root.into(),
            endpoint: super::OciRuntimeEndpoint::windows_named_pipe(endpoint)?,
            box_owned_owner: None,
        };
        config.validate()?;
        Ok(config)
    }

    /// Attach Box-owned Host ensure/recovery for the qualification service.
    ///
    /// The configured endpoint must be the deterministic pipe derived from
    /// `service_root`. This does not select production MicroVM routing.
    pub fn with_box_owned_owner(
        mut self,
        service_root: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
        shim_path: impl Into<PathBuf>,
        vm_rootfs: impl Into<PathBuf>,
        system_image_manifest: impl Into<PathBuf>,
    ) -> ExecutionManagerResult<Self> {
        self.box_owned_owner = Some(WindowsWhpxBoxOwnedOwner {
            service_root: service_root.into(),
            runtime_path: runtime_path.into(),
            shim_path: shim_path.into(),
            vm_rootfs: vm_rootfs.into(),
            system_image_manifest: system_image_manifest.into(),
        });
        self.validate()?;
        Ok(self)
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    pub fn endpoint(&self) -> &super::OciRuntimeEndpoint {
        &self.endpoint
    }

    pub fn box_owned_owner(&self) -> Option<&WindowsWhpxBoxOwnedOwner> {
        self.box_owned_owner.as_ref()
    }

    pub fn from_environment(home_dir: &Path) -> ExecutionManagerResult<Option<Self>> {
        parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: std::env::var_os(OCI_MIGRATION_ENV),
                runtime_root: std::env::var_os(OCI_HOST_ROOT_ENV),
                endpoint: std::env::var_os(OCI_WHPX_ENDPOINT_ENV),
                box_owned: std::env::var_os(OCI_WHPX_BOX_OWNED_ENV),
                service_root: std::env::var_os(OCI_WHPX_SERVICE_ROOT_ENV),
                service_bin: std::env::var_os(OCI_WHPX_SERVICE_BIN_ENV),
                service_shim: std::env::var_os(OCI_WHPX_SERVICE_SHIM_ENV),
                service_vm_rootfs: std::env::var_os(OCI_WHPX_SERVICE_VM_ROOTFS_ENV),
                service_manifest: std::env::var_os(OCI_WHPX_SERVICE_MANIFEST_ENV),
            },
            home_dir,
        )
    }

    fn validate(&self) -> ExecutionManagerResult<()> {
        validate_absolute_normalized(&self.runtime_root, "WHPX OCI runtime root")?;
        match &self.endpoint {
            super::OciRuntimeEndpoint::WindowsNamedPipe { name } => {
                if let Some(owner) = &self.box_owned_owner {
                    validate_absolute_normalized(&owner.service_root, "WHPX OCI service root")?;
                    validate_absolute_normalized(&owner.runtime_path, "WHPX OCI runtime binary")?;
                    validate_absolute_normalized(&owner.shim_path, "WHPX OCI shim")?;
                    validate_absolute_normalized(&owner.vm_rootfs, "WHPX OCI vm-rootfs")?;
                    validate_absolute_normalized(
                        &owner.system_image_manifest,
                        "WHPX OCI system-image manifest",
                    )?;
                    let expected = super::oci_whpx_owner::owned_pipe_name(&owner.service_root)?;
                    if name != &expected {
                        return Err(ExecutionManagerError::InvalidRequest(format!(
                            "Box-owned WHPX OCI endpoint must be {expected} (got {name})"
                        )));
                    }
                }
                Ok(())
            }
            super::OciRuntimeEndpoint::UnixSocket { .. } => {
                Err(ExecutionManagerError::InvalidRequest(
                    "WHPX OCI qualification requires a Windows named-pipe endpoint".to_string(),
                ))
            }
        }
    }
}

impl LocalExecutionManager {
    /// Compose the retained backend with the externally launched Box/WHPX OCI service.
    pub async fn with_windows_whpx_oci_qualification(
        state_path: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
        config: WindowsWhpxOciMigrationConfig,
    ) -> ExecutionManagerResult<Self> {
        Self::with_windows_whpx_oci_qualification_and_pull_progress(
            state_path.into(),
            home_dir.into(),
            config,
            None,
        )
        .await
    }

    async fn with_windows_whpx_oci_qualification_and_pull_progress(
        state_path: PathBuf,
        home_dir: PathBuf,
        config: WindowsWhpxOciMigrationConfig,
        pull_progress_fn: Option<crate::PullProgressFn>,
    ) -> ExecutionManagerResult<Self> {
        config.validate()?;

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            let mut provider =
                super::WindowsWhpxOciBundleProvider::new(home_dir.clone(), config.runtime_root());
            if let Some(progress) = pull_progress_fn.as_ref() {
                provider = provider.with_pull_progress_fn(progress.clone());
            }
            let provider = Arc::new(provider);
            let oci = if let Some(owner) = config.box_owned_owner() {
                let artifacts = super::oci_whpx_owner::WindowsWhpxOwnerArtifacts::certify(
                    owner.runtime_path.clone(),
                    owner.shim_path.clone(),
                    owner.vm_rootfs.clone(),
                    owner.system_image_manifest.clone(),
                )?;
                let endpoint = super::oci_whpx_owner::ensure_windows_whpx_oci_owner(
                    &owner.service_root,
                    &artifacts,
                )
                .await?;
                if &endpoint != config.endpoint() {
                    return Err(ExecutionManagerError::Internal(format!(
                        "Windows WHPX OCI owner ensure returned a different endpoint ({endpoint:?}) than configured ({:?})",
                        config.endpoint()
                    )));
                }
                Arc::new(
                    super::OciLocalExecutionBackend::connect(endpoint, provider)
                        .await?
                        .with_windows_whpx_owner_recovery(owner.service_root.clone(), artifacts),
                )
            } else {
                Arc::new(
                    super::OciLocalExecutionBackend::connect(config.endpoint().clone(), provider)
                        .await?,
                )
            };
            Ok(Self::with_oci_migration_backend_and_pull_progress(
                state_path,
                home_dir,
                oci,
                super::OciMigrationPolicy::MicrovmViaOci,
                pull_progress_fn,
            ))
        }

        #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
        {
            let _ = (state_path, home_dir, config, pull_progress_fn);
            Err(ExecutionManagerError::Unavailable(
                "Box/WHPX OCI qualification requires Windows x86_64".to_string(),
            ))
        }
    }

    /// Compose the retained backend with the externally launched Box/KVM OCI service.
    pub async fn with_linux_kvm_oci_qualification(
        state_path: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
        config: LinuxKvmOciMigrationConfig,
    ) -> ExecutionManagerResult<Self> {
        Self::with_linux_kvm_oci_qualification_and_pull_progress(
            state_path.into(),
            home_dir.into(),
            config,
            None,
        )
        .await
    }

    async fn with_linux_kvm_oci_qualification_and_pull_progress(
        state_path: PathBuf,
        home_dir: PathBuf,
        config: LinuxKvmOciMigrationConfig,
        pull_progress_fn: Option<crate::PullProgressFn>,
    ) -> ExecutionManagerResult<Self> {
        config.validate()?;

        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))]
        {
            let mut provider =
                LinuxKvmOciBundleProvider::new(home_dir.clone(), config.runtime_root());
            if let Some(progress) = pull_progress_fn.as_ref() {
                provider = provider.with_pull_progress_fn(progress.clone());
            }
            let provider = Arc::new(provider);
            let oci = if let Some(owner) = config.box_owned_owner() {
                let artifacts = super::oci_kvm_owner::LinuxKvmOwnerArtifacts::certify(
                    owner.runtime_path.clone(),
                    owner.shim_path.clone(),
                    owner.system_image_manifest.clone(),
                )?;
                // Fresh KVM qualification construction: reclaiming a dead Host
                // must tear down Live-survivable session-owner/shim orphans.
                // Retained-manager Live reopen keeps the default (no reap).
                let endpoint = super::oci_kvm_owner::ensure_linux_kvm_oci_owner_with_options(
                    &owner.service_root,
                    &artifacts,
                    super::oci_kvm_owner::EnsureLinuxKvmOwnerOptions {
                        reap_orphaned_session_children: true,
                        runtime_root: Some(config.runtime_root().to_path_buf()),
                    },
                )
                .await?;
                if &endpoint != config.endpoint() {
                    return Err(ExecutionManagerError::Internal(format!(
                        "Linux KVM OCI owner ensure returned a different endpoint ({endpoint:?}) than configured ({:?})",
                        config.endpoint()
                    )));
                }
                Arc::new(
                    OciLocalExecutionBackend::connect(endpoint, provider)
                        .await?
                        .with_linux_kvm_owner_recovery(owner.service_root.clone(), artifacts),
                )
            } else {
                Arc::new(
                    OciLocalExecutionBackend::connect(config.endpoint().clone(), provider).await?,
                )
            };
            Ok(Self::with_oci_migration_backend_and_pull_progress(
                state_path,
                home_dir,
                oci,
                OciMigrationPolicy::MicrovmViaOci,
                pull_progress_fn,
            ))
        }

        #[cfg(not(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )))]
        {
            let _ = (state_path, home_dir, config, pull_progress_fn);
            Err(ExecutionManagerError::Unavailable(
                "Box/KVM OCI qualification requires Linux x86_64 or aarch64".to_string(),
            ))
        }
    }

    /// Select the configured local-execution backend composition.
    ///
    /// MicroVM is the product path. On Linux, packaged KVM DedicatedVm is
    /// preferred when artifacts are discoverable; otherwise Box-libkrun.
    /// Explicit `off`/`legacy` keeps the legacy VM backend. Historical
    /// Sandbox migration values are rejected.
    ///
    /// On Windows x86_64, packaged WHPX DedicatedVm is preferred when Host
    /// artifacts are discoverable; missing packages soft-fall to Box-libkrun.
    /// Explicit `off`/`legacy` keeps the legacy VM backend.
    pub async fn with_configured_backend(
        state_path: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
    ) -> ExecutionManagerResult<Self> {
        Self::with_configured_backend_and_pull_progress(state_path, home_dir, None).await
    }

    /// Configured construction retaining the CLI's image-pull progress hook on
    /// both the legacy and migrated preparation paths.
    pub async fn with_configured_backend_and_pull_progress(
        state_path: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
        pull_progress_fn: Option<crate::PullProgressFn>,
    ) -> ExecutionManagerResult<Self> {
        let state_path = state_path.into();
        let home_dir = home_dir.into();

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            match super::MacosHvfOciMigrationConfig::from_environment()? {
                Some(config) => {
                    Self::with_macos_hvf_oci_and_pull_progress(
                        state_path,
                        home_dir,
                        config,
                        pull_progress_fn,
                    )
                    .await
                }
                None => legacy_backend(state_path, home_dir, pull_progress_fn),
            }
        }

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        if std::env::var_os(super::oci_hvf::OCI_HVF_ENDPOINT_ENV).is_some() {
            return Err(ExecutionManagerError::Unavailable(
                "explicit HVF endpoint requires macOS arm64; no backend fallback".into(),
            ));
        }

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            match WindowsWhpxOciMigrationConfig::from_environment(&home_dir)? {
                Some(config) => {
                    Self::with_windows_whpx_oci_qualification_and_pull_progress(
                        state_path,
                        home_dir,
                        config,
                        pull_progress_fn,
                    )
                    .await
                }
                None => legacy_backend(state_path, home_dir, pull_progress_fn),
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(config) = LinuxKvmOciMigrationConfig::from_environment(&home_dir)? {
                return Self::with_linux_kvm_oci_qualification_and_pull_progress(
                    state_path,
                    home_dir,
                    config,
                    pull_progress_fn,
                )
                .await;
            }
            reject_removed_sandbox_migration_env()?;
            legacy_backend(state_path, home_dir, pull_progress_fn)
        }

        #[cfg(not(any(
            target_os = "linux",
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64")
        )))]
        {
            reject_removed_sandbox_migration_env()?;
            legacy_backend(state_path, home_dir, pull_progress_fn)
        }
    }
}

fn legacy_backend(
    state_path: PathBuf,
    home_dir: PathBuf,
    pull_progress_fn: Option<crate::PullProgressFn>,
) -> ExecutionManagerResult<LocalExecutionManager> {
    let mut backend = crate::local_execution::VmLocalExecutionBackend::new(&home_dir);
    if let Some(progress) = pull_progress_fn {
        backend = backend.with_pull_progress_fn(progress);
    }
    Ok(LocalExecutionManager::new(
        state_path,
        home_dir,
        Arc::new(backend),
    ))
}

fn reject_removed_sandbox_migration_env() -> ExecutionManagerResult<()> {
    let Some(mode) = std::env::var_os(OCI_MIGRATION_ENV).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let mode = mode.to_str().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(format!(
            "{OCI_MIGRATION_ENV} must contain UTF-8 text"
        ))
    })?;
    match mode.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "disabled" | "legacy" => Ok(()),
        "1" | "true" | "on" | "sandbox" | "sandbox-via-oci" | "all" | "all-via-oci" => {
            Err(ExecutionManagerError::InvalidRequest(format!(
                "legacy shared-kernel OCI migration was removed; set {OCI_MIGRATION_ENV}=off for MicroVM-only or use {OCI_KVM_ENDPOINT_ENV} / LinuxKvmOciMigrationConfig for DedicatedVm"
            )))
        }
        "microvm" | "microvm-via-oci" | "kvm" => Ok(()),
        value => Err(ExecutionManagerError::InvalidRequest(format!(
            "unsupported {OCI_MIGRATION_ENV} value {value:?}; expected off or MicroVM DedicatedVm qualification"
        ))),
    }
}

/// Explicit on/off flag for qualification Host ownership.
///
/// Absent or empty stays off. Unknown text fails closed so a typo cannot
/// silently select the external qualification endpoint.
fn parse_explicit_bool(env_name: &str, value: Option<OsString>) -> ExecutionManagerResult<bool> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    let value = value.to_str().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(format!("{env_name} must contain UTF-8 text"))
    })?;
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "no" => Ok(false),
        "1" | "true" | "on" | "yes" | "box-owned" => Ok(true),
        other => Err(ExecutionManagerError::InvalidRequest(format!(
            "unsupported {env_name} value {other:?}; expected 0/1, false/true, off/on, no/yes, or box-owned"
        ))),
    }
}

fn parse_windows_environment(
    inputs: WindowsWhpxEnvironmentInputs,
    home_dir: &Path,
) -> ExecutionManagerResult<Option<WindowsWhpxOciMigrationConfig>> {
    let WindowsWhpxEnvironmentInputs {
        mode,
        runtime_root,
        endpoint,
        box_owned,
        service_root,
        service_bin,
        service_shim,
        service_vm_rootfs,
        service_manifest,
    } = inputs;
    let Some(mode) = mode.filter(|value| !value.is_empty()) else {
        // Gate 5: absent `K_BOX_OCI_MIGRATION` selects packaged DedicatedVm
        // when runtime/shim/vm-rootfs/system-image are discoverable. Missing
        // packages soft-fall to Box-libkrun/WHPX instead of hard-failing hosts
        // without the WHPX Host package.
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            let _ = (endpoint, box_owned);
            let host_root_override = runtime_root
                .filter(|value| !value.is_empty())
                .map(PathBuf::from);
            return match parse_windows_whpx_packaged_box_owned(
                home_dir,
                host_root_override,
                service_root,
                service_bin,
                service_shim,
                service_vm_rootfs,
                service_manifest,
            ) {
                Ok(config) => Ok(Some(config)),
                Err(ExecutionManagerError::InvalidRequest(message))
                    if message.contains("was not found in packaged Kbox locations") =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            };
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
        {
            let _ = (
                runtime_root,
                endpoint,
                box_owned,
                service_root,
                service_bin,
                service_shim,
                service_vm_rootfs,
                service_manifest,
            );
            return Ok(None);
        }
    };
    let mode = mode.to_str().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(format!(
            "{OCI_MIGRATION_ENV} must contain UTF-8 text"
        ))
    })?;
    match mode.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "disabled" | "legacy" => return Ok(None),
        "all" | "all-via-oci" | "microvm" | "microvm-via-oci" | "whpx" => {}
        "1" | "true" | "on" | "sandbox" | "sandbox-via-oci" => {
            return Err(ExecutionManagerError::InvalidRequest(
                "Windows OCI qualification supports only MicroVM/WHPX routing; use all or microvm"
                    .to_string(),
            ))
        }
        value => {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "unsupported {OCI_MIGRATION_ENV} value {value:?}; expected off, microvm, or all"
            )))
        }
    }

    let explicit_host_root = runtime_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let runtime_root = explicit_host_root
        .clone()
        .unwrap_or_else(|| default_service_root(home_dir));
    let box_owned = parse_explicit_bool(OCI_WHPX_BOX_OWNED_ENV, box_owned)?;

    if box_owned {
        let service_root = service_root
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_SERVICE_ROOT_ENV} must be set when {OCI_WHPX_BOX_OWNED_ENV} is enabled"
                ))
            })?;
        let service_bin = service_bin
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_SERVICE_BIN_ENV} must be set when {OCI_WHPX_BOX_OWNED_ENV} is enabled"
                ))
            })?;
        let service_shim = service_shim
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_SERVICE_SHIM_ENV} must be set when {OCI_WHPX_BOX_OWNED_ENV} is enabled"
                ))
            })?;
        let service_vm_rootfs = service_vm_rootfs
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_SERVICE_VM_ROOTFS_ENV} must be set when {OCI_WHPX_BOX_OWNED_ENV} is enabled"
                ))
            })?;
        let service_manifest = service_manifest
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_SERVICE_MANIFEST_ENV} must be set when {OCI_WHPX_BOX_OWNED_ENV} is enabled"
                ))
            })?;
        let endpoint = match endpoint.filter(|value| !value.is_empty()) {
            Some(value) => value.into_string().map_err(|_| {
                ExecutionManagerError::InvalidRequest(format!(
                    "{OCI_WHPX_ENDPOINT_ENV} must contain UTF-8 text"
                ))
            })?,
            None => super::oci_whpx_owner::owned_pipe_name(&service_root)?,
        };
        return WindowsWhpxOciMigrationConfig::new(runtime_root, endpoint)?
            .with_box_owned_owner(
                service_root,
                service_bin,
                service_shim,
                service_vm_rootfs,
                service_manifest,
            )
            .map(Some);
    }

    let endpoint = endpoint.filter(|value| !value.is_empty());
    if let Some(endpoint) = endpoint {
        let endpoint = endpoint.into_string().map_err(|_| {
            ExecutionManagerError::InvalidRequest(format!(
                "{OCI_WHPX_ENDPOINT_ENV} must contain UTF-8 text"
            ))
        })?;
        return WindowsWhpxOciMigrationConfig::new(runtime_root, endpoint).map(Some);
    }

    // Opt-in microvm|all without qualification endpoint → packaged Box-owned Host.
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return parse_windows_whpx_packaged_box_owned(
            home_dir,
            explicit_host_root,
            service_root,
            service_bin,
            service_shim,
            service_vm_rootfs,
            service_manifest,
        )
        .map(Some);
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    {
        let _ = (
            service_root,
            service_bin,
            service_shim,
            service_vm_rootfs,
            service_manifest,
        );
        Err(ExecutionManagerError::InvalidRequest(format!(
            "{OCI_WHPX_ENDPOINT_ENV} must be set explicitly for the qualification-only WHPX service"
        )))
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn parse_windows_whpx_packaged_box_owned(
    home_dir: &Path,
    host_root_override: Option<PathBuf>,
    service_root: Option<OsString>,
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_vm_rootfs: Option<OsString>,
    service_manifest: Option<OsString>,
) -> ExecutionManagerResult<WindowsWhpxOciMigrationConfig> {
    let discovered = super::oci_whpx_packaged::discover_packaged_windows_whpx_artifacts(
        super::oci_whpx_packaged::PackagedWindowsWhpxOverrides {
            runtime_path: service_bin
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            shim_path: service_shim
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            vm_rootfs: service_vm_rootfs
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            system_image_manifest: service_manifest
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
        },
    )?;
    // Mutable service/runtime root must stay disjoint from immutable
    // system-image. Packaged `bootstrap-vm-rootfs/` beside Host binaries is a
    // seed; ensure materializes the live bootstrap under the service root.
    let service_root = service_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or(host_root_override)
        .unwrap_or_else(|| default_service_root(home_dir));
    std::fs::create_dir_all(&service_root).map_err(|error| {
        ExecutionManagerError::InvalidRequest(format!(
            "failed to create Windows WHPX OCI service root {}: {error}",
            service_root.display()
        ))
    })?;
    let service_root = service_root.canonicalize().map_err(|error| {
        ExecutionManagerError::InvalidRequest(format!(
            "Windows WHPX OCI service root must resolve to an existing directory {}: {error}",
            service_root.display()
        ))
    })?;
    let system_image_directory = discovered.system_image_manifest.parent().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(format!(
            "packaged WHPX system-image manifest has no parent directory: {}",
            discovered.system_image_manifest.display()
        ))
    })?;
    if discovered.system_image_manifest.starts_with(&service_root)
        || service_root.starts_with(system_image_directory)
    {
        return Err(ExecutionManagerError::InvalidRequest(format!(
            "packaged WHPX system-image {} and mutable service root {} must be disjoint (do not set {OCI_WHPX_SERVICE_ROOT_ENV} / {OCI_HOST_ROOT_ENV} to the Host package install root that contains system-image/)",
            system_image_directory.display(),
            service_root.display()
        )));
    }
    let vm_rootfs =
        materialize_windows_whpx_service_bootstrap(&service_root, &discovered.vm_rootfs)?;
    let endpoint = super::oci_whpx_owner::owned_pipe_name(&service_root)?;
    WindowsWhpxOciMigrationConfig::new(service_root.clone(), endpoint)?.with_box_owned_owner(
        service_root,
        discovered.runtime_path,
        discovered.shim_path,
        vm_rootfs,
        discovered.system_image_manifest,
    )
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn materialize_windows_whpx_service_bootstrap(
    service_root: &Path,
    packaged_seed: &Path,
) -> ExecutionManagerResult<PathBuf> {
    let bootstrap = service_root.join("bootstrap-vm-rootfs");
    if !bootstrap.exists() {
        std::fs::create_dir_all(&bootstrap).map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "failed to create Windows WHPX service bootstrap {}: {error}",
                bootstrap.display()
            ))
        })?;
        if packaged_seed.is_dir() {
            let seed = packaged_seed.canonicalize().ok();
            if seed.as_ref() != Some(&bootstrap) {
                if let Ok(entries) = std::fs::read_dir(packaged_seed) {
                    for entry in entries.flatten() {
                        let source = entry.path();
                        if source.is_dir() {
                            continue;
                        }
                        let destination = bootstrap.join(entry.file_name());
                        let _ = std::fs::copy(&source, &destination);
                    }
                }
            }
        }
    }
    let canonical = bootstrap.canonicalize().map_err(|error| {
        ExecutionManagerError::Unavailable(format!(
            "failed to resolve Windows WHPX service bootstrap {}: {error}",
            bootstrap.display()
        ))
    })?;
    if canonical == service_root || !canonical.starts_with(service_root) {
        return Err(ExecutionManagerError::InvalidRequest(format!(
            "Windows WHPX service bootstrap {} must be a strict descendant of service root {}",
            canonical.display(),
            service_root.display()
        )));
    }
    Ok(canonical)
}

#[derive(Default)]
struct WindowsWhpxEnvironmentInputs {
    mode: Option<OsString>,
    runtime_root: Option<OsString>,
    endpoint: Option<OsString>,
    box_owned: Option<OsString>,
    service_root: Option<OsString>,
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_vm_rootfs: Option<OsString>,
    service_manifest: Option<OsString>,
}

#[derive(Default)]
struct LinuxKvmEnvironmentInputs {
    mode: Option<OsString>,
    runtime_root: Option<OsString>,
    endpoint: Option<OsString>,
    box_owned: Option<OsString>,
    service_root: Option<OsString>,
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_manifest: Option<OsString>,
}

fn parse_linux_kvm_environment(
    inputs: LinuxKvmEnvironmentInputs,
    home_dir: &Path,
) -> ExecutionManagerResult<Option<LinuxKvmOciMigrationConfig>> {
    let LinuxKvmEnvironmentInputs {
        mode,
        runtime_root,
        endpoint,
        box_owned,
        service_root,
        service_bin,
        service_shim,
        service_manifest,
    } = inputs;
    let Some(mode) = mode.filter(|value| !value.is_empty()) else {
        // Gate 5: absent `K_BOX_OCI_MIGRATION` selects packaged DedicatedVm
        // when runtime/shim/system-image are discoverable. Missing packages
        // soft-fall through to MicroVM / legacy (Box-libkrun) instead of
        // hard-failing hosts without the KVM package.
        #[cfg(target_os = "linux")]
        {
            let _ = (endpoint, box_owned);
            let host_root_override = runtime_root
                .filter(|value| !value.is_empty())
                .map(PathBuf::from);
            return match parse_linux_kvm_packaged_box_owned(
                home_dir,
                host_root_override,
                service_root,
                service_bin,
                service_shim,
                service_manifest,
            ) {
                Ok(config) => Ok(Some(config)),
                Err(ExecutionManagerError::InvalidRequest(message))
                    if message.contains("was not found in packaged Kbox locations") =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            };
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (
                runtime_root,
                endpoint,
                box_owned,
                service_root,
                service_bin,
                service_shim,
                service_manifest,
            );
            return Ok(None);
        }
    };
    let mode = mode.to_str().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(format!(
            "{OCI_MIGRATION_ENV} must contain UTF-8 text"
        ))
    })?;
    match mode.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "disabled" | "legacy" | "1" | "true" | "on" | "sandbox"
        | "sandbox-via-oci" => return Ok(None),
        "all" | "all-via-oci" | "microvm" | "microvm-via-oci" | "kvm" => {}
        value => {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "unsupported {OCI_MIGRATION_ENV} value {value:?}; expected off, sandbox, microvm, or all"
            )))
        }
    }

    let host_root_override = runtime_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let box_owned = parse_explicit_bool(OCI_KVM_BOX_OWNED_ENV, box_owned)?;

    if box_owned {
        return parse_linux_kvm_explicit_box_owned(
            home_dir,
            host_root_override,
            endpoint,
            service_root,
            service_bin,
            service_shim,
            service_manifest,
        )
        .map(Some);
    }

    if let Some(endpoint) = endpoint.filter(|value| !value.is_empty()) {
        let runtime_root = host_root_override.unwrap_or_else(|| default_service_root(home_dir));
        return LinuxKvmOciMigrationConfig::new(runtime_root, PathBuf::from(endpoint)).map(Some);
    }

    // Opt-in microvm|all without qualification endpoint → packaged Box-owned Host.
    #[cfg(target_os = "linux")]
    {
        parse_linux_kvm_packaged_box_owned(
            home_dir,
            host_root_override,
            service_root,
            service_bin,
            service_shim,
            service_manifest,
        )
        .map(Some)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (service_root, service_bin, service_shim, service_manifest);
        Err(ExecutionManagerError::InvalidRequest(format!(
            "{OCI_KVM_ENDPOINT_ENV} must be set explicitly for the qualification-only KVM service"
        )))
    }
}

fn parse_linux_kvm_explicit_box_owned(
    _home_dir: &Path,
    host_root_override: Option<PathBuf>,
    endpoint: Option<OsString>,
    service_root: Option<OsString>,
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_manifest: Option<OsString>,
) -> ExecutionManagerResult<LinuxKvmOciMigrationConfig> {
    let service_root = service_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            ExecutionManagerError::InvalidRequest(format!(
                "{OCI_KVM_SERVICE_ROOT_ENV} must be set when {OCI_KVM_BOX_OWNED_ENV} is enabled"
            ))
        })?;
    let (service_bin, service_shim, service_manifest) =
        resolve_linux_kvm_owner_artifacts(service_bin, service_shim, service_manifest)?;
    let runtime_root = host_root_override.unwrap_or_else(|| service_root.join("runtime"));
    let endpoint = endpoint
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| service_root.join("runtime.sock"));
    LinuxKvmOciMigrationConfig::new(runtime_root, endpoint)?.with_box_owned_owner(
        service_root,
        service_bin,
        service_shim,
        service_manifest,
    )
}

#[cfg(target_os = "linux")]
fn parse_linux_kvm_packaged_box_owned(
    home_dir: &Path,
    host_root_override: Option<PathBuf>,
    service_root: Option<OsString>,
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_manifest: Option<OsString>,
) -> ExecutionManagerResult<LinuxKvmOciMigrationConfig> {
    let (service_bin, service_shim, service_manifest) =
        resolve_linux_kvm_owner_artifacts(service_bin, service_shim, service_manifest)?;
    let service_root = service_root
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_service_root(home_dir));
    let runtime_root = host_root_override.unwrap_or_else(|| service_root.join("runtime"));
    let endpoint = service_root.join("runtime.sock");
    LinuxKvmOciMigrationConfig::new(runtime_root, endpoint)?.with_box_owned_owner(
        service_root,
        service_bin,
        service_shim,
        service_manifest,
    )
}

fn resolve_linux_kvm_owner_artifacts(
    service_bin: Option<OsString>,
    service_shim: Option<OsString>,
    service_manifest: Option<OsString>,
) -> ExecutionManagerResult<(PathBuf, PathBuf, PathBuf)> {
    let override_bin = service_bin
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let override_shim = service_shim
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let override_manifest = service_manifest
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    #[cfg(target_os = "linux")]
    {
        match (override_bin, override_shim, override_manifest) {
            (Some(bin), Some(shim), Some(manifest)) => Ok((bin, shim, manifest)),
            (runtime_path, shim_path, system_image_manifest) => {
                let discovered = super::oci_kvm_packaged::discover_packaged_linux_kvm_artifacts(
                    super::oci_kvm_packaged::PackagedLinuxKvmOverrides {
                        runtime_path,
                        shim_path,
                        system_image_manifest,
                    },
                )?;
                Ok((
                    discovered.runtime_path,
                    discovered.shim_path,
                    discovered.system_image_manifest,
                ))
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let service_bin = override_bin.ok_or_else(|| {
            ExecutionManagerError::InvalidRequest(format!(
                "{OCI_KVM_SERVICE_BIN_ENV} must be set when {OCI_KVM_BOX_OWNED_ENV} is enabled"
            ))
        })?;
        let service_shim = override_shim.ok_or_else(|| {
            ExecutionManagerError::InvalidRequest(format!(
                "{OCI_KVM_SERVICE_SHIM_ENV} must be set when {OCI_KVM_BOX_OWNED_ENV} is enabled"
            ))
        })?;
        let service_manifest = override_manifest.ok_or_else(|| {
            ExecutionManagerError::InvalidRequest(format!(
                "{OCI_KVM_SERVICE_MANIFEST_ENV} must be set when {OCI_KVM_BOX_OWNED_ENV} is enabled"
            ))
        })?;
        Ok((service_bin, service_shim, service_manifest))
    }
}

fn default_service_root(home_dir: &Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt as _;

        // Keep runtime.sock below the small sockaddr_un limit while separating
        // different Kbox homes owned by the same UID.
        let digest = Sha256::digest(home_dir.as_os_str().as_bytes());
        // SAFETY: geteuid has no preconditions or failure result.
        let uid = unsafe { libc::geteuid() };
        std::env::temp_dir().join(format!("kbox-oci-{uid}-{}", hex::encode(&digest[..6])))
    }

    #[cfg(not(target_os = "linux"))]
    home_dir.join("run").join("oci-host")
}

pub(super) fn validate_absolute_normalized(path: &Path, label: &str) -> ExecutionManagerResult<()> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ExecutionManagerError::InvalidRequest(format!(
            "{label} must be an absolute normalized non-root path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn windows_environment_without_endpoint_fail_closed_or_accepts_qualification_pipe() {
        let home = absolute("koci-config-home");
        // Opt-in without endpoint uses packaged discovery; empty install fail-closes.
        assert!(parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: Some(OsString::from("all")),
                ..Default::default()
            },
            &home,
        )
        .is_err());

        let config = parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                runtime_root: Some(absolute("koci-whpx-root").into_os_string()),
                endpoint: Some(OsString::from(DEFAULT_OCI_WHPX_ENDPOINT)),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::windows_named_pipe(
                DEFAULT_OCI_WHPX_ENDPOINT,
            )
            .unwrap()
        );
        assert!(config.box_owned_owner().is_none());
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[test]
    fn windows_packaged_opt_in_builds_box_owned_without_endpoint() {
        use std::fs;

        let home = absolute("koci-whpx-packaged-home");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        let plant =
            std::env::temp_dir().join(format!("kwhpx-migration-packaged-{}", std::process::id()));
        let _ = fs::remove_dir_all(&plant);
        fs::create_dir_all(plant.join("bin")).unwrap();
        fs::create_dir_all(plant.join("system-image")).unwrap();
        fs::create_dir_all(plant.join("bootstrap-vm-rootfs")).unwrap();
        let runtime = plant.join("bin").join("koci.exe");
        let shim = plant.join("bin").join("koci-kshim.exe");
        let manifest = plant.join("system-image").join("system-image.json");
        let seed_bootstrap = plant.join("bootstrap-vm-rootfs");
        fs::write(&runtime, b"runtime").unwrap();
        fs::write(&shim, b"shim").unwrap();
        fs::write(&manifest, b"{}").unwrap();

        let config = parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                service_bin: Some(runtime.clone().into_os_string()),
                service_shim: Some(shim.clone().into_os_string()),
                service_vm_rootfs: Some(seed_bootstrap.into_os_string()),
                service_manifest: Some(manifest.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();

        let service_root = default_service_root(&home).canonicalize().unwrap();
        let expected_pipe =
            super::super::oci_whpx_owner::owned_pipe_name(&service_root).expect("derive pipe");
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::windows_named_pipe(expected_pipe).unwrap()
        );
        let owner = config
            .box_owned_owner()
            .expect("packaged opt-in is Box-owned");
        assert_eq!(owner.service_root(), service_root.as_path());
        assert_eq!(
            owner.runtime_path(),
            runtime.canonicalize().unwrap().as_path()
        );
        assert_eq!(owner.shim_path(), shim.canonicalize().unwrap().as_path());
        assert_eq!(
            owner.system_image_manifest(),
            manifest.canonicalize().unwrap().as_path()
        );
        assert_eq!(
            owner.vm_rootfs(),
            service_root.join("bootstrap-vm-rootfs").as_path()
        );
        let _ = fs::remove_dir_all(&plant);
        let _ = fs::remove_dir_all(&home);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[test]
    fn windows_whpx_absent_mode_soft_skips_when_packaged_artifacts_missing() {
        use std::fs;

        let home = absolute("koci-whpx-gate5-missing-home");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        // Isolate discovery from any tip-prove `K_HOME` still exported in the
        // developer shell (packaged search roots include `dirs_home()`).
        let previous = std::env::var_os("K_HOME");
        // SAFETY: unit test temporarily redirects packaged discovery roots.
        unsafe {
            std::env::set_var("K_HOME", &home);
        }
        let result = parse_windows_environment(WindowsWhpxEnvironmentInputs::default(), &home);
        match previous {
            // SAFETY: See the enclosing function's safety documentation.
            Some(value) => unsafe {
                std::env::set_var("K_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("K_HOME");
            },
        }
        assert_eq!(
            result.unwrap(),
            None,
            "gate 5 must soft-skip hosts without packaged WHPX artifacts"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[test]
    fn windows_whpx_absent_mode_uses_packaged_box_owned_when_artifacts_present() {
        use std::fs;

        let home = absolute("koci-whpx-gate5-default-home");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        let plant =
            std::env::temp_dir().join(format!("kwhpx-migration-gate5-{}", std::process::id()));
        let _ = fs::remove_dir_all(&plant);
        fs::create_dir_all(plant.join("bin")).unwrap();
        fs::create_dir_all(plant.join("system-image")).unwrap();
        fs::create_dir_all(plant.join("bootstrap-vm-rootfs")).unwrap();
        let runtime = plant.join("bin").join("koci.exe");
        let shim = plant.join("bin").join("koci-kshim.exe");
        let manifest = plant.join("system-image").join("system-image.json");
        let seed_bootstrap = plant.join("bootstrap-vm-rootfs");
        fs::write(&runtime, b"runtime").unwrap();
        fs::write(&shim, b"shim").unwrap();
        fs::write(&manifest, b"{}").unwrap();

        let config = parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: None,
                service_bin: Some(runtime.clone().into_os_string()),
                service_shim: Some(shim.clone().into_os_string()),
                service_vm_rootfs: Some(seed_bootstrap.into_os_string()),
                service_manifest: Some(manifest.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .expect("gate 5 packaged default");

        let service_root = default_service_root(&home).canonicalize().unwrap();
        let expected_pipe =
            super::super::oci_whpx_owner::owned_pipe_name(&service_root).expect("derive pipe");
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::windows_named_pipe(expected_pipe).unwrap()
        );
        let owner = config
            .box_owned_owner()
            .expect("gate 5 packaged path is Box-owned");
        assert_eq!(owner.service_root(), service_root.as_path());
        assert_eq!(
            owner.runtime_path(),
            runtime.canonicalize().unwrap().as_path()
        );
        assert_eq!(owner.shim_path(), shim.canonicalize().unwrap().as_path());
        assert_eq!(
            owner.system_image_manifest(),
            manifest.canonicalize().unwrap().as_path()
        );
        let _ = fs::remove_dir_all(&plant);
        let _ = fs::remove_dir_all(&home);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[test]
    fn windows_packaged_opt_in_rejects_service_root_overlapping_system_image() {
        use std::fs;

        let home = absolute("koci-whpx-packaged-overlap-home");
        let plant = std::env::temp_dir().join(format!(
            "kwhpx-migration-packaged-overlap-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&plant);
        fs::create_dir_all(plant.join("bin")).unwrap();
        fs::create_dir_all(plant.join("system-image")).unwrap();
        fs::create_dir_all(plant.join("bootstrap-vm-rootfs")).unwrap();
        let runtime = plant.join("bin").join("koci.exe");
        let shim = plant.join("bin").join("koci-kshim.exe");
        let manifest = plant.join("system-image").join("system-image.json");
        let bootstrap = plant.join("bootstrap-vm-rootfs");
        fs::write(&runtime, b"runtime").unwrap();
        fs::write(&shim, b"shim").unwrap();
        fs::write(&manifest, b"{}").unwrap();

        let error = parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                // Package install root overlaps immutable system-image/.
                service_root: Some(plant.clone().into_os_string()),
                service_bin: Some(runtime.into_os_string()),
                service_shim: Some(shim.into_os_string()),
                service_vm_rootfs: Some(bootstrap.into_os_string()),
                service_manifest: Some(manifest.into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .expect_err("service root overlapping system-image must fail closed");
        let message = error.to_string();
        assert!(
            message.contains("must be disjoint"),
            "unexpected error: {message}"
        );
        let _ = fs::remove_dir_all(&plant);
    }

    #[test]
    fn windows_box_owned_environment_derives_pipe_from_service_root() {
        let home = absolute("koci-config-home");
        let service_root = absolute("kwhpx-service");
        let expected =
            super::super::oci_whpx_owner::owned_pipe_name(&service_root).expect("derive pipe");
        let config = parse_windows_environment(
            WindowsWhpxEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                runtime_root: Some(absolute("koci-whpx-root").into_os_string()),
                box_owned: Some(OsString::from("1")),
                service_root: Some(service_root.clone().into_os_string()),
                service_bin: Some(absolute("koci.exe").into_os_string()),
                service_shim: Some(absolute("koci-kshim.exe").into_os_string()),
                service_vm_rootfs: Some(absolute("system").into_os_string()),
                service_manifest: Some(absolute("system-image.json").into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::windows_named_pipe(expected).unwrap()
        );
        let owner = config.box_owned_owner().expect("box-owned owner");
        assert_eq!(owner.service_root(), service_root.as_path());
        assert_eq!(
            owner.system_image_manifest(),
            absolute("system-image.json").as_path()
        );
    }

    #[test]
    fn linux_kvm_environment_requires_explicit_socket_and_accepts_microvm() {
        let home = absolute("koci-config-home");
        let endpoint = absolute("koci-kvm-box-runtime.sock");
        assert!(parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                ..Default::default()
            },
            &home
        )
        .is_err());
        assert_eq!(
            parse_linux_kvm_environment(
                LinuxKvmEnvironmentInputs {
                    mode: Some(OsString::from("sandbox")),
                    ..Default::default()
                },
                &home
            )
            .unwrap(),
            None
        );

        let config = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                runtime_root: Some(absolute("koci-kvm-runtime").into_os_string()),
                endpoint: Some(endpoint.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::unix_socket(&endpoint).unwrap()
        );
        assert_eq!(
            config.runtime_root(),
            absolute("koci-kvm-runtime").as_path()
        );
        assert!(config.box_owned_owner().is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_kvm_absent_mode_soft_skips_when_packaged_artifacts_missing() {
        let home = absolute("koci-kvm-gate5-missing-home");
        assert_eq!(
            parse_linux_kvm_environment(LinuxKvmEnvironmentInputs::default(), &home).unwrap(),
            None,
            "gate 5 must soft-skip hosts without packaged KVM artifacts"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_kvm_absent_mode_uses_packaged_box_owned_when_artifacts_present() {
        let home = absolute("koci-kvm-gate5-default-home");
        let service_root = absolute("koci-kvm-gate5-default-service");
        let runtime = absolute("koci-gate5-runtime-bin");
        let shim = absolute("koci-gate5-shim-bin");
        let manifest = absolute("koci-gate5-system-image.json");

        let config = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: None,
                service_root: Some(service_root.clone().into_os_string()),
                service_bin: Some(runtime.clone().into_os_string()),
                service_shim: Some(shim.clone().into_os_string()),
                service_manifest: Some(manifest.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .expect("gate 5 packaged default");

        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::unix_socket(
                service_root.join("runtime.sock")
            )
            .unwrap()
        );
        let owner = config
            .box_owned_owner()
            .expect("gate 5 packaged path is Box-owned");
        assert_eq!(owner.runtime_path(), runtime.as_path());
        assert_eq!(owner.shim_path(), shim.as_path());
        assert_eq!(owner.system_image_manifest(), manifest.as_path());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_kvm_packaged_opt_in_builds_box_owned_without_endpoint() {
        let home = absolute("koci-kvm-packaged-home");
        let service_root = absolute("koci-kvm-packaged-service");
        let runtime = absolute("koci-packaged-runtime-bin");
        let shim = absolute("koci-packaged-shim-bin");
        let manifest = absolute("koci-packaged-system-image.json");

        let config = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                service_root: Some(service_root.clone().into_os_string()),
                service_bin: Some(runtime.clone().into_os_string()),
                service_shim: Some(shim.clone().into_os_string()),
                service_manifest: Some(manifest.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::unix_socket(
                service_root.join("runtime.sock")
            )
            .unwrap()
        );
        assert_eq!(
            config.runtime_root(),
            service_root.join("runtime").as_path()
        );
        let owner = config
            .box_owned_owner()
            .expect("packaged path is Box-owned");
        assert_eq!(owner.service_root(), service_root.as_path());
        assert_eq!(owner.runtime_path(), runtime.as_path());
        assert_eq!(owner.shim_path(), shim.as_path());
        assert_eq!(owner.system_image_manifest(), manifest.as_path());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_kvm_packaged_opt_in_all_mode_matches_microvm() {
        let home = absolute("koci-kvm-packaged-all-home");
        let service_root = absolute("koci-kvm-packaged-all-service");
        let config = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("all")),
                service_root: Some(service_root.clone().into_os_string()),
                service_bin: Some(absolute("koci-all").into_os_string()),
                service_shim: Some(absolute("koci-kshim-all").into_os_string()),
                service_manifest: Some(absolute("system-image-all.json").into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();
        assert!(config.box_owned_owner().is_some());
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::unix_socket(
                service_root.join("runtime.sock")
            )
            .unwrap()
        );
    }

    #[test]
    fn linux_kvm_box_owned_requires_service_artifacts_and_fences_endpoint() {
        let home = absolute("koci-config-home");
        let service_root = absolute("koci-kvm-service");
        let endpoint = service_root.join("runtime.sock");
        assert!(parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                runtime_root: Some(absolute("koci-kvm-runtime").into_os_string()),
                endpoint: Some(endpoint.clone().into_os_string()),
                box_owned: Some(OsString::from("1")),
                service_root: Some(service_root.clone().into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .is_err());

        let config = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("kvm")),
                runtime_root: Some(absolute("koci-kvm-runtime").into_os_string()),
                box_owned: Some(OsString::from("true")),
                service_root: Some(service_root.clone().into_os_string()),
                service_bin: Some(absolute("koci").into_os_string()),
                service_shim: Some(absolute("koci-kvm-shim").into_os_string()),
                service_manifest: Some(absolute("system-image.json").into_os_string()),
                ..Default::default()
            },
            &home,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            config.endpoint(),
            &crate::local_execution::OciRuntimeEndpoint::unix_socket(&endpoint).unwrap()
        );
        let owner = config.box_owned_owner().expect("box-owned owner");
        assert_eq!(owner.service_root(), service_root.as_path());
        assert_eq!(owner.runtime_path(), absolute("koci").as_path());
        assert_eq!(owner.shim_path(), absolute("koci-kvm-shim").as_path());
        assert_eq!(
            owner.system_image_manifest(),
            absolute("system-image.json").as_path()
        );
    }

    #[test]
    fn kvm_box_owned_flag_rejects_unknown_values() {
        let home = absolute("koci-kvm-flag-home");
        let error = parse_linux_kvm_environment(
            LinuxKvmEnvironmentInputs {
                mode: Some(OsString::from("microvm")),
                endpoint: Some(absolute("runtime.sock").into_os_string()),
                box_owned: Some(OsString::from("maybe")),
                ..Default::default()
            },
            &home,
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains(OCI_KVM_BOX_OWNED_ENV),
            "unexpected error: {message}"
        );
        assert!(message.contains("maybe"), "unexpected error: {message}");
    }
}
