//! Leftover attachment alias cleanup for MicroVM destroy/reap.
//!
//! Aliases may remain under the legacy layout dir `sandbox/attachments`.
//! Destroy paths call [`cleanup_mount_aliases`] to tear them down.

use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use kbox_core::error::BoxError;
use kbox_core::error::Result;

#[cfg(target_os = "linux")]
const ATTACHMENTS_DIRECTORY: &str = "sandbox/attachments";

#[cfg(target_os = "linux")]
pub(crate) fn mount_alias_root(home_dir: &Path, box_id: &str) -> PathBuf {
    home_dir
        .join("boxes")
        .join(box_id)
        .join(ATTACHMENTS_DIRECTORY)
}

/// Unmount and remove every leftover attachment alias for one box.
///
/// Mount points are discovered from mountinfo instead of trusting directory
/// entries left by a crashed process. Nothing below the alias root is deleted
/// until every attached mount has been detached.
#[cfg(target_os = "linux")]
pub(crate) fn cleanup_mount_aliases(home_dir: &Path, box_id: &str) -> Result<()> {
    let root = mount_alias_root(home_dir, box_id);
    let mountinfo = std::fs::read_to_string("/proc/self/mountinfo").map_err(|error| {
        BoxError::StateError(format!(
            "Failed to inspect leftover attachment mounts for {box_id}: {error}"
        ))
    })?;
    let mut mount_points = mount_points_below(&mountinfo, &root);
    mount_points.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    for mount_point in mount_points {
        detach_mount(&mount_point).map_err(|error| {
            BoxError::StateError(format!(
                "Failed to detach attachment alias {}: {error}",
                mount_point.display()
            ))
        })?;
    }

    match std::fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            std::fs::remove_file(&root).map_err(BoxError::IoError)?;
            Err(BoxError::StateError(format!(
                "Attachment root was an unsafe symlink and was removed: {}",
                root.display()
            )))
        }
        Ok(metadata) if !metadata.is_dir() => Err(BoxError::StateError(format!(
            "Attachment root is not a directory: {}",
            root.display()
        ))),
        Ok(_) => std::fs::remove_dir_all(&root).map_err(BoxError::IoError),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(BoxError::IoError(error)),
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn cleanup_mount_aliases(_home_dir: &Path, _box_id: &str) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn detach_mount(target: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let target = std::ffi::CString::new(target.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "mount path has NUL"))?;
    // SAFETY: See the enclosing function's safety documentation.
    if unsafe { libc::umount2(target.as_ptr(), 0) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if matches!(
        error.raw_os_error(),
        Some(libc::EINVAL) | Some(libc::ENOENT)
    ) {
        return Ok(());
    }
    // SAFETY: See the enclosing function's safety documentation.
    if unsafe { libc::umount2(target.as_ptr(), libc::MNT_DETACH) } == 0 {
        return Ok(());
    }
    Err(std::io::Error::last_os_error())
}

#[cfg(target_os = "linux")]
fn mount_points_below(mountinfo: &str, root: &Path) -> Vec<PathBuf> {
    mountinfo
        .lines()
        .filter_map(|line| line.split_whitespace().nth(4))
        .map(decode_mountinfo_path)
        .map(PathBuf::from)
        .filter(|path| path == root || path.starts_with(root))
        .collect()
}

#[cfg(target_os = "linux")]
fn decode_mountinfo_path(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::host_fs::{validate_external_mount_access, IdMapping};

    fn mappings() -> IdMappingPlan {
        IdMappingPlan {
            uid_mappings: vec![IdMapping {
                container_id: 0,
                host_id: 100_000,
                size: 1,
            }],
            gid_mappings: vec![IdMapping {
                container_id: 0,
                host_id: 100_000,
                size: 1,
            }],
            maximum_container_uid: 0,
            maximum_container_gid: 0,
        }
    }

    fn can_mount() -> bool {
        // SAFETY: See the enclosing function's safety documentation.
        if unsafe { libc::geteuid() } != 0 {
            return false;
        }
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status
                    .lines()
                    .find_map(|line| line.strip_prefix("CapEff:\t"))
                    .and_then(|value| u64::from_str_radix(value, 16).ok())
            })
            .is_some_and(|capabilities| capabilities & (1 << 21) != 0)
    }

    fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

}
