use super::*;

impl AgentVmAttachmentManifest {
    /// Bind raw ext4 storage together with optional directories, files and workload root.
    #[expect(
        clippy::too_many_arguments,
        reason = "one exact existing transport inventory per attachment kind"
    )]
    pub fn new_with_raw_storage(
        target: ContainerTarget,
        guest_bundle: GuestPath,
        bundle: &OciBundle,
        attachment_digest: impl Into<String>,
        storage: Vec<AgentVmStorageAttachment>,
        directories: Vec<AgentVmDirectoryAttachment>,
        files: Vec<AgentVmFileAttachment>,
        rootfs_image: Option<AgentVmRootfsImage>,
    ) -> Result<Self> {
        let manifest = Self {
            schema_version: memory_manifest_schema(bundle)?
                .unwrap_or(AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION)
                .into(),
            nproc_transport_schema: None,
            target,
            guest_bundle,
            config_digest: bundle.config_digest().into(),
            attachment_digest: attachment_digest.into(),
            network: Vec::new(),
            storage,
            directories,
            files,
            rootfs_image,
            storage_path_restrictions_digest: StoragePathRestrictions::from_bundle(bundle)?
                .as_ref()
                .map(StoragePathRestrictions::digest)
                .transpose()?,
        };
        manifest.with_nproc_admission(bundle)
    }
}

#[cfg(test)]
#[path = "raw_storage_tests.rs"]
mod tests;
