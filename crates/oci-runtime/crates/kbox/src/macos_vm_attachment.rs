use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use koci_agent_protocol::{
    AgentVmAttachmentManifest, AgentVmDirectoryAttachment, AGENT_RUNTIME_SHARE_GUEST_ROOT,
    AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME, AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES,
};
use koci_sdk::{
    Error, ErrorCode, OciBundle, Result, StoragePathRestriction, StoragePathRestrictions,
    MAX_CONFIG_BYTES,
};

use crate::macos_assets::PinnedFile;
use crate::macos_runtime_share::MacosRuntimeShare;

#[path = "macos_file_attachment.rs"]
mod file;
pub(crate) use file::MacosFileAttachment;
#[path = "macos_storage_attachment.rs"]
mod storage;
pub(crate) use storage::MacosStorageAttachment;

pub(crate) struct MacosVmAttachments {
    storage_disks: Vec<MacosStorageAttachment>,
    files: Vec<MacosFileAttachment>,
    manifest: PinnedFile,
    configuration: PinnedFile,
    directories: Vec<MacosDirectoryAttachment>,
    path_restrictions: Vec<(PathBuf, StoragePathRestriction)>,
    restrictions_digest: Option<String>,
    rootfs_disk: Option<PinnedFile>,
}

impl MacosVmAttachments {
    pub(crate) fn open(share: &MacosRuntimeShare, expected: Option<&str>) -> Result<Option<Self>> {
        share.reverify()?;
        let path = share.path().join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
        let Some(expected) = expected else {
            return match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(attachment_error(
                    "unexpected VM attachment manifest without a digest",
                )),
            };
        };
        let manifest = PinnedFile::open_bounded(
            &path,
            "macOS VM attachment manifest",
            AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES as u64,
        )?;
        if manifest.path().parent() != Some(share.path())
            || format!("sha256:{}", manifest.sha256()) != expected
        {
            return Err(attachment_error(
                "VM attachment manifest path or digest differs from its Host binding",
            ));
        }
        require_private_control(&manifest)?;
        let decoded = AgentVmAttachmentManifest::from_bytes(
            &manifest.read_bounded(AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES as u64)?,
        )?;
        if (decoded.directories().is_empty()
            && decoded.files().is_empty()
            && decoded.rootfs_image().is_none()
            && decoded.storage().is_empty())
            || !decoded.network().is_empty()
            || decoded.guest_bundle().as_str() != format!("{AGENT_RUNTIME_SHARE_GUEST_ROOT}/bundle")
        {
            return Err(attachment_error(
                "macOS VM attachments require directories and the exact dedicated Guest bundle",
            ));
        }
        let bundle = share.path().join("bundle");
        let configuration = PinnedFile::open_bounded(
            &bundle.join("config.json"),
            "macOS VM attachment configuration",
            MAX_CONFIG_BYTES,
        )?;
        if configuration.path().parent() != Some(bundle.as_path()) {
            return Err(attachment_error(
                "VM attachment configuration escaped its exact bundle",
            ));
        }
        require_private_control(&configuration)?;
        let encoded = String::from_utf8(configuration.read_bounded(MAX_CONFIG_BYTES)?)
            .map_err(|error| attachment_error(error.to_string()))?;
        let bundle = OciBundle::from_json(&bundle, encoded)?;
        decoded.validate_bundle(&bundle)?;
        if !decoded.storage().is_empty() && !koci_sdk::raw_storage_images_requested(&bundle)? {
            return Err(attachment_error(
                "macOS raw storage requires the explicit v7 capability contract",
            ));
        }
        let restrictions = StoragePathRestrictions::from_bundle(&bundle)?;
        let restrictions_digest = restrictions
            .as_ref()
            .map(StoragePathRestrictions::digest)
            .transpose()?;
        let mut path_restrictions = Vec::new();
        if let Some(restrictions) = restrictions {
            for restriction in restrictions.entries() {
                let directory = decoded
                    .directories()
                    .iter()
                    .find(|directory| directory.attachment().identity() == restriction.identity())
                    .ok_or_else(|| attachment_error("path restrictions have no bound directory"))?;
                path_restrictions
                    .push((PathBuf::from(directory.host_source()), restriction.clone()));
            }
        }
        let mut directories = Vec::with_capacity(decoded.directories().len());
        for grant in decoded.directories() {
            directories.push(MacosDirectoryAttachment::open(grant.clone())?);
        }
        for (index, directory) in directories.iter().enumerate() {
            if overlaps(directory.path(), share.path())
                || directories[..index]
                    .iter()
                    .any(|other| overlaps(directory.path(), other.path()))
            {
                return Err(attachment_error(
                    "directory grants overlap the private runtime share or each other",
                ));
            }
        }
        let memory = koci_sdk::MemorySecretFiles::from_bundle(&bundle)?;
        let mut files = Vec::with_capacity(decoded.files().len());
        for grant in decoded.files() {
            let file = MacosFileAttachment::open(grant.clone(), share.path())?;
            if overlaps(file.source_path(), share.path())
                || overlaps(
                    file.source_path(),
                    &koci_agent_protocol::file_projection_root(share.path()),
                )
                || directories
                    .iter()
                    .any(|d| overlaps(file.source_path(), d.path()))
                || files.iter().any(|f: &MacosFileAttachment| {
                    f.grant().source_identity() == file.grant().source_identity()
                })
            {
                return Err(attachment_error(
                    "file grants overlap control paths, directories or another file",
                ));
            }
            let index = grant.mount_index()?;
            if memory
                .as_ref()
                .is_some_and(|value| value.mount_indices().contains(&index))
            {
                for path in [file.source_path(), file.path()] {
                    // Verify RAM before Seatbelt; existing pinned device/inode checks retain it after confinement.
                    require_memory_backing(path)?;
                }
            }
            files.push(file);
        }
        let rootfs_disk = if let Some(rootfs) = decoded.rootfs_image() {
            let disk = PinnedFile::open_workload_disk(
                &koci_agent_protocol::workload_rootfs_disk_path(share.path()),
                rootfs.image().size(),
            )?;
            require_private_control(&disk)?;
            disk.require(
                rootfs.image().size(),
                rootfs
                    .image()
                    .sha256()
                    .strip_prefix("sha256:")
                    .ok_or_else(|| attachment_error("rootfs digest has no algorithm"))?,
            )?;
            let metadata = fs::symlink_metadata(disk.path())
                .map_err(|error| attachment_error(error.to_string()))?;
            let id = koci_agent_protocol::AgentVmStorageSourceIdentity::new(
                metadata.dev(),
                metadata.rdev(),
                metadata.ino(),
                metadata.len(),
            )?;
            if id != rootfs.source_identity() {
                return Err(attachment_error(
                    "workload disk inode differs from the exact generation binding",
                ));
            }
            if directories
                .iter()
                .any(|directory| overlaps(directory.path(), disk.path()))
            {
                return Err(attachment_error(
                    "caller grant overlaps the private workload disk",
                ));
            }
            Some(disk)
        } else {
            None
        };
        let mut storage_disks: Vec<MacosStorageAttachment> = Vec::new();
        for grant in decoded.storage() {
            let pin = MacosStorageAttachment::open(grant.clone())?;
            let source = pin.disk().path();
            if overlaps(source, share.path())
                || directories
                    .iter()
                    .any(|directory| overlaps(source, directory.path()))
                || files
                    .iter()
                    .any(|file| overlaps(source, file.source_path()))
                || storage_disks
                    .iter()
                    .any(|disk| overlaps(source, disk.disk().path()))
                || rootfs_disk
                    .as_ref()
                    .is_some_and(|disk| overlaps(source, disk.path()))
            {
                return Err(attachment_error(
                    "raw storage overlaps control paths or another grant",
                ));
            }
            storage_disks.push(pin);
        }
        share.reverify()?;
        Ok(Some(Self {
            storage_disks,
            files,
            manifest,
            configuration,
            directories,
            path_restrictions,
            restrictions_digest,
            rootfs_disk,
        }))
    }

    pub(crate) fn files(&self) -> &[MacosFileAttachment] {
        &self.files
    }

    pub(crate) fn storage_disks(&self) -> &[MacosStorageAttachment] {
        &self.storage_disks
    }

    pub(crate) fn rootfs_disk(&self) -> Option<&PinnedFile> {
        self.rootfs_disk.as_ref()
    }

    pub(crate) fn directories(&self) -> &[MacosDirectoryAttachment] {
        &self.directories
    }

    pub(crate) fn path_restrictions(&self) -> &[(PathBuf, StoragePathRestriction)] {
        &self.path_restrictions
    }

    pub(crate) fn restrictions_digest(&self) -> Option<&str> {
        self.restrictions_digest.as_deref()
    }

    pub(crate) fn reverify(&self) -> Result<()> {
        self.manifest.reverify("VM entry")?;
        self.configuration.reverify("VM entry")?;
        if let Some(disk) = &self.rootfs_disk {
            disk.reverify("VM entry")?;
        }
        for file in &self.files {
            file.reverify()?;
        }
        for disk in &self.storage_disks {
            disk.reverify()?;
        }
        for directory in &self.directories {
            directory.reverify()?;
        }
        Ok(())
    }
}

pub(crate) struct MacosDirectoryAttachment {
    grant: AgentVmDirectoryAttachment,
    directory: File,
}

impl MacosDirectoryAttachment {
    fn open(grant: AgentVmDirectoryAttachment) -> Result<Self> {
        let path = Path::new(grant.host_source());
        let canonical = path
            .canonicalize()
            .map_err(|error| attachment_error(error.to_string()))?;
        if canonical != path {
            return Err(attachment_error(
                "directory grant resolves through a Host path alias",
            ));
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY)
            .open(path)
            .map_err(|error| {
                attachment_error(format!("failed to pin caller directory: {error}"))
            })?;
        let pinned = Self { grant, directory };
        pinned.reverify()?;
        Ok(pinned)
    }

    pub(crate) fn grant(&self) -> &AgentVmDirectoryAttachment {
        &self.grant
    }
    pub(crate) fn path(&self) -> &Path {
        Path::new(self.grant.host_source())
    }
    pub(crate) fn pinned_path(&self) -> PathBuf {
        PathBuf::from(format!("/dev/fd/{}", self.directory.as_raw_fd()))
    }

    fn reverify(&self) -> Result<()> {
        let path = fs::symlink_metadata(self.path())
            .map_err(|error| attachment_error(error.to_string()))?;
        let opened = self
            .directory
            .metadata()
            .map_err(|error| attachment_error(error.to_string()))?;
        if self
            .path()
            .canonicalize()
            .map_err(|error| attachment_error(error.to_string()))?
            != self.path()
        {
            return Err(attachment_error(
                "caller directory gained a Host path alias before VM entry",
            ));
        }
        let expected = self.grant.source_identity();
        // SAFETY: geteuid has no preconditions or failure return.
        let uid = unsafe { libc::geteuid() };
        for metadata in [path, opened] {
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || metadata.uid() != uid
                || metadata.dev() != expected.device()
                || metadata.ino() != expected.inode()
            {
                return Err(attachment_error(
                    "caller directory identity changed before VM entry",
                ));
            }
        }
        Ok(())
    }
}

fn require_private_control(file: &PinnedFile) -> Result<()> {
    let metadata =
        fs::symlink_metadata(file.path()).map_err(|error| attachment_error(error.to_string()))?;
    // SAFETY: geteuid has no preconditions or failure return.
    let uid = unsafe { libc::geteuid() };
    if metadata.uid() != uid || metadata.mode() & 0o777 != 0o600 || metadata.nlink() != 1 {
        return Err(attachment_error(
            "VM attachment control files must be private and unaliased",
        ));
    }
    Ok(())
}

fn overlaps(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}
fn attachment_error(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::FailedPrecondition, message).for_operation("pin-macos-vm-attachments")
}

#[cfg(test)]
#[path = "macos_vm_attachment_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "macos_workload_rootfs_tests.rs"]
mod workload_tests;

fn require_memory_backing(path: &Path) -> Result<()> {
    if !crate::macos_path_is_on_ram_disk(path)? {
        return Err(attachment_error(
            "memory Secret file source and projection must remain on actual ram:// storage",
        ));
    }
    Ok(())
}
