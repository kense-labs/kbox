use super::{require, Result};

pub(super) const TMPFS_SIZE_BYTES: u64 = 4 * 1024 * 1024;

/// Box has exactly one isolation boundary: a hardware-backed MicroVM.
///
/// Mount intent is validated from the persisted managed request. The guest owns
/// the mounts, so there is no host-side shared-kernel OCI bundle, attachment
/// alias, or host mount namespace to inspect here. Live mount visibility is
/// asserted from inside the guest by the mounts conformance profile.
pub(super) fn require_bind_config(
    record: &crate::BoxRecord,
    target: &str,
    read_only: bool,
) -> Result<()> {
    let config = &record
        .managed_execution
        .as_ref()
        .ok_or_else(|| super::protocol("Volume fixture lost managed metadata"))?
        .request
        .config;
    let expected_suffix = format!(":{target}:{}", if read_only { "ro" } else { "rw" });
    require(
        config
            .volumes
            .iter()
            .filter(|volume| volume.ends_with(&expected_suffix))
            .count()
            == 1,
        "Runtime Volume intent changed before provider launch",
    )
}

pub(super) fn require_tmpfs_config(
    record: &crate::BoxRecord,
    target: &str,
    read_only: bool,
) -> Result<()> {
    let config = &record
        .managed_execution
        .as_ref()
        .ok_or_else(|| super::protocol("tmpfs fixture lost managed metadata"))?
        .request
        .config;
    let expected = format!(
        "{target}:size={TMPFS_SIZE_BYTES},{}",
        if read_only { "ro" } else { "rw" }
    );
    require(
        config.tmpfs == vec![expected],
        "Runtime tmpfs intent changed before provider launch",
    )
}

pub(super) fn require_live_tmpfs_mount(
    record: &crate::BoxRecord,
    target: &str,
    read_only: bool,
) -> Result<()> {
    require_tmpfs_config(record, target, read_only)
}
