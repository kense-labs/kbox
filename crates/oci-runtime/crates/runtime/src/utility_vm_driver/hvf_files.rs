//! Live caller-file projections, disjoint from the writable Guest runtime share.
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use koci_agent_protocol::{
    file_projection_root, file_tag, AgentVmAttachmentManifest, AgentVmDirectorySourceIdentity,
    AgentVmFileAttachment, AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME, AGENT_VM_PROJECTED_FILE_NAME,
};
use koci_sdk::{CreateAttachments, Error, ErrorCode, FileSourceIdentity, OciBundle, Result};

use super::layout::{is_private_directory, path_metadata};
use super::UtilityVmLaunchRequest;

pub(super) async fn prepare(
    request: &UtilityVmLaunchRequest<'_>,
) -> Result<Vec<AgentVmFileAttachment>> {
    let Some(grants) = request.attachment_contract.file_grants(request.bundle)? else {
        return Ok(Vec::new());
    };
    let share = request.runtime_share.to_path_buf();
    let bundle = request.bundle.clone();
    let contract = request.attachment_contract.clone();
    let manifest_path = share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    let manifest = if path_metadata(&manifest_path).await?.is_some() {
        let manifest = super::vm_attachment_transport::read_manifest(&manifest_path).await?;
        manifest.validate_bundle(&bundle)?;
        if manifest.target() != request.target {
            return Err(failure(
                "file projection manifest targets another generation",
            ));
        }
        Some(manifest)
    } else {
        None
    };
    let prepared = tokio::task::spawn_blocking(move || {
        prepare_blocking(&share, &bundle, &contract, &grants, manifest.as_ref())
    })
    .await
    .map_err(|e| failure(format!("file projection task: {e}")))??;
    Ok(prepared.publish())
}

struct PreparedFiles {
    files: Vec<AgentVmFileAttachment>,
    // The worker result owns unpublished aliases even if its awaiting future is dropped.
    root: Option<tempfile::TempDir>,
    _lock: File,
}
impl PreparedFiles {
    fn publish(mut self) -> Vec<AgentVmFileAttachment> {
        if let Some(root) = self.root.take() {
            let _ = root.keep();
        }
        self.files
    }
}

fn prepare_blocking(
    share: &Path,
    bundle: &OciBundle,
    contract: &CreateAttachments,
    grants: &koci_sdk::FileGrants,
    manifest: Option<&AgentVmAttachmentManifest>,
) -> Result<PreparedFiles> {
    let config: serde_json::Value =
        serde_json::from_str(bundle.config_json()).map_err(|e| failure(e.to_string()))?;
    let digest = bundle.config_digest();
    let root = file_projection_root(share);
    let pending_root = match fs::symlink_metadata(&root) {
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(
            tempfile::Builder::new()
                .prefix(
                    root.file_name()
                        .ok_or_else(|| failure("projection root has no filename"))?,
                )
                .permissions(fs::Permissions::from_mode(0o700))
                .rand_bytes(0)
                .tempdir_in(
                    root.parent()
                        .ok_or_else(|| failure("projection root has no parent"))?,
                )
                .map_err(io_failure)?,
        ),
        Err(e) => return Err(io_failure(e)),
    };
    let lock = lock_root(&root)?;
    // Check every source before changing any caller inode's link count.
    let mut sources = Vec::new();
    for grant in grants.entries() {
        let source = source_path(&config, grant.mount_index())?;
        if source.starts_with(
            share
                .parent()
                .ok_or_else(|| failure("share has no parent"))?,
        ) {
            return Err(failure("caller file overlaps runtime control resources"));
        }
        for storage in contract.storage() {
            if grants
                .entries()
                .iter()
                .any(|g| g.identity() == storage.identity())
            {
                continue;
            }
            let pointer = storage.mount().json_pointer();
            let directory = config
                .pointer(pointer)
                .and_then(|m| m.get("source"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| failure("directory grant has no source"))?;
            if source.starts_with(directory) || Path::new(directory).starts_with(&source) {
                return Err(failure("file and directory grants overlap"));
            }
        }
        let tag = file_tag(digest, grant.identity().as_str())?;
        let file = open_source(
            &source,
            grant.source_identity(),
            if pending_root.is_some() { 1 } else { 2 },
        )?;
        sources.push((grant, source, tag, file));
    }
    validate_names(&root, sources.iter().map(|(_, _, tag, _)| tag.as_str()))?;
    let mut files = Vec::new();
    for (grant, source, tag, pinned) in sources {
        let projection = root.join(tag);
        let alias = projection.join(AGENT_VM_PROJECTED_FILE_NAME);
        if pending_root.is_some() {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&projection)
                .map_err(io_failure)?;
        }
        let prepare = || -> Result<_> {
            let dir_id = private_identity(&projection)?;
            if pending_root.is_some() {
                // /dev/fd hardlinking is unsupported on Darwin. Publish no VM until the
                // privately created pathname link matches the already pinned object.
                fs::hard_link(&source, &alias).map_err(|e| {
                    if e.raw_os_error() == Some(libc::EXDEV) {
                        Error::new(
                            ErrorCode::Unsupported,
                            "live file grants require the same filesystem; copying is forbidden",
                        )
                        .for_operation("prepare-hvf-files")
                    } else {
                        io_failure(e)
                    }
                })?;
            }
            validate_names(&projection, [AGENT_VM_PROJECTED_FILE_NAME].into_iter())?;
            check_file(
                &fs::symlink_metadata(&alias).map_err(io_failure)?,
                grant.source_identity(),
                2,
            )?;
            check_file(
                &pinned.metadata().map_err(io_failure)?,
                grant.source_identity(),
                2,
            )?;
            let _source = open_source(&source, grant.source_identity(), 2)?;
            let attachment = contract
                .storage()
                .iter()
                .find(|a| a.identity() == grant.identity())
                .ok_or_else(|| failure("file has no classified storage grant"))?;
            let file = AgentVmFileAttachment::new(
                attachment.clone(),
                source
                    .to_str()
                    .ok_or_else(|| failure("caller file is not UTF-8"))?,
                grant.source_identity(),
                dir_id,
                digest,
            )?;
            if let Some(manifest) = manifest {
                if !manifest.files().contains(&file) {
                    return Err(failure(
                        "private projection differs from its published identity",
                    ));
                }
            }
            Ok(file)
        };
        files.push(prepare()?);
    }
    files.sort();
    Ok(PreparedFiles {
        files,
        root: pending_root,
        _lock: lock,
    })
}

fn lock_root(root: &Path) -> Result<File> {
    let identity = private_identity(root)?;
    let lock = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY)
        .open(root)
        .map_err(io_failure)?;
    let opened = lock.metadata().map_err(io_failure)?;
    if opened.dev() != identity.device() || opened.ino() != identity.inode() {
        return Err(failure("projection root changed during admission"));
    }
    fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Error::new(
                ErrorCode::Conflict,
                "file projection is still being prepared",
            )
            .retryable(true)
            .for_operation("prepare-hvf-files")
        } else {
            io_failure(error)
        }
    })?;
    Ok(lock)
}

fn source_path(config: &serde_json::Value, index: usize) -> Result<PathBuf> {
    config
        .pointer(&format!("/mounts/{index}/source"))
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| failure("file grant has no exact mount source"))
}

fn open_source(path: &Path, id: FileSourceIdentity, links: u64) -> Result<File> {
    if fs::canonicalize(path).map_err(io_failure)?.as_os_str() != path.as_os_str() {
        return Err(failure("caller file resolves through a path alias"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW_ANY)
        .open(path)
        .map_err(io_failure)?;
    check_file(&file.metadata().map_err(io_failure)?, id, links)?;
    check_file(&fs::symlink_metadata(path).map_err(io_failure)?, id, links)?;
    Ok(file)
}
fn check_file(metadata: &fs::Metadata, id: FileSourceIdentity, links: u64) -> Result<()> {
    // SAFETY: geteuid has no preconditions or failure return.
    let uid = unsafe { libc::geteuid() };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || metadata.dev() != id.device()
        || metadata.ino() != id.inode()
        || metadata.nlink() != links
    {
        return Err(failure(
            "caller file identity, owner, type or link count changed",
        ));
    }
    Ok(())
}
fn private_identity(path: &Path) -> Result<AgentVmDirectorySourceIdentity> {
    let meta = fs::symlink_metadata(path).map_err(io_failure)?;
    if !is_private_directory(&meta) || fs::canonicalize(path).map_err(io_failure)? != path {
        return Err(failure(
            "file projection is not a private canonical directory",
        ));
    }
    AgentVmDirectorySourceIdentity::new(meta.dev(), meta.ino())
}
fn validate_names<'a>(root: &Path, allowed: impl Iterator<Item = &'a str>) -> Result<()> {
    let allowed: std::collections::BTreeSet<_> = allowed.collect();
    for entry in fs::read_dir(root).map_err(io_failure)? {
        let entry = entry.map_err(io_failure)?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|n| allowed.contains(n))
        {
            return Err(failure("file projection contains an unauthorized entry"));
        }
    }
    Ok(())
}

/// Called after VM reaping and exact-generation handoff marker validation.
pub(super) async fn cleanup(share: &Path, target: &koci_sdk::ContainerTarget) -> Result<()> {
    let root = file_projection_root(share);
    let Some(metadata) = path_metadata(&root).await? else {
        return Ok(());
    };
    if !is_private_directory(&metadata) {
        return Err(failure(
            "refusing cleanup of a non-private file projection root",
        ));
    }
    let bundle = OciBundle::load(share.join("bundle")).await?;
    let grants = koci_sdk::FileGrants::from_bundle(&bundle)?
        .ok_or_else(|| failure("projection has no file-grant owner"))?;
    let manifest_path = share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    let manifest = if path_metadata(&manifest_path).await?.is_some() {
        let m = super::vm_attachment_transport::read_manifest(&manifest_path).await?;
        m.validate_bundle(&bundle)?;
        if m.target() != target {
            return Err(failure("projection cleanup targets another generation"));
        }
        Some(m)
    } else {
        None
    };
    let root_copy = root.clone();
    let _lock = tokio::task::spawn_blocking(move || -> Result<File> {
        let lock = lock_root(&root_copy)?;
        let digest = bundle.config_digest();
        let tags = grants
            .entries()
            .iter()
            .map(|g| file_tag(digest, g.identity().as_str()))
            .collect::<Result<Vec<_>>>()?;
        validate_names(&root_copy, tags.iter().map(String::as_str))?;
        for (grant, tag) in grants.entries().iter().zip(tags) {
            let dir = root_copy.join(tag);
            if !dir.try_exists().map_err(io_failure)? {
                continue;
            }
            let id = private_identity(&dir)?;
            if let Some(m) = &manifest {
                if !m.files().iter().any(|f| {
                    f.attachment().identity() == grant.identity() && f.projection_identity() == id
                }) {
                    return Err(failure("refusing cleanup of a replaced file projection"));
                }
            }
            validate_names(&dir, [AGENT_VM_PROJECTED_FILE_NAME].into_iter())?;
            let alias = dir.join(AGENT_VM_PROJECTED_FILE_NAME);
            match fs::symlink_metadata(&alias) {
                Ok(m) => {
                    // Caller rename/unlink is allowed after entry. Cleanup revokes only
                    // our original alias, never follows the caller pathname or replacement.
                    if !m.is_file()
                        || m.dev() != grant.source_identity().device()
                        || m.ino() != grant.source_identity().inode()
                    {
                        return Err(failure("refusing cleanup of a replaced file alias"));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && manifest.is_none() => (),
                Err(e) => return Err(io_failure(e)),
            }
        }
        Ok(lock)
    })
    .await
    .map_err(|e| failure(e.to_string()))??;
    super::directory_cleanup::remove_directory_all_bound(&root, &metadata).await
}
fn io_failure(e: std::io::Error) -> Error {
    failure(format!("live file projection I/O: {e}"))
}
fn failure(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::FailedPrecondition, message).for_operation("prepare-hvf-files")
}

#[cfg(test)]
#[path = "hvf_files_tests.rs"]
mod tests;
