use koci_sdk::{Result, RootfsImage};
use serde::{Deserialize, Serialize};

use super::{manifest_error, AgentVmStorageSourceIdentity};

/// Exact writable workload disk, distinct from immutable boot assets and caller storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentVmRootfsImage {
    image: RootfsImage,
    source_identity: AgentVmStorageSourceIdentity,
}
impl AgentVmRootfsImage {
    /// Bind the initial source content to this generation's unique disk inode.
    pub fn new(image: RootfsImage, source_identity: AgentVmStorageSourceIdentity) -> Result<Self> {
        let rootfs = Self {
            image,
            source_identity,
        };
        rootfs.validate()?;
        Ok(rootfs)
    }
    /// Immutable initial image descriptor.
    pub fn image(&self) -> &RootfsImage {
        &self.image
    }
    /// Host inode identity reproduced by the Guest's virtio-blk serial.
    pub fn source_identity(&self) -> AgentVmStorageSourceIdentity {
        self.source_identity
    }
    pub(super) fn validate(&self) -> Result<()> {
        self.image.validate()?;
        let id = self.source_identity;
        AgentVmStorageSourceIdentity::new(id.device(), id.raw_device(), id.inode(), id.size())?;
        if id.size() != self.image.size() {
            return Err(manifest_error(
                "rootfs disk size differs from immutable image descriptor",
            ));
        }
        Ok(())
    }
}

/// Workload disk is a private sibling of the exact-generation share, never guest-exported.
pub fn workload_rootfs_disk_path(share: &std::path::Path) -> std::path::PathBuf {
    share.with_extension("rootfs.ext4")
}
