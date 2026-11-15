//! Volume management for persistent named volumes.
//!
//! Provides `VolumeStore` for persisting volume state and
//! managing volume data directories.

mod store;

pub use store::VolumeStore;
pub(crate) use store::ALLOCATION_ID_LABEL;

pub(crate) const ANONYMOUS_EXT4_ARTIFACT_DIRECTORY: &str = "ext4-v1";

pub(crate) fn anonymous_ext4_disk_path(source: &std::path::Path) -> std::path::PathBuf {
    source
        .join(ANONYMOUS_EXT4_ARTIFACT_DIRECTORY)
        .join("rootfs.ext4")
}
