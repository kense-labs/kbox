//! Prepare existing Box anonymous capabilities as guest-native ext4 allocations.
use super::{spec::PlannedAnonymousVolume, VmManager};
use crate::{
    rootfs::{Ext4Artifact, Ext4ArtifactOptions, OciWorkloadRootfs},
    VolumeStore,
};
use kbox_core::{
    error::{BoxError, Result},
    volume::VolumeConfig,
};
use std::path::Path;

use crate::volume::ANONYMOUS_EXT4_ARTIFACT_DIRECTORY as ARTIFACT_DIRECTORY;

#[cfg(test)]
#[path = "oci_anonymous_tests.rs"]
mod tests;
#[cfg(test)]
fn disk_path(volume: &VolumeConfig) -> std::path::PathBuf {
    crate::volume::anonymous_ext4_disk_path(Path::new(&volume.mount_point))
}
fn allocation_uuid(volume: &VolumeConfig) -> Result<uuid::Uuid> {
    let value = volume
        .labels
        .get(crate::volume::ALLOCATION_ID_LABEL)
        .ok_or_else(|| {
            BoxError::ConfigError(
                "raw anonymous volume lacks a store-issued allocation identity".into(),
            )
        })?;
    uuid::Uuid::parse_str(value).map_err(|error| BoxError::ConfigError(error.to_string()))
}

impl VmManager {
    pub(super) fn anonymous_disk_plan(
        &self,
        image: &crate::oci::OciImageConfig,
    ) -> Result<Vec<(PlannedAnonymousVolume, VolumeConfig)>> {
        let store = VolumeStore::new(
            self.home_dir.join("volumes.json"),
            self.home_dir.join("volumes"),
        );
        let mut planned = Vec::new();
        for mut plan in self.plan_anonymous_volumes(image)? {
            if !self.anonymous_volumes.contains(&plan.name) {
                if self.anonymous_volumes.contains(&plan.legacy_name) {
                    plan.name = plan.legacy_name.clone();
                } else {
                    return Err(BoxError::ConfigError(
                        "anonymous disk differs from durable ownership plan".into(),
                    ));
                }
            }
            let volume = store.require_anonymous(&plan.name, &self.box_id)?;
            allocation_uuid(&volume)?;
            planned.push((plan, volume));
        }
        if planned.len() != self.anonymous_volumes.len() {
            return Err(BoxError::ConfigError(
                "anonymous ownership inventory drifted from image mounts".into(),
            ));
        }
        Ok(planned)
    }
}

/// This runs inside the existing private blocking rootfs job; never writes published volumes.
pub(super) fn stage(
    rootfs: &mut OciWorkloadRootfs,
    plans: &[(PlannedAnonymousVolume, VolumeConfig)],
    disk_mib: u32,
    staging: &Path,
) -> Result<Vec<(VolumeConfig, Ext4Artifact)>> {
    let mut staged = Vec::new();
    for (index, (plan, volume)) in plans.iter().enumerate() {
        let artifact = Path::new(&volume.mount_point).join(ARTIFACT_DIRECTORY);
        let options =
            Ext4ArtifactOptions::from_disk_mib(disk_mib, allocation_uuid(volume)?.into_bytes())?;
        match std::fs::symlink_metadata(&artifact) {
            Ok(_) => {
                let current = crate::rootfs::resume_volume_ext4_artifact(&artifact)?;
                if current.manifest.capacity_bytes != options.capacity_bytes
                    || current.manifest.fs_uuid != hex::encode(options.fs_uuid)
                {
                    return Err(BoxError::ConfigError(
                        "anonymous disk capacity or UUID differs from its allocation".into(),
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if std::fs::read_dir(&volume.mount_point)
                    .map_err(BoxError::IoError)?
                    .next()
                    .transpose()
                    .map_err(BoxError::IoError)?
                    .is_some()
                {
                    return Err(BoxError::ConfigError("anonymous allocation contains existing data without an ext4 artifact; refusing to replace it".into()));
                }
                let disk = rootfs.publish_volume(
                    &plan.guest_path,
                    &staging.join(format!("volume-{index}")),
                    options,
                )?;
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&disk.disk, std::fs::Permissions::from_mode(0o600))
                    .map_err(BoxError::IoError)?;
                std::fs::File::open(&disk.disk)
                    .and_then(|file| file.sync_all())
                    .map_err(BoxError::IoError)?;
                staged.push((volume.clone(), disk));
            }
            Err(error) => return Err(BoxError::IoError(error)),
        }
    }
    Ok(staged)
}

/// Publish complete artifacts under the existing catalog lock, with no late blocking job writes.
pub(super) fn publish(
    home: &Path,
    owner: &str,
    staged: Vec<(VolumeConfig, Ext4Artifact)>,
) -> Result<()> {
    if staged.is_empty() {
        return Ok(());
    }
    let store = VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    let _lock = crate::file_lock::FileLock::try_acquire(store.path())
        .map_err(BoxError::IoError)?
        .ok_or_else(|| {
            BoxError::StateError(
                "anonymous disk publication needs the exact catalog lock; retry the operation"
                    .into(),
            )
        })?;
    for (expected, artifact) in staged {
        let current = store.require_anonymous(&expected.name, owner)?;
        if current.mount_point != expected.mount_point
            || allocation_uuid(&current)? != allocation_uuid(&expected)?
        {
            return Err(BoxError::ConfigError(
                "anonymous allocation changed before disk publication".into(),
            ));
        }
        let destination = Path::new(&current.mount_point).join(ARTIFACT_DIRECTORY);
        if std::fs::read_dir(&current.mount_point)
            .map_err(BoxError::IoError)?
            .next()
            .transpose()
            .map_err(BoxError::IoError)?
            .is_some()
        {
            return Err(BoxError::ConfigError(
                "anonymous allocation changed before disk publication".into(),
            ));
        }
        std::fs::rename(&artifact.directory, &destination).map_err(BoxError::IoError)?;
        std::fs::File::open(&current.mount_point)
            .and_then(|file| file.sync_all())
            .map_err(BoxError::IoError)?;
    }
    Ok(())
}
