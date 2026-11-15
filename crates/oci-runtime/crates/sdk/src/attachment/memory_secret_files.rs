//! A required memory-backing constraint on existing exact read-only file grants.
use serde::{Deserialize, Serialize};

use super::{invalid_attachment, AttachmentSource, CreateAttachments, RuntimeExtensionAttachment};
use crate::{FileGrants, OciBundle, Result, StorageAccessMode};

mod environment;
pub use environment::MemorySecretEnvironmentBinding;

/// Requires actual memory-backed source and projection at the Host boundary.
pub const MEMORY_SECRET_FILES_EXTENSION: &str = "dev.kbox.memory-secret-files";
/// Initial bounded memory-only file mechanism; ordinary file v1 remains unchanged.
pub const MEMORY_SECRET_FILES_VERSION: u16 = 1;
/// Memory-only files plus exact existing environment bindings, consumed before launch.
pub const MEMORY_SECRET_ENVIRONMENT_VERSION: u16 = 2;
/// RAM files and optional protected environment bindings with required raw storage.
pub const MEMORY_SECRET_RAW_STORAGE_VERSION: u16 = 3;

/// Exact read-only file mounts containing already-authorized transient material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemorySecretFiles {
    schema_version: u16,
    mount_indices: Vec<usize>,
}
impl MemorySecretFiles {
    /// Canonicalize a nonempty bounded set; duplicate mounts refuse.
    pub fn new(mut mount_indices: Vec<usize>) -> Result<Self> {
        mount_indices.sort_unstable();
        let value = Self {
            schema_version: MEMORY_SECRET_FILES_VERSION,
            mount_indices,
        };
        value.validate()?;
        Ok(value)
    }
    /// Require the environment projection contract without changing files-only v1.
    pub fn new_for_environment(mount_indices: Vec<usize>) -> Result<Self> {
        let mut value = Self::new(mount_indices)?;
        value.schema_version = MEMORY_SECRET_ENVIRONMENT_VERSION;
        Ok(value)
    }
    /// Require the combined raw-storage transport without changing v1/v2.
    pub fn new_for_raw_storage(mount_indices: Vec<usize>) -> Result<Self> {
        let mut value = Self::new(mount_indices)?;
        value.schema_version = MEMORY_SECRET_RAW_STORAGE_VERSION;
        Ok(value)
    }
    /// Required capability version; version2 additionally binds environment projection.
    pub const fn version(&self) -> u16 {
        self.schema_version
    }
    /// Validated environment bindings, each addressing one exact memory RO grant.
    pub fn environment_bindings(
        &self,
        bundle: &OciBundle,
    ) -> Result<Vec<MemorySecretEnvironmentBinding>> {
        let environment = bundle
            .spec()
            .process()
            .as_ref()
            .and_then(|p| p.env().as_deref())
            .unwrap_or_default();
        let bindings = MemorySecretEnvironmentBinding::from_environment(environment)?;
        match (self.schema_version, bindings) {
            (MEMORY_SECRET_FILES_VERSION | MEMORY_SECRET_RAW_STORAGE_VERSION, None) => Ok(Vec::new()),
            (MEMORY_SECRET_ENVIRONMENT_VERSION | MEMORY_SECRET_RAW_STORAGE_VERSION, Some(bindings)) => {
                let mounts = bundle.spec().mounts().as_deref().unwrap_or_default();
                for binding in &bindings {
                    if !self.mount_indices.iter().any(|index| mounts.get(*index).is_some_and(|m| m.destination().to_str() == Some(binding.path()))) {
                        return Err(invalid_attachment("Secret environment path lacks its exact memory file grant"));
                    }
                }
                Ok(bindings)
            }
            _ => Err(invalid_attachment("Secret environment projection requires explicit memory-file version2/3 and a nonempty binding manifest")),
        }
    }
    /// Exact indices, each bound to an existing read-only FileGrant.
    pub fn mount_indices(&self) -> &[usize] {
        &self.mount_indices
    }
    /// Encode only classifications; no material, names or resolution credentials.
    pub fn annotation_value(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|e| invalid_attachment(e.to_string()))
    }
    fn validate(&self) -> Result<()> {
        if !matches!(
            self.schema_version,
            MEMORY_SECRET_FILES_VERSION
                | MEMORY_SECRET_ENVIRONMENT_VERSION
                | MEMORY_SECRET_RAW_STORAGE_VERSION
        ) || self.mount_indices.is_empty()
            || self.mount_indices.len() > 128
            || self.mount_indices.iter().any(|index| *index >= 4096)
            || self.mount_indices.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid_attachment(
                "memory Secret files require version1/2/3 and ordered unique bounded indices",
            ));
        }
        Ok(())
    }
    /// Decode and verify exact existing file authority without filesystem effects.
    pub fn from_bundle(bundle: &OciBundle) -> Result<Option<Self>> {
        let Some(encoded) = bundle
            .spec()
            .annotations()
            .as_ref()
            .and_then(|a| a.get(MEMORY_SECRET_FILES_EXTENSION))
        else {
            return Ok(None);
        };
        if encoded.len() > 16 * 1024 {
            return Err(invalid_attachment(
                "memory Secret files exceed their byte bound",
            ));
        }
        let value: Self = serde_json::from_str(encoded)
            .map_err(|e| invalid_attachment(format!("invalid memory Secret files: {e}")))?;
        value.validate()?;
        if crate::raw_storage_images_requested(bundle)?
            != (value.version() == MEMORY_SECRET_RAW_STORAGE_VERSION)
        {
            return Err(invalid_attachment(
                "memory Secret files with raw storage require exact version3; v1/v2 retain files-only transport",
            ));
        }
        let files = FileGrants::from_bundle(bundle)?
            .ok_or_else(|| invalid_attachment("memory Secret files require exact file grants"))?;
        for index in &value.mount_indices {
            if !files.entries().iter().any(|f| {
                f.mount_index() == *index && f.access_mode() == StorageAccessMode::ReadOnly
            }) {
                return Err(invalid_attachment(
                    "memory Secret file lacks its exact read-only file grant",
                ));
            }
        }
        value.environment_bindings(bundle)?;
        Ok(Some(value))
    }
}
impl CreateAttachments {
    pub(super) fn attach_memory_secret_files(&mut self, bundle: &OciBundle) -> Result<()> {
        if let Some(memory) = MemorySecretFiles::from_bundle(bundle)? {
            self.extensions.insert(
                MEMORY_SECRET_FILES_EXTENSION.into(),
                RuntimeExtensionAttachment::from_annotation(
                    &super::decode_configuration(bundle)?,
                    MEMORY_SECRET_FILES_EXTENSION,
                    memory.version(),
                    true,
                )?,
            );
            super::insert_unique_source(
                &mut self.secrets,
                AttachmentSource::extension(MEMORY_SECRET_FILES_EXTENSION),
                "secret",
            )?;
        }
        Ok(())
    }
    /// Revalidate required capability and classification together with the file bindings.
    pub fn memory_secret_files(&self, bundle: &OciBundle) -> Result<Option<MemorySecretFiles>> {
        let files = MemorySecretFiles::from_bundle(bundle)?;
        let extension = self.extensions.get(MEMORY_SECRET_FILES_EXTENSION);
        let classified = self.secrets.iter().any(|source| {
            matches!(source,
            AttachmentSource::RuntimeExtension {name} if name == MEMORY_SECRET_FILES_EXTENSION)
        });
        if files.is_some() != extension.is_some()
            || files.is_some() != classified
            || extension.is_some_and(|e| {
                !e.required() || Some(e.version()) != files.as_ref().map(MemorySecretFiles::version)
            })
        {
            return Err(invalid_attachment(
                "memory Secret files require matching required extension and Secret classification",
            ));
        }
        if files.is_some() {
            self.file_grants(bundle)?;
        }
        Ok(files)
    }
}
