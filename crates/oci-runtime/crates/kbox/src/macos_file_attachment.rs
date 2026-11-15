//! Descriptor-pinned live file data grants; the caller parent is metadata-only.
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use koci_agent_protocol::{AgentVmFileAttachment, AGENT_VM_PROJECTED_FILE_NAME};
use koci_sdk::Result;

use super::attachment_error;

pub(crate) struct MacosFileAttachment {
    grant: AgentVmFileAttachment,
    projection: PathBuf,
    directory: File,
    source: File,
}
impl MacosFileAttachment {
    pub(super) fn open(grant: AgentVmFileAttachment, share: &Path) -> Result<Self> {
        let projection = grant.projection_path(share);
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY)
            .open(&projection)
            .map_err(|e| attachment_error(e.to_string()))?;
        // Only an immutable read descriptor is retained. Native virtio-fs opens
        // the alias after Seatbelt installation and must pass its exact data grant.
        let source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW_ANY)
            .open(grant.host_source())
            .map_err(|e| attachment_error(e.to_string()))?;
        let file = Self {
            grant,
            projection,
            directory,
            source,
        };
        file.reverify()?;
        Ok(file)
    }
    pub(crate) fn grant(&self) -> &AgentVmFileAttachment {
        &self.grant
    }
    pub(crate) fn path(&self) -> &Path {
        &self.projection
    }
    pub(crate) fn source_path(&self) -> &Path {
        Path::new(self.grant.host_source())
    }
    pub(crate) fn alias_path(&self) -> PathBuf {
        self.projection.join(AGENT_VM_PROJECTED_FILE_NAME)
    }
    pub(crate) fn pinned_path(&self) -> PathBuf {
        PathBuf::from(format!("/dev/fd/{}", self.directory.as_raw_fd()))
    }

    pub(super) fn reverify(&self) -> Result<()> {
        for path in [self.path(), self.source_path()] {
            if path
                .canonicalize()
                .map_err(|e| attachment_error(e.to_string()))?
                .as_os_str()
                != path.as_os_str()
            {
                return Err(attachment_error(
                    "file grant acquired a Host pathname alias",
                ));
            }
        }
        // SAFETY: geteuid has no preconditions or failure return.
        let uid = unsafe { libc::geteuid() };
        let expected = self.grant.projection_identity();
        for meta in [fs::symlink_metadata(self.path()), self.directory.metadata()] {
            let meta = meta.map_err(|e| attachment_error(e.to_string()))?;
            if !meta.is_dir()
                || meta.file_type().is_symlink()
                || meta.uid() != uid
                || meta.mode() & 0o777 != 0o700
                || meta.dev() != expected.device()
                || meta.ino() != expected.inode()
            {
                return Err(attachment_error("private file projection identity changed"));
            }
        }
        let names = fs::read_dir(self.path())
            .map_err(|e| attachment_error(e.to_string()))?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| attachment_error(e.to_string()))?;
        if names != [std::ffi::OsString::from(AGENT_VM_PROJECTED_FILE_NAME)] {
            return Err(attachment_error(
                "private projection must contain exactly the authorized file",
            ));
        }
        let expected = self.grant.source_identity();
        for meta in [
            self.source.metadata(),
            fs::symlink_metadata(self.source_path()),
            fs::symlink_metadata(self.alias_path()),
        ] {
            let meta = meta.map_err(|e| attachment_error(e.to_string()))?;
            if !meta.is_file()
                || meta.file_type().is_symlink()
                || meta.uid() != uid
                || meta.nlink() != 2
                || meta.dev() != expected.device()
                || meta.ino() != expected.inode()
            {
                return Err(attachment_error(
                    "caller file identity, type, owner or alias count changed",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "macos_file_attachment_tests.rs"]
mod tests;
