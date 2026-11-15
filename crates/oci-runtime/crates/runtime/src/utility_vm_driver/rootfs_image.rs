//! Generation-owned raw workload disk outside the guest-exported runtime share.
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use koci_agent_protocol::{
    workload_rootfs_disk_path, AgentVmRootfsImage, AgentVmStorageSourceIdentity,
    AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME,
};
use koci_sdk::{Error, ErrorCode, Result, RootfsImage, ROOTFS_IMAGE_FILE_NAME};

use super::layout::{is_private_file, path_metadata};
use super::UtilityVmLaunchRequest;

pub(super) async fn prepare(
    request: &UtilityVmLaunchRequest<'_>,
) -> Result<Option<AgentVmRootfsImage>> {
    let Some(image) = request.attachment_contract.rootfs_image(request.bundle)? else {
        return Ok(None);
    };
    let source = request.bundle.directory().join(ROOTFS_IMAGE_FILE_NAME);
    let disk = workload_rootfs_disk_path(request.runtime_share);
    let manifest_path = request
        .runtime_share
        .join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    if let Some(metadata) = path_metadata(&disk).await? {
        if path_metadata(&source).await?.is_some() {
            return Err(failure("both workload disk and operation input exist"));
        }
        let rootfs = AgentVmRootfsImage::new(image.clone(), identity(&metadata)?)?;
        if path_metadata(&manifest_path).await?.is_some() {
            let manifest = super::vm_attachment_transport::read_manifest(&manifest_path).await?;
            manifest.validate_bundle(request.bundle)?;
            if manifest.target() != request.target || manifest.rootfs_image() != Some(&rootfs) {
                return Err(failure(
                    "existing workload disk differs from its exact generation manifest",
                ));
            }
        }
        // Launch retries are pre-entry only; a modified prior disk cannot become a fresh VM.
        verify_content(&disk, &image).await?;
        return Ok(Some(rootfs));
    }
    let metadata = tokio::fs::symlink_metadata(&source)
        .await
        .map_err(io_failure)?;
    let id = identity(&metadata)?;
    verify_content(&source, &image).await?;
    let current = tokio::fs::symlink_metadata(&source)
        .await
        .map_err(io_failure)?;
    if identity(&current)? != id {
        return Err(failure("rootfs input changed during verification"));
    }
    tokio::fs::rename(&source, &disk)
        .await
        .map_err(io_failure)?;
    let moved = tokio::fs::symlink_metadata(&disk)
        .await
        .map_err(io_failure)?;
    if identity(&moved)? != id {
        return Err(failure("rootfs input changed during ownership transfer"));
    }
    Ok(Some(AgentVmRootfsImage::new(image, id)?))
}

pub(super) fn identity(metadata: &std::fs::Metadata) -> Result<AgentVmStorageSourceIdentity> {
    if !is_private_file(metadata) || metadata.nlink() != 1 {
        return Err(failure(
            "workload disk must be a private, single-link regular file",
        ));
    }
    AgentVmStorageSourceIdentity::new(
        metadata.dev(),
        metadata.rdev(),
        metadata.ino(),
        metadata.len(),
    )
}

async fn verify_content(path: &Path, image: &RootfsImage) -> Result<()> {
    let file = crate::file_security::open_verified_regular_file(path)
        .await
        .map_err(io_failure)?;
    let metadata = file.file.metadata().await.map_err(io_failure)?;
    if identity(&metadata)?.size() != image.size() {
        return Err(failure("rootfs image capacity changed"));
    }
    let digest = crate::file_security::sha256_verified_file(file, Some(image.size()), path)
        .await
        .map_err(io_failure)?;
    if format!("sha256:{digest}") != image.sha256() {
        return Err(failure("workload rootfs content digest mismatch"));
    }
    Ok(())
}

/// Called only after exact-generation handoff ownership and VM reaping were established.
pub(super) async fn cleanup(share: &Path, target: &koci_sdk::ContainerTarget) -> Result<()> {
    let disk = workload_rootfs_disk_path(share);
    let Some(metadata) = path_metadata(&disk).await? else {
        return Ok(());
    };
    let id = identity(&metadata)?;
    let bundle = koci_sdk::OciBundle::load(share.join("bundle")).await?;
    let image = RootfsImage::from_bundle(&bundle)?
        .ok_or_else(|| failure("workload disk has no bundle owner"))?;
    let manifest_path = share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME);
    if path_metadata(&manifest_path).await?.is_some() {
        let manifest = super::vm_attachment_transport::read_manifest(&manifest_path).await?;
        manifest.validate_bundle(&bundle)?;
        if manifest.target() != target
            || manifest
                .rootfs_image()
                .map(AgentVmRootfsImage::source_identity)
                != Some(id)
        {
            return Err(failure("refusing cleanup of a replaced workload disk"));
        }
    } else {
        verify_content(&disk, &image).await?;
    }
    super::vm_attachment_transport::remove_private_file_bound(&disk, &metadata).await
}

fn io_failure(error: std::io::Error) -> Error {
    failure(format!("workload disk I/O: {error}"))
}
fn failure(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::FailedPrecondition, message).for_operation("prepare-hvf-rootfs-image")
}
