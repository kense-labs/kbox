//! Retain boot-failure diagnostics before a failed boot's teardown deletes
//! the box directory. A failed boot's console and shim logs are the only
//! evidence for why the guest exec server never became ready; keep one
//! bounded copy per box id under `<home>/diag/failed-boots/<box_id>/`.

use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, DirBuilder, OpenOptions};
#[cfg(unix)]
use cap_std::fs::{DirBuilderExt, OpenOptionsExt};
use std::ffi::OsStr;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};

const MAX_ENTRIES: usize = 64;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SNAPSHOT_BYTES: u64 = 8 * MAX_FILE_BYTES;
const PRIORITY_LOGS: [&str; 4] = [
    "console.log",
    "console.err.log",
    "shim.stdout.log",
    "shim.stderr.log",
];

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    files: usize,
    bytes: u64,
    skipped: usize,
    truncated: usize,
    limit_reached: bool,
}

/// Copy `box_dir/logs` into the retained diagnostics location. One copy per
/// box id, replaced on retry. Best-effort: failures are logged and never
/// propagate into the boot-failure path.
pub(crate) fn retain_boot_failure_logs(home_dir: &Path, box_dir: &Path, box_id: &str) {
    match retain(home_dir, box_dir, box_id) {
        Ok(snapshot) => tracing::debug!(box_id, ?snapshot, "Retained bounded boot-failure logs"),
        Err(error) => tracing::warn!(box_id, %error, "Failed to retain boot-failure logs"),
    }
}

fn retain(home_dir: &Path, box_dir: &Path, box_id: &str) -> io::Result<Snapshot> {
    let mut components = Path::new(box_id).components();
    if box_id.len() > 128
        || !matches!(components.next(), Some(Component::Normal(name)) if name == OsStr::new(box_id))
        || components.next().is_some()
        || box_dir != home_dir.join("boxes").join(box_id)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid diagnostic scope",
        ));
    }
    let home = Dir::open_ambient_dir(home_dir, cap_std::ambient_authority())?;
    let source = home
        .open_dir_nofollow("boxes")?
        .open_dir_nofollow(box_id)?
        .open_dir_nofollow("logs")?;
    let diag = private_child(&home, "diag")?;
    let failures = private_child(&diag, "failed-boots")?;
    let slot = private_child(&failures, box_id)?;
    let target = private_child(&slot, "logs")?;
    clear_bounded_slot(&target)?;
    match slot.remove_file_or_symlink("retention.json") {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let snapshot = copy_logs(&source, &target)?;
    let temporary = format!(".retention-{}", uuid::Uuid::new_v4());
    write_report(&slot, &temporary, &snapshot)?;
    Ok(snapshot)
}

fn write_report(slot: &Dir, temporary: &str, snapshot: &Snapshot) -> io::Result<()> {
    let report = (|| {
        let mut file = slot.open_with(temporary, &private_file_options())?;
        serde_json::to_writer_pretty(&mut file, snapshot)?;
        file.write_all(b"\n")?;
        drop(file);
        slot.rename(temporary, slot, "retention.json")
    })();
    if report.is_err() {
        let _ = slot.remove_file_or_symlink(temporary);
    }
    report
}

fn private_child(parent: &Dir, name: &str) -> io::Result<Dir> {
    match parent.open_dir_nofollow(name) {
        Ok(directory) => Ok(directory),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let mut builder = DirBuilder::new();
            #[cfg(unix)]
            builder.mode(0o700);
            match parent.create_dir_with(name, &builder) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
            parent.open_dir_nofollow(name)
        }
        Err(error) => Err(error),
    }
}

fn private_file_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.mode(0o600);
    options
}

fn clear_bounded_slot(target: &Dir) -> io::Result<()> {
    let mut names = Vec::new();
    for entry in target.entries()?.take(MAX_ENTRIES + 1) {
        let entry = entry?;
        if names.len() == MAX_ENTRIES || entry.file_type()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsafe prior diagnostic slot",
            ));
        }
        names.push(entry.file_name());
    }
    for name in names {
        target.remove_file_or_symlink(name)?;
    }
    Ok(())
}

fn copy_logs(source: &Dir, target: &Dir) -> io::Result<Snapshot> {
    let mut snapshot = Snapshot::default();
    for name in PRIORITY_LOGS {
        copy_log(source, target, OsStr::new(name), &mut snapshot)?;
    }
    for (index, entry) in source.entries()?.take(MAX_ENTRIES + 1).enumerate() {
        if index == MAX_ENTRIES || snapshot.files == MAX_ENTRIES {
            snapshot.limit_reached = true;
            break;
        }
        let name = entry?.file_name();
        if !PRIORITY_LOGS.iter().any(|priority| name == *priority) {
            copy_log(source, target, &name, &mut snapshot)?;
        }
    }
    Ok(snapshot)
}

fn copy_log(source: &Dir, target: &Dir, name: &OsStr, snapshot: &mut Snapshot) -> io::Result<()> {
    let metadata = match source.symlink_metadata(name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        snapshot.skipped += 1;
        return Ok(());
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK);
    let mut input = source.open_with(name, &options)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        snapshot.skipped += 1;
        return Ok(());
    }
    let remaining = MAX_SNAPSHOT_BYTES - snapshot.bytes;
    if remaining == 0 {
        snapshot.limit_reached = true;
        return Ok(());
    }
    let length = metadata.len().min(MAX_FILE_BYTES).min(remaining);
    input.seek(SeekFrom::Start(metadata.len() - length))?;
    let mut output = target.open_with(name, &private_file_options())?;
    let bytes = io::copy(&mut input.take(length), &mut output)?;
    snapshot.files += 1;
    snapshot.bytes += bytes;
    snapshot.truncated += usize::from(bytes < metadata.len());
    Ok(())
}

#[cfg(test)]
#[path = "boot_diag_tests.rs"]
mod tests;
