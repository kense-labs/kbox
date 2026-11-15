//! Bounded Box tmpfs intent for the existing OCI Guest mount executor.

use super::{error, VmManager};
use kbox_core::{guest_exec::GuestTmpfsMount, BoxConfig, Result};
use oci_spec::runtime::{Mount, MountBuilder};
use std::{collections::HashSet, path::Path};

pub(super) fn mounts(config: &BoxConfig) -> Result<Vec<Mount>> {
    if config.tmpfs.len() > 128 {
        return Err(error("tmpfs exceeds 128 mounts"));
    }
    let mut targets = Vec::new();
    let mut mounts = Vec::new();
    let mut other_targets = vec!["/workspace".to_owned()];
    for volume in &config.volumes {
        other_targets.push(VmManager::parse_volume_spec(volume)?.guest_path);
    }
    for value in &config.tmpfs {
        let parsed = GuestTmpfsMount::parse(value).map_err(error)?;
        let path = VmManager::normalize_guest_mount_path(parsed.path())?;
        let shared_memory = path == "/dev/shm";
        if (!shared_memory
            && ["/dev", "/proc", "/sys"]
                .into_iter()
                .any(|other| overlaps(&path, other)))
            || other_targets.iter().any(|other| overlaps(&path, other))
            || targets.iter().any(|other: &String| overlaps(&path, other))
        {
            return Err(error("tmpfs overlaps a system, caller, or tmpfs mount"));
        }
        let mut keys = HashSet::new();
        for option in parsed.options() {
            if kbox_core::guest_exec::TmpfsMountFlag::parse(option).is_some() {
                continue;
            }
            let (key, value) = option
                .split_once('=')
                .ok_or_else(|| error("unsupported tmpfs option"))?;
            if !keys.insert(key) {
                return Err(error("duplicate tmpfs data option"));
            }
            let valid = match key {
                "size" => size(value).is_some_and(|bytes| bytes > 0 && bytes <= 512 * 1024 * 1024),
                "mode" => {
                    !value.is_empty()
                        && value.bytes().all(|b| matches!(b, b'0'..=b'7'))
                        && u32::from_str_radix(value, 8).is_ok_and(|mode| mode <= 0o7777)
                }
                "uid" | "gid" => {
                    !value.is_empty()
                        && value.bytes().all(|b| b.is_ascii_digit())
                        && value.parse::<u32>().is_ok_and(|id| id != u32::MAX)
                }
                _ => false,
            };
            if !valid {
                return Err(error("unsupported or out-of-range tmpfs data option"));
            }
        }
        let mut options: Vec<String> = parsed.options().map(str::to_owned).collect();
        if shared_memory {
            let bytes = options
                .first()
                .filter(|_| options.len() == 1)
                .and_then(|option| option.strip_prefix("size="))
                .and_then(size)
                .filter(|bytes| *bytes >= 4096 && bytes % 4096 == 0)
                .ok_or_else(|| error("/dev/shm requires one page-aligned size option"))?;
            // Match the Guest's synthesized shared-memory safety flags and sticky mode.
            options = vec![
                "rw".into(),
                "nosuid".into(),
                "nodev".into(),
                "noexec".into(),
                "mode=1777".into(),
                format!("size={bytes}"),
            ];
        }
        if !options
            .iter()
            .any(|option| matches!(option.as_str(), "ro" | "rw"))
        {
            options.push("rw".into());
        }
        mounts.push(
            MountBuilder::default()
                .destination(path.clone())
                .typ("tmpfs")
                .source(if shared_memory { "shm" } else { "tmpfs" })
                .options(options)
                .build()
                .map_err(error)?,
        );
        targets.push(path);
    }
    Ok(mounts)
}

fn size(value: &str) -> Option<u64> {
    let (digits, scale) = match value.as_bytes().last()? {
        b'k' | b'K' => (&value[..value.len() - 1], 1024),
        b'm' | b'M' => (&value[..value.len() - 1], 1024 * 1024),
        b'g' | b'G' => (&value[..value.len() - 1], 1024 * 1024 * 1024),
        _ => (value, 1),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u64>().ok()?.checked_mul(scale)
}

pub(super) fn overlaps(left: &str, right: &str) -> bool {
    Path::new(left).starts_with(right) || Path::new(right).starts_with(left)
}
