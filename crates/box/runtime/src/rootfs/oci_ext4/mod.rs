//! Mount-free OCI layer assembly for guest-native ext4 root filesystems.

use std::path::Path;

use kbox_core::error::{BoxError, Result};

use crate::oci::OciImage;

use super::ext4::{publish_ext4_build_plan, Ext4Artifact, Ext4ArtifactOptions};

mod layer;
mod plan;
mod runtime;
mod spool;
mod tree;

use tree::LogicalRootfs;

pub(super) const CONTENT_STAGING_PREFIX: &str = ".koci-ext4-content-";

/// OCI workload filesystem, independent of the VM's system image and PID 1.
pub(crate) struct OciWorkloadRootfs(LogicalRootfs);

impl std::fmt::Debug for OciWorkloadRootfs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OciWorkloadRootfs")
            .field("entries", &self.0.entries.len())
            .finish_non_exhaustive()
    }
}

impl OciWorkloadRootfs {
    pub(crate) fn load(image: &OciImage, parent: &Path) -> Result<Self> {
        Ok(Self(assemble_image(image, parent)?))
    }

    /// Capture the pristine CLI baseline without materializing a host rootfs.
    pub(crate) fn diff_baseline_bytes(&self) -> Result<Vec<u8>> {
        use kbox_core::rootfs_baseline::{
            GuestDiffBaseline, RootfsFileInfo, MAX_GUEST_DIFF_BASELINE_BYTES,
        };
        use std::collections::BTreeMap;
        use std::os::unix::ffi::OsStrExt;
        let mut entries = BTreeMap::new();
        for (path, entry) in &self.0.entries {
            if path.is_empty() {
                continue;
            }
            let raw = path.join(&b'/');
            let relative = Path::new(std::ffi::OsStr::from_bytes(&raw));
            if kbox_core::rootfs_metadata::is_runtime_internal_rootfs_path(relative) {
                continue;
            }
            let node = &self.0.nodes[&entry.node];
            let (size, kind, is_dir) = match &node.kind {
                tree::NodeKind::Directory => (0, 0o040000, true),
                tree::NodeKind::Regular { size, .. } => (*size, 0o100000, false),
                tree::NodeKind::Symlink { target } => (target.len() as u64, 0o120000, false),
                tree::NodeKind::Special(_) => continue,
            };
            entries.insert(
                super::baseline::rootfs_path_string(relative),
                RootfsFileInfo {
                    size,
                    mode: kind | u32::from(node.meta.mode & 0o7777),
                    is_dir,
                },
            );
        }
        let baseline = GuestDiffBaseline::new(entries);
        baseline.validate().map_err(BoxError::BuildError)?;
        let encoded = serde_json::to_vec(&baseline.entries)
            .map_err(|error| BoxError::BuildError(error.to_string()))?;
        if encoded.len() > MAX_GUEST_DIFF_BASELINE_BYTES {
            return Err(BoxError::BuildError(
                "workload diff baseline exceeds the existing byte limit".into(),
            ));
        }
        Ok(encoded)
    }

    pub(crate) fn publish(
        &self,
        destination: &Path,
        options: Ext4ArtifactOptions,
    ) -> Result<Ext4Artifact> {
        let (builder, fills) = self.0.declare(options)?;
        publish_ext4_build_plan(destination, options, builder, fills)
    }

    /// Seed an independent volume from the layered Guest namespace without Host chown.
    pub(crate) fn publish_volume(
        &mut self,
        guest_directory: &str,
        destination: &Path,
        options: Ext4ArtifactOptions,
    ) -> Result<Ext4Artifact> {
        if !guest_directory.starts_with('/') || guest_directory == "/" {
            return Err(tree::build_error(
                "volume seed requires an absolute non-root directory",
            ));
        }
        let path = tree::guest_path(guest_directory.trim_start_matches('/'))?;
        let directory = self
            .0
            .resolve_directory(&path, true)?
            .ok_or_else(|| tree::build_error("volume seed directory could not be resolved"))?;
        if directory.is_empty() {
            return Err(tree::build_error("volume seed resolves to the Guest root"));
        }
        let (builder, fills) = self.0.declare_subtree(&directory, options)?;
        publish_ext4_build_plan(destination, options, builder, fills)
    }
}

impl crate::process_path::ProcessRootfs for OciWorkloadRootfs {
    fn ensure_directory(&mut self, path: &str) -> Result<()> {
        self.0
            .resolve_directory(&tree::guest_path(path.trim_start_matches('/'))?, true)?;
        Ok(())
    }

    fn executable_exists(&mut self, path: &str) -> Result<bool> {
        let Some(path) = self.0.resolve_path(
            &tree::guest_path(path.trim_start_matches('/'))?,
            true,
            false,
        )?
        else {
            return Ok(false);
        };
        let node = &self.0.nodes[&self.0.entries[&path].node];
        Ok(matches!(node.kind, tree::NodeKind::Regular { .. }) && node.meta.mode & 0o111 != 0)
    }

    fn read_text(&mut self, path: &str) -> Result<Option<String>> {
        self.0.read_runtime_text(path)
    }
}

fn assemble_image(image: &OciImage, parent: &Path) -> Result<LogicalRootfs> {
    let mut rootfs = LogicalRootfs::new(parent)?;
    rootfs.create_base_structure()?;
    for layer in image.layer_blobs() {
        layer::apply_layer(layer.path, layer.digest, layer.size, &mut rootfs)?;
    }
    Ok(rootfs)
}

pub(super) fn publish_oci_layers_ext4(
    image: &OciImage,
    guest_init: &Path,
    guest_init_sha256: &str,
    destination: &Path,
    options: Ext4ArtifactOptions,
) -> Result<Ext4Artifact> {
    let parent = destination.parent().ok_or_else(|| {
        BoxError::BuildError(format!(
            "OCI ext4 destination has no parent: {}",
            destination.display()
        ))
    })?;
    let mut rootfs = assemble_image(image, parent)?;
    rootfs.install_guest_init(guest_init, guest_init_sha256)?;
    rootfs.create_essential_files()?;
    rootfs.validate_boot_contract()?;
    let (builder, fills) = rootfs.declare(options)?;
    publish_ext4_build_plan(destination, options, builder, fills)
}

#[cfg(test)]
mod tests;
