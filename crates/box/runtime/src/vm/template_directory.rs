//! Actual owned snapshot files and consumer-close proof; no image-hash deletion.

use cap_fs_ext::{DirExt, MetadataExt};
use cap_std::fs::Dir;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug)]
pub(crate) struct TemplateDirectory {
    path: PathBuf,
    directory: Dir,
    unproven_consumer: AtomicBool,
    removed: AtomicBool,
}

impl TemplateDirectory {
    /// Synchronous artifact I/O; the pool invokes this on a blocking worker.
    pub(crate) fn create(parent: &Path) -> io::Result<Arc<Self>> {
        std::fs::create_dir_all(parent)?;
        let mut builder = tempfile::Builder::new();
        builder.prefix("tpl-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let temporary = builder.tempdir_in(parent)?;
        let path = temporary.path().canonicalize()?;
        let directory = Dir::open_ambient_dir(&path, cap_std::ambient_authority())?;
        // Cleanup needs consumer-close proof, including when a builder fails.
        let _allocated = temporary.keep();
        Ok(Arc::new(Self {
            path,
            directory,
            unproven_consumer: AtomicBool::new(false),
            removed: AtomicBool::new(false),
        }))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn claim(self: &Arc<Self>) -> io::Result<TemplateUse> {
        self.validate_identity()?;
        Ok(TemplateUse {
            directory: Arc::clone(self),
            closed: false,
        })
    }

    /// Synchronous removal refuses unknown consumers and replaced namespaces.
    #[cfg(test)]
    pub(crate) fn remove(self: &Arc<Self>) -> io::Result<()> {
        if let Some(directory) = self.exclusive_directory()? {
            self.finish_removal(directory)?;
        }
        Ok(())
    }

    /// The pool serializes admission while rewriting or removing this directory.
    pub(crate) fn exclusive_directory(self: &Arc<Self>) -> io::Result<Option<Dir>> {
        if self.removed.load(Ordering::Acquire) {
            return Ok(None);
        }
        if self.unproven_consumer.load(Ordering::Acquire) || Arc::strong_count(self) != 1 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "snapshot template has an unfinalized consumer",
            ));
        }
        self.validate_identity()?;
        self.directory.try_clone().map(Some)
    }

    pub(crate) fn finish_removal(&self, directory: Dir) -> io::Result<()> {
        directory.remove_open_dir_all()?;
        // APFS can retain st_nlink=2 on an unlinked open directory.
        self.removed.store(true, Ordering::Release);
        Ok(())
    }

    fn validate_identity(&self) -> io::Result<()> {
        if self.removed.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "snapshot directory was finalized",
            ));
        }
        let owned = self.directory.dir_metadata()?;
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "snapshot directory has no parent",
            )
        })?;
        let name = self.path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "snapshot directory has no name")
        })?;
        let parent = Dir::open_ambient_dir(parent, cap_std::ambient_authority())?;
        let current = parent.open_dir_nofollow(name)?.dir_metadata()?;
        if (current.dev(), current.ino()) != (owned.dev(), owned.ino()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "snapshot directory identity changed",
            ));
        }
        for name in ["template.ram", "template.state"] {
            match self.directory.symlink_metadata(name) {
                Ok(metadata) if !metadata.is_file() || metadata.nlink() != 1 => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "snapshot input is not a private single-link file",
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TemplateUse {
    directory: Arc<TemplateDirectory>,
    closed: bool,
}

impl TemplateUse {
    /// A workload grant must never expose its own host snapshot artifacts.
    pub(crate) fn validate_grant(&self, source: &Path) -> io::Result<()> {
        self.directory.validate_identity()?;
        let source = source.canonicalize()?;
        if source.starts_with(self.directory.path()) || self.directory.path().starts_with(&source) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "workload mount overlaps private snapshot artifacts",
            ));
        }
        Ok(())
    }

    pub(crate) fn closed(mut self) {
        self.closed = true;
    }
}

impl Drop for TemplateUse {
    fn drop(&mut self) {
        // Dropping a manager is not proof that its backend released the mapping.
        if !self.closed {
            self.directory
                .unproven_consumer
                .store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
#[path = "template_directory_tests.rs"]
mod tests;
