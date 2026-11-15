//! File-authority records use a directory so unaware readers and stale writers fail I/O.
use std::path::{Path, PathBuf};

pub(super) const RECORDS: &str = "records.v1.json";

pub(super) fn data_path(path: &Path) -> std::io::Result<Option<PathBuf>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_dir() {
        if metadata.file_type().is_symlink() && path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Box state directory cannot be a symlink",
            ));
        }
        return Ok(None);
    }
    let records = path.join(RECORDS);
    let metadata = std::fs::symlink_metadata(&records)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Box state records must be a regular file",
        ));
    }
    Ok(Some(records))
}

#[cfg(target_os = "macos")]
pub(super) fn publish(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory = tempfile::Builder::new()
        .prefix(".kbox-state-v1-")
        .tempdir_in(parent)?;
    kbox_core::fs_atomic::write_durable(
        &directory.path().join("records.tmp"),
        &directory.path().join(RECORDS),
        bytes,
    )?;
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            let source = CString::new(directory.path().as_os_str().as_bytes())
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
            let destination = CString::new(path.as_os_str().as_bytes())
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
            // SAFETY: both C strings remain alive; the caller holds the canonical state lock.
            if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_SWAP) }
                != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            // The old flat state now occupies this exact owned staging path; the new copy is published.
            std::fs::remove_file(directory.keep())?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::rename(directory.path(), path)?;
            let _published = directory.keep();
        }
        Err(error) => return Err(error),
    }
    if let Ok(parent) = std::fs::File::open(parent) {
        let _ = parent.sync_all();
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(super) fn publish(_path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this platform cannot publish a file-authority state store",
    ))
}
