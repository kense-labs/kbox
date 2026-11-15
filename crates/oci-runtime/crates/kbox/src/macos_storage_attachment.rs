use super::*;
use koci_agent_protocol::{AgentVmStorageAttachment, AgentVmStorageSourceIdentity};
use koci_sdk::StorageAccessMode;

pub(crate) struct MacosStorageAttachment {
    grant: AgentVmStorageAttachment,
    disk: PinnedFile,
}

impl MacosStorageAttachment {
    pub(super) fn open(grant: AgentVmStorageAttachment) -> Result<Self> {
        if !(16 * 1024 * 1024..=64 * 1024 * 1024 * 1024).contains(&grant.source_identity().size()) {
            return Err(attachment_error(
                "raw storage disk capacity is outside the supported ext4 range",
            ));
        }
        let disk = PinnedFile::open_storage_disk(
            Path::new(grant.host_source()),
            grant.source_identity().size(),
            grant.access_mode() == StorageAccessMode::ReadWrite,
        )?;
        let pinned = Self { grant, disk };
        pinned.reverify()?;
        if pinned.grant.access_mode() == StorageAccessMode::ReadOnly {
            crate::macos_readonly::validate_read_only_file_descriptors(&[pinned.disk.path()])?;
        }
        Ok(pinned)
    }

    pub(crate) fn grant(&self) -> &AgentVmStorageAttachment {
        &self.grant
    }
    pub(crate) fn disk(&self) -> &PinnedFile {
        &self.disk
    }

    pub(super) fn reverify(&self) -> Result<()> {
        let source = Path::new(self.grant.host_source());
        if self.disk.path() != source
            || source
                .canonicalize()
                .map_err(|error| attachment_error(error.to_string()))?
                != source
        {
            return Err(attachment_error(
                "raw storage source resolves through a Host path alias",
            ));
        }
        require_private_control(&self.disk)?;
        self.disk.reverify("VM entry")?;
        let metadata =
            fs::symlink_metadata(source).map_err(|error| attachment_error(error.to_string()))?;
        let actual = AgentVmStorageSourceIdentity::new(
            metadata.dev(),
            metadata.rdev(),
            metadata.ino(),
            metadata.len(),
        )?;
        if actual != self.grant.source_identity() {
            return Err(attachment_error(
                "raw storage source inode or capacity differs from its exact grant",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "macos_storage_attachment_tests.rs"]
mod tests;
