//! Production Box-owned bundle preparation for MicroVM DedicatedVm OCI services.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use kbox_core::config::{ResourceConfig, TeeConfig};
use kbox_core::{
    BoxError, ExecutionBackend, ExecutionManagerError, ExecutionManagerResult, NetworkMode,
};
use koci_sdk::{CreateAttachments, IoMode, IsolationRequest, OciBundle, ProcessIo};

use super::{
    OciBundlePreparationContext, OciBundleProvider, OciPreparedExecution, VmLocalExecutionBackend,
};
use crate::BoxRecord;

/// Qualification-only producer for OCI Runtime's Windows dedicated-VM service.
#[derive(Clone)]
pub struct WindowsWhpxOciBundleProvider {
    inner: DedicatedVmOciBundleProvider,
}

/// Qualification-only producer for OCI Runtime's Linux KVM dedicated-VM service.
#[derive(Clone)]
pub struct LinuxKvmOciBundleProvider {
    inner: DedicatedVmOciBundleProvider,
}

/// Box bundle producer for the public macOS DedicatedVm Host Service.
#[derive(Clone)]
pub struct MacosHvfOciBundleProvider {
    inner: DedicatedVmOciBundleProvider,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DedicatedVmOciProfile {
    MacosHvf,
    WindowsWhpx,
    LinuxKvm,
}

impl DedicatedVmOciProfile {
    const fn label(self) -> &'static str {
        match self {
            Self::MacosHvf => "HVF",
            Self::WindowsWhpx => "WHPX",
            Self::LinuxKvm => "KVM",
        }
    }

    const fn host_supported(self) -> bool {
        match self {
            Self::MacosHvf => cfg!(all(target_os = "macos", target_arch = "aarch64")),
            Self::WindowsWhpx => cfg!(all(target_os = "windows", target_arch = "x86_64")),
            Self::LinuxKvm => {
                cfg!(all(
                    target_os = "linux",
                    any(target_arch = "x86_64", target_arch = "aarch64")
                ))
            }
        }
    }

    fn host_requirement(self) -> &'static str {
        match self {
            Self::MacosHvf => "macOS arm64",
            Self::WindowsWhpx => "Windows x86_64",
            Self::LinuxKvm => "Linux x86_64 or aarch64",
        }
    }
}

#[derive(Clone)]
struct DedicatedVmOciBundleProvider {
    preparer: VmLocalExecutionBackend,
    runtime_root: PathBuf,
    profile: DedicatedVmOciProfile,
}

impl WindowsWhpxOciBundleProvider {
    pub fn new(home_dir: impl Into<PathBuf>, runtime_root: impl Into<PathBuf>) -> Self {
        Self {
            inner: DedicatedVmOciBundleProvider::new(
                home_dir,
                runtime_root,
                DedicatedVmOciProfile::WindowsWhpx,
            ),
        }
    }

    pub fn runtime_root(&self) -> &Path {
        self.inner.runtime_root()
    }

    pub fn with_pull_progress_fn(mut self, pull_progress_fn: crate::PullProgressFn) -> Self {
        self.inner = self.inner.with_pull_progress_fn(pull_progress_fn);
        self
    }
}

impl LinuxKvmOciBundleProvider {
    pub fn new(home_dir: impl Into<PathBuf>, runtime_root: impl Into<PathBuf>) -> Self {
        Self {
            inner: DedicatedVmOciBundleProvider::new(
                home_dir,
                runtime_root,
                DedicatedVmOciProfile::LinuxKvm,
            ),
        }
    }

    pub fn runtime_root(&self) -> &Path {
        self.inner.runtime_root()
    }

    pub fn with_pull_progress_fn(mut self, pull_progress_fn: crate::PullProgressFn) -> Self {
        self.inner = self.inner.with_pull_progress_fn(pull_progress_fn);
        self
    }
}

impl MacosHvfOciBundleProvider {
    pub fn new(home_dir: impl Into<PathBuf>, runtime_root: impl Into<PathBuf>) -> Self {
        Self {
            inner: DedicatedVmOciBundleProvider::new(
                home_dir,
                runtime_root,
                DedicatedVmOciProfile::MacosHvf,
            ),
        }
    }

    pub fn runtime_root(&self) -> &Path {
        self.inner.runtime_root()
    }

    pub fn with_pull_progress_fn(mut self, pull_progress_fn: crate::PullProgressFn) -> Self {
        self.inner = self.inner.with_pull_progress_fn(pull_progress_fn);
        self
    }
}

impl DedicatedVmOciBundleProvider {
    fn new(
        home_dir: impl Into<PathBuf>,
        runtime_root: impl Into<PathBuf>,
        profile: DedicatedVmOciProfile,
    ) -> Self {
        Self {
            preparer: VmLocalExecutionBackend::new(home_dir),
            runtime_root: runtime_root.into(),
            profile,
        }
    }

    fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    fn with_pull_progress_fn(mut self, pull_progress_fn: crate::PullProgressFn) -> Self {
        self.preparer = self.preparer.with_pull_progress_fn(pull_progress_fn);
        self
    }
}

#[async_trait]
impl OciBundleProvider for WindowsWhpxOciBundleProvider {
    fn preflight(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<()> {
        self.inner.preflight(record, context)
    }

    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
        _capabilities: &koci_sdk::AttachmentCapabilities,
    ) -> ExecutionManagerResult<super::LocalExecutionResourcePlan> {
        self.inner
            .plan_create_resources(record, _capabilities)
            .await
    }

    async fn prepare(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<OciPreparedExecution> {
        self.inner.prepare(record, context).await
    }

    async fn cleanup(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.inner.cleanup(record).await
    }

    async fn ensure_log_projection(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner.ensure_log_projection(record, binding).await
    }

    async fn wait_log_projection_drained(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_drained(record, binding)
            .await
    }

    async fn wait_log_projection_stopped_after_owner_loss(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_stopped_after_owner_loss(record, binding)
            .await
    }
}

#[async_trait]
impl OciBundleProvider for LinuxKvmOciBundleProvider {
    fn preflight(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<()> {
        self.inner.preflight(record, context)
    }

    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
        _capabilities: &koci_sdk::AttachmentCapabilities,
    ) -> ExecutionManagerResult<super::LocalExecutionResourcePlan> {
        self.inner
            .plan_create_resources(record, _capabilities)
            .await
    }

    async fn prepare(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<OciPreparedExecution> {
        self.inner.prepare(record, context).await
    }

    async fn cleanup(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.inner.cleanup(record).await
    }

    async fn ensure_log_projection(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner.ensure_log_projection(record, binding).await
    }

    async fn wait_log_projection_drained(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_drained(record, binding)
            .await
    }

    async fn wait_log_projection_stopped_after_owner_loss(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_stopped_after_owner_loss(record, binding)
            .await
    }
}

#[async_trait]
impl OciBundleProvider for MacosHvfOciBundleProvider {
    fn preflight(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<()> {
        self.inner.preflight(record, context)
    }

    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
        _capabilities: &koci_sdk::AttachmentCapabilities,
    ) -> ExecutionManagerResult<super::LocalExecutionResourcePlan> {
        self.inner
            .plan_create_resources(record, _capabilities)
            .await
    }

    async fn prepare(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<OciPreparedExecution> {
        self.inner.prepare(record, context).await
    }

    async fn cleanup(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        self.inner.cleanup(record).await
    }

    async fn ensure_log_projection(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner.ensure_log_projection(record, binding).await
    }

    async fn wait_log_projection_drained(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_drained(record, binding)
            .await
    }

    async fn wait_log_projection_stopped_after_owner_loss(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        self.inner
            .wait_log_projection_stopped_after_owner_loss(record, binding)
            .await
    }
}

#[async_trait]
impl OciBundleProvider for DedicatedVmOciBundleProvider {
    fn preflight(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<()> {
        if !self.profile.host_supported() {
            return Err(ExecutionManagerError::Unavailable(format!(
                "Box/{} OCI qualification requires {}",
                self.profile.label(),
                self.profile.host_requirement()
            )));
        }
        if !matches!(context.isolation(), IsolationRequest::DedicatedVm) {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "Box/{} OCI qualification requires dedicated MicroVM isolation",
                self.profile.label()
            )));
        }
        let metadata = record.managed_execution.as_ref().ok_or_else(|| {
            ExecutionManagerError::Internal(format!(
                "execution {} has no managed lifecycle metadata",
                record.id
            ))
        })?;
        metadata
            .validate()
            .map_err(|error| ExecutionManagerError::Internal(error.to_string()))?;
        if metadata.plan.backend != ExecutionBackend::Kbox {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "execution {} did not resolve to the MicroVM backend",
                record.id
            )));
        }
        validate_dedicated_vm_qualification(record, self.profile)?;
        if self.profile == DedicatedVmOciProfile::MacosHvf {
            super::oci_storage_attachments::validate_box_named_volume_sources(
                self.preparer.home_dir(),
                record,
            )?;
            if !record.anonymous_volumes.is_empty()
                && !context.attachment_capabilities().supports_extension(
                    koci_sdk::RAW_STORAGE_IMAGES_EXTENSION,
                    koci_sdk::RAW_STORAGE_IMAGES_VERSION,
                )
            {
                return Err(unqualified(
                    self.profile,
                    "Host lacks required raw ext4 storage transport",
                ));
            }
        }
        if self.profile == DedicatedVmOciProfile::MacosHvf
            && !context.attachment_capabilities().supports_extension(
                koci_sdk::ROOTFS_IMAGE_EXTENSION,
                if metadata.request.config.read_only {
                    koci_sdk::ROOTFS_IMAGE_READONLY_VERSION
                } else {
                    koci_sdk::ROOTFS_IMAGE_VERSION
                },
            )
        {
            return Err(ExecutionManagerError::InvalidRequest("HVF Host Service does not support the required guest-native rootfs image transport".into()));
        }
        if self.profile == DedicatedVmOciProfile::MacosHvf
            && (!metadata.caller_files.is_empty()
                || metadata.request.config.volumes.iter().any(|v| {
                    crate::vm::VmManager::parse_volume_spec(v).is_ok_and(|v| v.host_path.is_file())
                }))
            && !context.attachment_capabilities().supports_extension(
                koci_sdk::FILE_GRANTS_EXTENSION,
                koci_sdk::FILE_GRANTS_VERSION,
            )
        {
            return Err(ExecutionManagerError::InvalidRequest(
                "HVF Host Service does not support required exact file grants".into(),
            ));
        }
        if self.profile == DedicatedVmOciProfile::MacosHvf
            && metadata.request.policy.managed_secret_root.is_some()
            && !context.attachment_capabilities().supports_extension(
                koci_sdk::MEMORY_SECRET_FILES_EXTENSION,
                if !record.anonymous_volumes.is_empty() {
                    koci_sdk::MEMORY_SECRET_RAW_STORAGE_VERSION
                } else if metadata
                    .request
                    .config
                    .extra_env
                    .iter()
                    .any(|(key, _)| key == kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST)
                {
                    koci_sdk::MEMORY_SECRET_ENVIRONMENT_VERSION
                } else {
                    koci_sdk::MEMORY_SECRET_FILES_VERSION
                },
            )
        {
            return Err(unqualified(
                self.profile,
                "Host lacks required memory Secret file transport",
            ));
        }
        if self.profile == DedicatedVmOciProfile::MacosHvf
            && kbox_core::guest_exec::GuestRlimit::parse_specs(
                &metadata.request.config.resource_limits.ulimits,
            )
            .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?
            .iter()
            .any(|limit| limit.resource() == kbox_core::guest_exec::GuestRlimitResource::Nproc)
            && !context.attachment_capabilities().supports_extension(
                koci_sdk::NPROC_ADMISSION_EXTENSION,
                koci_sdk::NPROC_ADMISSION_VERSION,
            )
        {
            return Err(unqualified(
                self.profile,
                "Host lacks required kernel-counted nproc admission",
            ));
        }
        context.runtime_bundle_handoff_directory(&self.runtime_root)?;
        validate_runtime_root(&self.runtime_root, self.profile)
    }

    async fn plan_create_resources(
        &self,
        record: &BoxRecord,
        _capabilities: &koci_sdk::AttachmentCapabilities,
    ) -> ExecutionManagerResult<super::LocalExecutionResourcePlan> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if self.profile == DedicatedVmOciProfile::MacosHvf {
            let mut manager = self.preparer.new_oci_preparation_manager(record)?;
            let anonymous_volumes =
                manager
                    .plan_image_anonymous_volumes()
                    .await
                    .map_err(|error| {
                        dedicated_vm_preparation_error(self.profile, "plan image volumes", error)
                    })?;
            if record
                .managed_execution
                .as_ref()
                .is_some_and(|metadata| metadata.request.policy.managed_secret_root.is_some())
                && !anonymous_volumes.is_empty()
                && (!_capabilities.supports_extension(
                    koci_sdk::MEMORY_SECRET_FILES_EXTENSION,
                    koci_sdk::MEMORY_SECRET_RAW_STORAGE_VERSION,
                ) || !_capabilities.supports_extension(
                    koci_sdk::RAW_STORAGE_IMAGES_EXTENSION,
                    koci_sdk::RAW_STORAGE_IMAGES_VERSION,
                ))
            {
                return Err(unqualified(
                    self.profile,
                    "Host lacks required combined memory Secret/raw storage transport",
                ));
            }
            if anonymous_volumes.len()
                + record
                    .managed_execution
                    .as_ref()
                    .map_or(0, |metadata| metadata.request.config.volumes.len())
                + 2
                > 128
            {
                return Err(unqualified(
                    self.profile,
                    "v7 storage inventory exceeds 128 including workspace and rootfs",
                ));
            }
            return Ok(super::LocalExecutionResourcePlan {
                anonymous_volumes,
                caller_files: super::oci_file_grants::capture(record).map_err(|error| {
                    dedicated_vm_preparation_error(self.profile, "plan caller files", error)
                })?,
            });
        }
        let _ = record;
        Ok(Default::default())
    }

    async fn prepare(
        &self,
        record: &BoxRecord,
        context: &OciBundlePreparationContext,
    ) -> ExecutionManagerResult<OciPreparedExecution> {
        self.preflight(record, context)?;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if self.profile == DedicatedVmOciProfile::MacosHvf {
            super::oci_file_grants::validate_bound(record).map_err(|error| {
                dedicated_vm_preparation_error(self.profile, "validate caller file identity", error)
            })?;
        }
        let metadata = record.managed_execution.as_ref().ok_or_else(|| {
            ExecutionManagerError::Internal(format!(
                "execution {} has no managed lifecycle metadata",
                record.id
            ))
        })?;
        let bundle_directory = context.runtime_bundle_handoff_directory(&self.runtime_root)?;
        let mut manager = self.preparer.new_oci_preparation_manager(record)?;
        let preparation = async {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            if self.profile == DedicatedVmOciProfile::MacosHvf {
                return manager
                    .prepare_runtime_owned_ext4_bundle(
                        &metadata.plan,
                        &bundle_directory,
                        &metadata.caller_files,
                    )
                    .await;
            }
            manager
                .prepare_runtime_owned_microvm_bundle(&metadata.plan, &bundle_directory)
                .await
        };
        let prepared = preparation.await.map_err(|error| {
            dedicated_vm_preparation_error(self.profile, "prepare bundle", error)
        })?;
        let bundle = match OciBundle::load(&prepared.bundle_dir).await {
            Ok(bundle) => bundle,
            Err(error) => {
                return Err(cleanup_after_dedicated_vm_prepare_failure(
                    &manager,
                    &bundle_directory,
                    format!("failed to load the generated portable OCI bundle: {error}"),
                ));
            }
        };
        let io = ProcessIo {
            stdin: if metadata.request.config.stdin_open {
                IoMode::Pipe
            } else {
                IoMode::Null
            },
            stdout: IoMode::Capture,
            stderr: IoMode::Capture,
            terminal_size: None,
        };
        let attachments = match CreateAttachments::from_bundle(&bundle, io) {
            Ok(attachments) => attachments,
            Err(error) => {
                return Err(cleanup_after_dedicated_vm_prepare_failure(
                    &manager,
                    &bundle_directory,
                    format!("failed to derive portable OCI bundle attachments: {error}"),
                ));
            }
        };
        let attach = (|| -> ExecutionManagerResult<CreateAttachments> {
            Ok(if self.profile == DedicatedVmOciProfile::MacosHvf {
                let mut attachments = attachments
                    .with_rootfs_image(&bundle)
                    .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
                if !record.volume_names.is_empty() || !prepared.anonymous_volumes.is_empty() {
                    attachments = super::oci_storage_attachments::attach_box_owned_volume_storage(
                        &bundle,
                        attachments,
                        self.preparer.home_dir(),
                        record,
                        &prepared.anonymous_volumes,
                    )?;
                }
                for (index, mount) in bundle
                    .spec()
                    .mounts()
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                {
                    if mount.typ().as_deref() == Some("bind")
                        && !attachments.storage().iter().any(|storage| {
                            storage.mount().json_pointer() == format!("/mounts/{index}")
                        })
                    {
                        let mode = if mount
                            .options()
                            .as_ref()
                            .is_some_and(|values| values.iter().any(|value| value == "ro"))
                        {
                            koci_sdk::StorageAccessMode::ReadOnly
                        } else {
                            koci_sdk::StorageAccessMode::ReadWrite
                        };
                        attachments = attachments
                            .attach_storage_mount(
                                &bundle,
                                index,
                                koci_sdk::StorageAttachmentId::new(format!(
                                    "kbox.hvf.mount.{index}"
                                ))
                                .map_err(|error| {
                                    ExecutionManagerError::InvalidRequest(error.to_string())
                                })?,
                                mode,
                                koci_sdk::StorageOwnership::Caller,
                                koci_sdk::StorageCleanup::DetachOnly,
                            )
                            .map_err(|error| {
                                ExecutionManagerError::InvalidRequest(error.to_string())
                            })?;
                    }
                }
                super::oci_storage_attachments::reject_unclassified_bind_mounts(
                    &bundle,
                    attachments,
                )?
            } else {
                attachments
            })
        })();
        let attachments = match attach {
            Ok(attachments) => attachments,
            Err(error) => {
                return Err(cleanup_after_dedicated_vm_prepare_failure(
                    &manager,
                    &bundle_directory,
                    format!("failed to bind caller storage: {error}"),
                ))
            }
        };
        let mut result = match OciPreparedExecution::with_attachments(
            bundle,
            attachments,
            prepared.console_output,
        ) {
            Ok(result) => result,
            Err(error) => {
                return Err(cleanup_after_dedicated_vm_prepare_failure(
                    &manager,
                    &bundle_directory,
                    format!("failed to validate portable OCI bundle attachments: {error}"),
                ));
            }
        };
        result = match result.with_runtime_bundle_handoff(context, &self.runtime_root) {
            Ok(result) => result,
            Err(error) => {
                return Err(cleanup_after_dedicated_vm_prepare_failure(
                    &manager,
                    &bundle_directory,
                    format!("failed to bind portable OCI bundle handoff: {error}"),
                ));
            }
        };
        result.anonymous_volumes = prepared.anonymous_volumes;
        Ok(result)
    }

    async fn cleanup(&self, record: &BoxRecord) -> ExecutionManagerResult<()> {
        let manager = self.preparer.new_oci_preparation_manager(record)?;
        manager
            .cleanup_runtime_owned_microvm_bundle()
            .map_err(|error| {
                dedicated_vm_preparation_error(self.profile, "cleanup bundle", error)
            })?;
        super::resources::cleanup_ephemeral_oci_anonymous(self.preparer.home_dir(), record).await
    }

    async fn ensure_log_projection(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        super::oci_log_projection::ensure(record, binding).await
    }

    async fn wait_log_projection_drained(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        super::oci_log_projection::wait_drained(record, binding).await
    }

    async fn wait_log_projection_stopped_after_owner_loss(
        &self,
        record: &BoxRecord,
        binding: &super::OciRuntimeBinding,
    ) -> ExecutionManagerResult<()> {
        super::oci_log_projection::wait_stopped_after_owner_loss(record, binding).await
    }
}

fn validate_dedicated_vm_qualification(
    record: &BoxRecord,
    profile: DedicatedVmOciProfile,
) -> ExecutionManagerResult<()> {
    let metadata = record.managed_execution.as_ref().ok_or_else(|| {
        ExecutionManagerError::Internal(format!(
            "execution {} has no managed lifecycle metadata",
            record.id
        ))
    })?;
    let config = &metadata.request.config;
    let defaults = ResourceConfig::default();
    let macos = profile == DedicatedVmOciProfile::MacosHvf;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if macos {
        let mut configured = config.clone();
        if let Some(bytes) = metadata.request.policy.shm_size {
            configured.tmpfs.push(format!("/dev/shm:size={bytes}"));
        }
        crate::vm::oci_hvf_spec::validate(&configured)
            .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
    }
    if config.resources.vcpus != 1 || config.resources.memory_mb != 512 {
        return Err(unqualified(
            profile,
            "the fixed profile requires exactly 1 vCPU and 512 MiB of memory",
        ));
    }
    if (!macos && config.resources.disk_mb != defaults.disk_mb)
        || config.resources.timeout != defaults.timeout
        || config.resources.ephemeral_storage_bytes.is_some()
    {
        return Err(unqualified(
            profile,
            "custom disk size or lifetime timeout is not qualified for this OCI profile",
        ));
    }
    if config.tee != TeeConfig::None {
        return Err(unqualified(
            profile,
            "TEE is not qualified for this OCI profile",
        ));
    }
    if (!macos && (!config.workspace.as_os_str().is_empty() || !config.volumes.is_empty()))
        || config.virtiofs_cache.is_some()
        || (!macos && !metadata.request.policy.volume_names.is_empty())
        || (!macos && metadata.request.policy.managed_secret_root.is_some())
    {
        return Err(unqualified(
            profile,
            "workspace, bind, named, and secret mounts are not qualified for this OCI profile",
        ));
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if macos {
        super::oci_secret_attachments::validate_hvf_managed_secret_files(record)?;
    }
    if !matches!(config.network, NetworkMode::None)
        || !matches!(record.network_mode, NetworkMode::None)
        || !config.port_map.is_empty()
        || !config.dns.is_empty()
        || !config.add_hosts.is_empty()
    {
        return Err(unqualified(
            profile,
            "this OCI profile requires network=none and no network customization",
        ));
    }
    if config.pool.enabled
        || config.pool.snapshot_fork
        || config.deferred_main
        || config.ksm
        || config.snapshot_mem_file.is_some()
        || config.snapshot_sock.is_some()
        || config.restore_from.is_some()
        || metadata.request.rootfs_snapshot_id.is_some()
    {
        return Err(unqualified(
            profile,
            "pool, deferred-main, KSM, and Snapshot modes are not qualified for this OCI profile",
        ));
    }
    if (!macos && !config.tmpfs.is_empty())
        || (!macos && config.resource_limits != Default::default())
        || (!macos && (!config.cap_add.is_empty() || !config.cap_drop.is_empty()))
        || !config.security_opt.is_empty()
        || !config.sysctls.is_empty()
        || config.privileged
        || (!macos && config.read_only)
        || config.sidecar.is_some()
    {
        return Err(unqualified(
            profile,
            "custom mounts, controls, privileges, and sidecars are not qualified for this OCI profile",
        ));
    }
    let policy = &metadata.request.policy;
    if policy.init
        || !policy.devices.is_empty()
        || policy.gpus.is_some()
        || (!macos && policy.shm_size.is_some())
        || policy.oom_kill_disable
        || policy.oom_score_adj.is_some()
    {
        return Err(unqualified(
            profile,
            "init, device, GPU, shared-memory, and OOM overrides are not qualified for this OCI profile",
        ));
    }
    if policy.platform.as_deref().is_some_and(|platform| {
        !platform_supported(profile, platform.trim().to_ascii_lowercase().as_str())
    }) {
        return Err(unqualified(
            profile,
            match profile {
                DedicatedVmOciProfile::MacosHvf => {
                    "this OCI profile supports only Linux arm64 images"
                }
                DedicatedVmOciProfile::WindowsWhpx => {
                    "this OCI profile supports only Linux amd64 images"
                }
                DedicatedVmOciProfile::LinuxKvm => {
                    "this OCI profile supports only Linux amd64 or arm64 images"
                }
            },
        ));
    }
    if record.cpus != 1 || record.memory_mb != 512 {
        return Err(ExecutionManagerError::Internal(format!(
            "Box record resources drifted from the fixed {} qualification profile",
            profile.label()
        )));
    }
    Ok(())
}

fn unqualified(profile: DedicatedVmOciProfile, message: &str) -> ExecutionManagerError {
    ExecutionManagerError::InvalidRequest(format!(
        "Box/{} OCI qualification rejected: {message}",
        profile.label()
    ))
}

fn platform_supported(profile: DedicatedVmOciProfile, platform: &str) -> bool {
    match profile {
        DedicatedVmOciProfile::MacosHvf => matches!(platform, "linux/arm64" | "linux/aarch64"),
        DedicatedVmOciProfile::WindowsWhpx => {
            matches!(platform, "linux/amd64" | "linux/x86_64")
        }
        DedicatedVmOciProfile::LinuxKvm => matches!(
            platform,
            "linux/amd64" | "linux/x86_64" | "linux/arm64" | "linux/aarch64"
        ),
    }
}

fn validate_runtime_root(
    path: &Path,
    profile: DedicatedVmOciProfile,
) -> ExecutionManagerResult<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        ExecutionManagerError::Unavailable(format!(
            "failed to inspect {} runtime root {}: {error}",
            profile.label(),
            path.display()
        ))
    })?;
    if !metadata.is_dir() || metadata_is_reparse_point(&metadata) {
        return Err(ExecutionManagerError::InvalidRequest(format!(
            "{} runtime root is not a plain directory: {}",
            profile.label(),
            path.display()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn dedicated_vm_preparation_error(
    profile: DedicatedVmOciProfile,
    action: &str,
    error: BoxError,
) -> ExecutionManagerError {
    match error {
        BoxError::ConfigError(message) => ExecutionManagerError::InvalidRequest(message),
        error => ExecutionManagerError::Unavailable(format!(
            "Box/{} OCI {action} failed: {error}",
            profile.label()
        )),
    }
}

fn cleanup_after_dedicated_vm_prepare_failure(
    manager: &crate::VmManager,
    bundle_directory: &Path,
    message: String,
) -> ExecutionManagerError {
    let bundle_cleanup = remove_scoped_bundle(bundle_directory);
    let rootfs_cleanup = manager.cleanup_runtime_owned_microvm_bundle();
    match (bundle_cleanup, rootfs_cleanup) {
        (Ok(()), Ok(())) => ExecutionManagerError::Internal(message),
        (bundle, rootfs) => ExecutionManagerError::Internal(format!(
            "{message}; cleanup also failed: handoff={bundle:?}, rootfs={rootfs:?}"
        )),
    }
}

fn remove_scoped_bundle(bundle_directory: &Path) -> std::io::Result<()> {
    if bundle_directory.file_name().and_then(|name| name.to_str()) != Some("bundle")
        || bundle_directory.parent().and_then(Path::parent).is_none()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to remove an unscoped bundle: {}",
                bundle_directory.display()
            ),
        ));
    }
    let metadata = match std::fs::symlink_metadata(bundle_directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.is_dir() || metadata_is_reparse_point(&metadata) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "bundle is not a plain directory: {}",
                bundle_directory.display()
            ),
        ));
    }
    std::fs::remove_dir_all(bundle_directory)
}

impl std::fmt::Debug for MacosHvfOciBundleProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacosHvfOciBundleProvider")
            .field("runtime_root", &self.inner.runtime_root)
            .finish_non_exhaustive()
    }
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_named_tests.rs"]
mod named_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_anonymous_tests.rs"]
mod anonymous_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_tmpfs_tests.rs"]
mod tmpfs_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_shm_tests.rs"]
mod shm_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_readonly_tests.rs"]
mod readonly_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_ulimit_tests.rs"]
mod ulimit_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_secret_tests.rs"]
mod secret_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_disk_tests.rs"]
mod disk_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_production_capability_tests.rs"]
mod capability_tests;
