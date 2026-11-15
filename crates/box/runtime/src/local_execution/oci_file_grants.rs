//! Caller file incarnations selected by the existing Box resource plan.
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use kbox_core::{BoxError, Result};
use koci_sdk::FileSourceIdentity;

use crate::{vm::VmManager, BoxRecord};

pub(crate) fn observe(path: &Path, unaliased: bool) -> Result<FileSourceIdentity> {
    let metadata = std::fs::symlink_metadata(path).map_err(BoxError::IoError)?;
    // SAFETY: geteuid has no preconditions or failure return.
    let uid = unsafe { libc::geteuid() };
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || (unaliased && metadata.nlink() != 1)
        || path.canonicalize().map_err(BoxError::IoError)?.as_os_str() != path.as_os_str()
    {
        return Err(BoxError::ConfigError(
            "caller file requires a canonical absolute, caller-owned unaliased regular source"
                .into(),
        ));
    }
    FileSourceIdentity::new(metadata.dev(), metadata.ino())
        .map_err(|error| BoxError::ConfigError(error.to_string()))
}

pub(crate) fn capture(record: &BoxRecord) -> Result<BTreeMap<PathBuf, FileSourceIdentity>> {
    let metadata = record
        .managed_execution
        .as_ref()
        .ok_or_else(|| BoxError::StateError("file planning has no managed execution".into()))?;
    let mut files = BTreeMap::new();
    for volume in &metadata.request.config.volumes {
        let path = VmManager::parse_volume_spec(volume)?.host_path;
        if std::fs::symlink_metadata(&path)
            .map_err(BoxError::IoError)?
            .is_file()
        {
            let identity = observe(&path, true)?;
            if files.insert(path, identity).is_some() {
                return Err(BoxError::ConfigError(
                    "duplicate file source grants are unsupported".into(),
                ));
            }
        }
    }
    Ok(files)
}

pub(crate) fn validate_bound(record: &BoxRecord) -> Result<()> {
    let metadata = record
        .managed_execution
        .as_ref()
        .ok_or_else(|| BoxError::StateError("file binding has no managed execution".into()))?;
    let mut seen = std::collections::BTreeSet::new();
    for volume in &metadata.request.config.volumes {
        let path = VmManager::parse_volume_spec(volume)?.host_path;
        if metadata.caller_files.contains_key(&path)
            || std::fs::symlink_metadata(&path)
                .map_err(BoxError::IoError)?
                .is_file()
        {
            let actual = observe(&path, false)?;
            if metadata.caller_files.get(&path) != Some(&actual) {
                return Err(BoxError::StateError("caller file identity differs from its durable reservation; replacement is not reauthorized".into()));
            }
            seen.insert(path);
        }
    }
    if seen.len() != metadata.caller_files.len() {
        return Err(BoxError::StateError(
            "caller file binding is not part of the persisted creation intent".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "oci_file_grants_tests.rs"]
mod tests;
