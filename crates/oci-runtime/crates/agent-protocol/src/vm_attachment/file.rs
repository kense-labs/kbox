use std::path::{Path, PathBuf};

use koci_sdk::{FileSourceIdentity, Result, StorageAttachment};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    manifest_error, validate_configuration_attachment, validate_digest,
    AgentVmDirectorySourceIdentity,
};

/// Runtime-owned projection mount; the original caller parent is never exported.
pub const AGENT_VM_FILES_GUEST_ROOT: &str = "/run/koci-runtime/bundle/.koci-files";
/// Bundle-relative namespace admitted only by the file transport.
pub const AGENT_VM_FILES_RELATIVE_ROOT: &str = ".koci-files";
/// Fixed alias name inside each one-file projection.
pub const AGENT_VM_PROJECTED_FILE_NAME: &str = "file";

/// Caller file object and the exact private projection pinned by the shim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentVmFileAttachment {
    attachment: StorageAttachment,
    host_source: String,
    source_identity: FileSourceIdentity,
    projection_identity: AgentVmDirectorySourceIdentity,
    tag: String,
}
impl AgentVmFileAttachment {
    /// Bind an existing caller storage grant to one live inode and private directory.
    pub fn new(
        attachment: StorageAttachment,
        host_source: impl Into<String>,
        source_identity: FileSourceIdentity,
        projection_identity: AgentVmDirectorySourceIdentity,
        configuration_digest: &str,
    ) -> Result<Self> {
        let tag = file_tag(configuration_digest, attachment.identity().as_str())?;
        let file = Self {
            attachment,
            host_source: host_source.into(),
            source_identity,
            projection_identity,
            tag,
        };
        file.validate_shape(configuration_digest)?;
        Ok(file)
    }
    /// Exact SDK Caller / DetachOnly grant.
    pub const fn attachment(&self) -> &StorageAttachment {
        &self.attachment
    }
    /// Original canonical caller file pathname, used only for identity verification.
    pub fn host_source(&self) -> &str {
        &self.host_source
    }
    /// File identity authorized by the owning Box reservation and SDK configuration.
    pub const fn source_identity(&self) -> FileSourceIdentity {
        self.source_identity
    }
    /// Private directory identity containing only the runtime's hardlink alias.
    pub const fn projection_identity(&self) -> AgentVmDirectorySourceIdentity {
        self.projection_identity
    }
    /// Exact virtio-fs tag, bound to the complete OCI configuration digest.
    pub fn tag(&self) -> &str {
        &self.tag
    }
    /// Runtime-owned host directory; it is disjoint from the guest runtime share.
    pub fn projection_path(&self, share: &Path) -> PathBuf {
        file_projection_root(share).join(&self.tag)
    }
    /// Protected Guest mountpoint for the projection.
    pub fn guest_mount_path(&self) -> String {
        format!("{AGENT_VM_FILES_GUEST_ROOT}/{}", self.tag)
    }
    /// Exact file source rewritten before OCI mount planning.
    pub fn guest_file_relative_path(&self) -> String {
        format!(
            "{AGENT_VM_FILES_RELATIVE_ROOT}/{}/{AGENT_VM_PROJECTED_FILE_NAME}",
            self.tag
        )
    }
    /// Mount index selected by the SDK storage descriptor.
    pub fn mount_index(&self) -> Result<usize> {
        super::storage::parse_mount_index(self.attachment.mount().json_pointer())
    }
    pub(super) fn validate_shape(&self, digest: &str) -> Result<()> {
        FileSourceIdentity::new(self.source_identity.device(), self.source_identity.inode())?;
        AgentVmDirectorySourceIdentity::new(
            self.projection_identity.device(),
            self.projection_identity.inode(),
        )?;
        let source = &self.host_source;
        if source.len() < 2
            || source.len() > 4096
            || !source.starts_with('/')
            || source.contains('\0')
            || source
                .split('/')
                .skip(1)
                .any(|c| c.is_empty() || c == "." || c == "..")
            || self.tag != file_tag(digest, self.attachment.identity().as_str())?
        {
            return Err(manifest_error(
                "file transport requires an exact canonical source and digest-bound tag",
            ));
        }
        self.mount_index()?;
        validate_digest(
            self.attachment.mount().value_digest(),
            "file mount value digest",
        )?;
        if self.attachment.ownership() != koci_sdk::StorageOwnership::Caller
            || self.attachment.cleanup() != koci_sdk::StorageCleanup::DetachOnly
        {
            return Err(manifest_error(
                "file transport requires Caller ownership and DetachOnly cleanup",
            ));
        }
        Ok(())
    }
    pub(super) fn validate_configuration(&self, config: &Value) -> Result<()> {
        validate_configuration_attachment(config, self.attachment.mount(), "caller file mount")?;
        let source = config
            .pointer(self.attachment.mount().json_pointer())
            .and_then(|m| m.get("source"))
            .and_then(Value::as_str);
        if source != Some(&self.host_source) {
            return Err(manifest_error(
                "file transport differs from configured source",
            ));
        }
        Ok(())
    }
}
/// Entire exact-generation projection subtree, owned by the existing bundle handoff.
pub fn file_projection_root(share: &Path) -> PathBuf {
    share.with_extension("files")
}
/// Deterministic tag used while preparing a file projection before its directory exists.
pub fn file_tag(digest: &str, identity: &str) -> Result<String> {
    validate_digest(digest, "file attachment digest")?;
    let mut hash = Sha256::new();
    hash.update(b"koci.agent-vm-file.v1\0");
    hash.update(digest.as_bytes());
    hash.update([0]);
    hash.update(identity.as_bytes());
    let encoded = format!("{:x}", hash.finalize());
    Ok(format!("kfile-{}", &encoded[..24]))
}
