use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    decode_configuration, invalid_attachment, CreateAttachments, StorageAccessMode, StorageCleanup,
    StorageOwnership,
};
use crate::{OciBundle, Result, StorageAttachmentId};

/// Required exact-inode ordinary-file transport, distinct from directory grants.
pub const FILE_GRANTS_EXTENSION: &str = "dev.kbox.file-grants";
/// Initial live file data grant protocol.
pub const FILE_GRANTS_VERSION: u16 = 1;
const MAX_FILES: usize = 128;
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Exact file incarnation; live contents and length are deliberately not identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileSourceIdentity {
    device: u64,
    inode: u64,
}
impl FileSourceIdentity {
    /// Bind a nonzero filesystem inode without authorizing its parent.
    pub fn new(device: u64, inode: u64) -> Result<Self> {
        if inode == 0 {
            return Err(invalid_attachment("file grant requires a nonzero inode"));
        }
        Ok(Self { device, inode })
    }
    /// Filesystem device observed by the caller and verified by Host and shim.
    pub const fn device(self) -> u64 {
        self.device
    }
    /// Source file inode, retained across restart of the owning execution.
    pub const fn inode(self) -> u64 {
        self.inode
    }
}

/// Caller-owned file data authority for one exact standard OCI bind mount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileGrant {
    mount_index: usize,
    identity: StorageAttachmentId,
    source_identity: FileSourceIdentity,
    access_mode: StorageAccessMode,
    ownership: StorageOwnership,
    cleanup: StorageCleanup,
}
impl FileGrant {
    /// Explicitly declare Caller / DetachOnly authority; RW allows data only.
    pub fn new(
        mount_index: usize,
        identity: StorageAttachmentId,
        source_identity: FileSourceIdentity,
        access_mode: StorageAccessMode,
    ) -> Result<Self> {
        FileSourceIdentity::new(source_identity.device(), source_identity.inode())?;
        if mount_index >= 4096 {
            return Err(invalid_attachment("file mount index exceeds its bound"));
        }
        Ok(Self {
            mount_index,
            identity,
            source_identity,
            access_mode,
            ownership: StorageOwnership::Caller,
            cleanup: StorageCleanup::DetachOnly,
        })
    }
    /// Exact standard OCI mount index.
    pub const fn mount_index(&self) -> usize {
        self.mount_index
    }
    /// Immutable caller storage allocation incarnation.
    pub const fn identity(&self) -> &StorageAttachmentId {
        &self.identity
    }
    /// Exact caller-authorized file object, not a replaceable path selection.
    pub const fn source_identity(&self) -> FileSourceIdentity {
        self.source_identity
    }
    /// Required read-only or data-write access.
    pub const fn access_mode(&self) -> StorageAccessMode {
        self.access_mode
    }
}

/// Bounded canonical live-file extension configured by the immutable OCI annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileGrants {
    schema_version: u16,
    entries: Vec<FileGrant>,
}
impl FileGrants {
    /// Canonicalize grants; duplicated mounts, allocations and inode aliases refuse.
    pub fn new(mut entries: Vec<FileGrant>) -> Result<Self> {
        entries.sort_by_key(FileGrant::mount_index);
        let grants = Self {
            schema_version: FILE_GRANTS_VERSION,
            entries,
        };
        grants.validate()?;
        Ok(grants)
    }
    /// Exact grants; no caller directory is implied by any entry.
    pub fn entries(&self) -> &[FileGrant] {
        &self.entries
    }
    /// Bounded OCI annotation value.
    pub fn annotation_value(&self) -> Result<String> {
        self.validate()?;
        let value =
            serde_json::to_string(self).map_err(|error| invalid_attachment(error.to_string()))?;
        if value.len() > MAX_BYTES {
            return Err(invalid_attachment("file grants exceed their byte limit"));
        }
        Ok(value)
    }
    /// Decode and bind to exact ordinary OCI mounts, with no filesystem effects.
    pub fn from_bundle(bundle: &OciBundle) -> Result<Option<Self>> {
        let Some(value) = bundle
            .spec()
            .annotations()
            .as_ref()
            .and_then(|a| a.get(FILE_GRANTS_EXTENSION))
        else {
            return Ok(None);
        };
        if value.len() > MAX_BYTES {
            return Err(invalid_attachment("file grants exceed their byte limit"));
        }
        let grants: Self = serde_json::from_str(value)
            .map_err(|error| invalid_attachment(format!("invalid file grants: {error}")))?;
        grants.validate()?;
        let config = decode_configuration(bundle)?;
        for grant in &grants.entries {
            let mount = config
                .pointer(&format!("/mounts/{}", grant.mount_index))
                .ok_or_else(|| invalid_attachment("file grant references an absent OCI mount"))?;
            let source = mount
                .get("source")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_attachment("file mount has no source"))?;
            let destination = mount
                .get("destination")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_attachment("file mount has no destination"))?;
            if source.len() > 4096
                || source.contains('\0')
                || !Path::new(source).is_absolute()
                || source
                    .split('/')
                    .skip(1)
                    .any(|c| c.is_empty() || c == "." || c == "..")
                || Path::new(source)
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
                || destination.len() > 4096
                || destination == "/"
                || !destination.starts_with('/')
                || destination.contains('\0')
                || destination
                    .split('/')
                    .skip(1)
                    .any(|c| c.is_empty() || c == "." || c == "..")
            {
                return Err(invalid_attachment("file grants require normalized absolute source and explicit guest file destination"));
            }
            if !matches!(
                mount.get("type").and_then(Value::as_str),
                Some("bind" | "none")
            ) || super::storage::mount_access_mode(&config, grant.mount_index)?
                != grant.access_mode
            {
                return Err(invalid_attachment(
                    "file grant type or access differs from its exact OCI bind",
                ));
            }
        }
        Ok(Some(grants))
    }
    fn validate(&self) -> Result<()> {
        if self.schema_version != FILE_GRANTS_VERSION
            || self.entries.is_empty()
            || self.entries.len() > MAX_FILES
        {
            return Err(invalid_attachment(
                "file grants require supported version and bounded nonempty entries",
            ));
        }
        let mut indexes = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut files = BTreeSet::new();
        let mut previous = None;
        for grant in &self.entries {
            FileSourceIdentity::new(grant.source_identity.device, grant.source_identity.inode)?;
            if grant.ownership != StorageOwnership::Caller
                || grant.cleanup != StorageCleanup::DetachOnly
                || grant.mount_index >= 4096
                || previous.is_some_and(|p| p >= grant.mount_index)
                || !indexes.insert(grant.mount_index)
                || !ids.insert(&grant.identity)
                || !files.insert(grant.source_identity)
            {
                return Err(invalid_attachment(
                    "file grants must be ordered unique mounts, allocations and file incarnations",
                ));
            }
            previous = Some(grant.mount_index);
        }
        Ok(())
    }
}

impl CreateAttachments {
    /// Decode the required file mechanism and require the exact Caller/DetachOnly mounts.
    pub fn file_grants(&self, bundle: &OciBundle) -> Result<Option<FileGrants>> {
        let grants = FileGrants::from_bundle(bundle)?;
        let extension = self.extensions.get(FILE_GRANTS_EXTENSION);
        if grants.is_some() != extension.is_some()
            || extension.is_some_and(|e| !e.required() || e.version() != FILE_GRANTS_VERSION)
        {
            return Err(invalid_attachment(
                "file grants and their supported required extension must be declared together",
            ));
        }
        if let Some(grants) = &grants {
            for grant in grants.entries() {
                let mount = self
                    .mounts
                    .get(grant.mount_index())
                    .ok_or_else(|| invalid_attachment("file grant mount absent"))?;
                if !self.storage.iter().any(|s| {
                    s.mount() == mount
                        && s.identity() == grant.identity()
                        && s.access_mode() == grant.access_mode()
                        && s.ownership() == StorageOwnership::Caller
                        && s.cleanup() == StorageCleanup::DetachOnly
                }) {
                    return Err(invalid_attachment(
                        "file grant lacks its exact caller-owned storage binding",
                    ));
                }
            }
        }
        Ok(grants)
    }
}

#[cfg(test)]
#[path = "file_grants_tests.rs"]
mod tests;
