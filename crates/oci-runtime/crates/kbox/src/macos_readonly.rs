//! Shared host write boundary for the Box and OCI macOS VM workers.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use cap_std::fs::{Dir, MetadataExt as _};
use koci_sdk::Result;

use crate::macos_worker_sandbox::{add_denied_rule, install, sandbox_error};

/// Descriptor-pinned read-only exports, enforced by the host before VM entry.
///
/// The caller retains this guard for the VM lifetime and passes `pinned_path`
/// to libkrun. Writable paths must include every other export and host control
/// path. Overlap and pre-existing hard links are refused rather than weakening
/// the source's access mode. Other host processes retain their own permissions.
#[derive(Debug)]
pub struct MacosReadOnlyMounts {
    directories: Vec<(PathBuf, File)>,
}

impl MacosReadOnlyMounts {
    /// Validate and pin caller directories without following their final entry.
    pub fn open(read_only: &[&Path], writable: &[&Path]) -> Result<Self> {
        let writable = writable
            .iter()
            .map(|path| canonical_resource_path(path))
            .collect::<Result<Vec<_>>>()?;
        let mut directories: Vec<(PathBuf, File)> = Vec::new();
        for path in read_only {
            let canonical = path.canonicalize().map_err(|error| {
                sandbox_error(format!("failed to resolve read-only export: {error}"))
            })?;
            if fs::symlink_metadata(path)
                .map_err(|error| sandbox_error(error.to_string()))?
                .file_type()
                .is_symlink()
            {
                return Err(sandbox_error(
                    "read-only exports must be plain disjoint directories outside writable grants",
                ));
            }
            for other in writable
                .iter()
                .chain(directories.iter().map(|(path, _)| path))
            {
                if overlaps(&canonical, other)? {
                    return Err(sandbox_error(
                        "read-only exports overlap writable grants or another export",
                    ));
                }
            }
            let directory = pin_directory(&canonical)?;
            directories.push((canonical, directory));
        }
        let mounts = Self { directories };
        mounts.reverify()?;
        Ok(mounts)
    }

    /// Stable fdesc path for the export at the original read-only input index.
    pub fn pinned_path(&self, index: usize) -> Option<PathBuf> {
        self.directories
            .get(index)
            .map(|(_, file)| descriptor_path(file))
    }

    /// Reject a source replacement between admission and the actual VM entry.
    pub fn reverify(&self) -> Result<()> {
        for (path, directory) in &self.directories {
            let current =
                fs::symlink_metadata(path).map_err(|error| sandbox_error(error.to_string()))?;
            let pinned = directory
                .metadata()
                .map_err(|error| sandbox_error(error.to_string()))?;
            if !current.is_dir()
                || current.file_type().is_symlink()
                || current.dev() != pinned.dev()
                || current.ino() != pinned.ino()
                || path
                    .canonicalize()
                    .map_err(|error| sandbox_error(error.to_string()))?
                    != *path
            {
                return Err(sandbox_error(
                    "read-only export identity changed before VM entry",
                ));
            }
            validate_unaliased_tree(directory)?;
        }
        Ok(())
    }

    /// Install the irreversible process-wide host boundary, inherited by threads.
    /// Host subprocesses and links involving read-only exports are denied.
    /// Guest processes are
    /// unaffected because they execute under the VM's independent kernel.
    pub fn confine(&self) -> Result<()> {
        let mut profile =
            String::from("(version 1)\n(allow default)\n(deny process-exec process-fork)\n");
        let mut parameters = Vec::new();
        self.append_rules(&mut profile, &mut parameters)?;
        install(&profile, &parameters)
    }

    pub(super) fn append_rules(
        &self,
        profile: &mut String,
        parameters: &mut Vec<String>,
    ) -> Result<()> {
        self.reverify()?;
        self.validate_open_descriptors()?;
        let mut ancestors = BTreeSet::new();
        for (path, _) in &self.directories {
            add_denied_rule(
                profile,
                parameters,
                "file-write* file-link",
                "subpath",
                path,
            )?;
            ancestors.extend(path.ancestors().skip(1));
        }
        for ancestor in ancestors {
            // Renaming an ancestor must not move a protected vnode out of its rule.
            add_denied_rule(
                profile,
                parameters,
                "file-write-unlink",
                "literal",
                ancestor,
            )?;
        }
        Ok(())
    }

    fn validate_open_descriptors(&self) -> Result<()> {
        validate_open_descriptors(&self.directories, &[])
    }
}

/// A pre-sandbox writable descriptor retains authority on Darwin; refuse it before activation.
pub(crate) fn validate_read_only_file_descriptors(files: &[&Path]) -> Result<()> {
    validate_open_descriptors(&[], files)
}

fn validate_open_descriptors(directories: &[(PathBuf, File)], files: &[&Path]) -> Result<()> {
    let mut file_identities = BTreeSet::new();
    for path in files {
        let metadata = fs::metadata(path).map_err(|error| sandbox_error(error.to_string()))?;
        if metadata.is_file() {
            file_identities.insert((metadata.dev(), metadata.ino()));
        }
    }
    if directories.is_empty() && file_identities.is_empty() {
        return Ok(());
    }
    for entry in fs::read_dir("/dev/fd").map_err(|error| sandbox_error(error.to_string()))? {
        let entry = entry.map_err(|error| sandbox_error(error.to_string()))?;
        let Some(fd) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        // SAFETY: F_GETFL only reads the flags of an integer descriptor.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || flags & libc::O_ACCMODE == libc::O_RDONLY {
            continue;
        }
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: stat points to a correctly sized writable native stat buffer.
        if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
            return Err(sandbox_error(
                "file descriptor changed during read-only admission",
            ));
        }
        // SAFETY: fstat succeeded and initialized the complete buffer.
        let stat = unsafe { stat.assume_init() };
        if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
            continue;
        }
        if file_identities.contains(&(stat.st_dev as u64, stat.st_ino)) {
            return Err(sandbox_error(
                "pre-opened writable descriptor aliases a read-only file",
            ));
        }
        if directories.is_empty() {
            continue;
        }
        let mut path = [0u8; libc::MAXPATHLEN as usize];
        // SAFETY: F_GETPATH writes at most MAXPATHLEN bytes to this buffer.
        if unsafe { libc::fcntl(fd, libc::F_GETPATH, path.as_mut_ptr()) } != 0 {
            return Err(sandbox_error(
                "cannot identify a pre-opened writable file descriptor",
            ));
        }
        let end = path
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| sandbox_error("unterminated descriptor path"))?;
        let path = Path::new(std::ffi::OsStr::from_bytes(&path[..end]));
        for (source, _) in directories {
            if overlaps(source, path)? {
                return Err(sandbox_error(
                    "pre-opened writable descriptor overlaps a read-only export",
                ));
            }
        }
    }
    Ok(())
}

fn canonical_resource_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(sandbox_error(
            "writable resource paths must be absolute and normalized",
        ));
    }
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .ok_or_else(|| sandbox_error("resource path has no parent"))?;
            let name = path
                .file_name()
                .ok_or_else(|| sandbox_error("resource path has no filename"))?;
            Ok(canonical_resource_path(parent)?.join(name))
        }
        Err(error) => Err(sandbox_error(error.to_string())),
    }
}

fn pin_directory(path: &Path) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY)
        .open(path)
        .map_err(|error| sandbox_error(format!("failed to pin read-only directory: {error}")))
}

fn descriptor_path(directory: &File) -> PathBuf {
    PathBuf::from(format!("/dev/fd/{}", directory.as_raw_fd()))
}

fn validate_unaliased_tree(directory: &File) -> Result<()> {
    let mut pending = vec![Dir::from_std_file(
        directory
            .try_clone()
            .map_err(|error| sandbox_error(error.to_string()))?,
    )];
    let mut count = 0;
    while let Some(directory) = pending.pop() {
        for entry in directory
            .entries()
            .map_err(|error| sandbox_error(error.to_string()))?
        {
            let entry = entry.map_err(|error| sandbox_error(error.to_string()))?;
            count += 1;
            if count > 100_000 {
                return Err(sandbox_error(
                    "read-only export exceeds the 100000-entry alias validation bound",
                ));
            }
            let metadata = entry
                .metadata()
                .map_err(|error| sandbox_error(error.to_string()))?;
            if metadata.is_file() && metadata.nlink() != 1 {
                // shortcut: path-based Seatbelt cannot protect existing hard-link aliases; admit them only with a native per-export read-only file server.
                return Err(sandbox_error(
                    "read-only exports with hard-linked files are unsupported",
                ));
            }
            if metadata.is_dir() {
                pending.push(
                    entry
                        .open_dir()
                        .map_err(|error| sandbox_error(error.to_string()))?,
                );
            }
        }
    }
    Ok(())
}

fn overlaps(left: &Path, right: &Path) -> Result<bool> {
    if left.starts_with(right) || right.starts_with(left) {
        return Ok(true);
    }
    let identity = |path: &Path| -> Result<Option<(u64, u64)>> {
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some((metadata.dev(), metadata.ino()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(sandbox_error(error.to_string())),
        }
    };
    // APFS firmlinks can alias the same directory without canonicalizing alike.
    for (root, path) in [(identity(left)?, right), (identity(right)?, left)] {
        if let Some(root) = root {
            for ancestor in path.ancestors() {
                if identity(ancestor)? == Some(root) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "macos_readonly_tests.rs"]
mod tests;
