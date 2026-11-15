//! Mount restrictions are checked on pinned entries inside the admitted root.
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

use nix::mount::{mount, MsFlags};

fn fd_path(file: &File) -> String {
    format!("/proc/self/fd/{}", file.as_raw_fd())
}

fn pin(root: &File, path: &str) -> io::Result<File> {
    let mut file = root.try_clone()?;
    let mut parts = Path::new(path)
        .components()
        .filter_map(|part| {
            if let Component::Normal(name) = part {
                Some(name)
            } else {
                None
            }
        })
        .peekable();
    while let Some(name) = parts.next() {
        file = OpenOptions::new()
            .read(true)
            .custom_flags(
                libc::O_PATH
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | if parts.peek().is_some() {
                        libc::O_DIRECTORY
                    } else {
                        0
                    },
            )
            .open(PathBuf::from(fd_path(&file)).join(name))?;
        if file.metadata()?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "symlink in restricted path",
            ));
        }
    }
    Ok(file)
}

fn bind(source: &File, target: &File, recursive: bool) -> io::Result<()> {
    let flags = libc::OPEN_TREE_CLONE
        | libc::OPEN_TREE_CLOEXEC
        | (libc::AT_EMPTY_PATH | if recursive { libc::AT_RECURSIVE } else { 0 }) as u32;
    // SAFETY: source and the empty C string remain live; successful open_tree returns a new owned FD.
    let descriptor =
        unsafe { libc::syscall(libc::SYS_open_tree, source.as_raw_fd(), c"".as_ptr(), flags) };
    if descriptor < 0 {
        let error = io::Error::last_os_error();
        tracing::debug!(%error, "restricted mount syscall failed");
        return Err(error);
    }
    // SAFETY: the nonnegative descriptor is newly returned by open_tree and has no other owner.
    let detached = unsafe { File::from_raw_fd(descriptor as i32) };
    // SAFETY: both descriptors and empty C strings remain live; only the pinned target receives the mount.
    let result = unsafe {
        libc::syscall(
            libc::SYS_move_mount,
            detached.as_raw_fd(),
            c"".as_ptr(),
            target.as_raw_fd(),
            c"".as_ptr(),
            libc::MOVE_MOUNT_F_EMPTY_PATH | libc::MOVE_MOUNT_T_EMPTY_PATH,
        )
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        tracing::debug!(%error, "restricted move_mount failed");
        return Err(error);
    }
    Ok(())
}

fn mount_id(file: &File) -> io::Result<u64> {
    std::fs::read_to_string(format!("/proc/self/fdinfo/{}", file.as_raw_fd()))?
        .lines()
        .find_map(|line| {
            line.strip_prefix("mnt_id:")
                .and_then(|v| v.trim().parse().ok())
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "missing restricted path mount identity",
            )
        })
}

fn readonly(file: &File) -> io::Result<bool> {
    Ok(nix::sys::statvfs::statvfs(fd_path(file).as_str())?
        .flags()
        .contains(nix::sys::statvfs::FsFlags::ST_RDONLY))
}

const MASK_SOURCE: &str = "kbox-path-mask";
const FILE_MASK_SOURCE: &str = "kbox-file-mask";

fn owns_mask(file: &File, source: &str) -> io::Result<bool> {
    let id = mount_id(file)?;
    Ok(std::fs::read_to_string("/proc/self/mountinfo")?
        .lines()
        .any(|line| {
            let Some((mount, filesystem)) = line.split_once(" - ") else {
                return false;
            };
            let mut fields = filesystem.split_whitespace();
            mount
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                == Some(id)
                && fields.next() == Some("tmpfs")
                && fields.next() == Some(source)
        }))
}

fn set_readonly(file: &File) -> io::Result<()> {
    let attributes = libc::mount_attr {
        attr_set: libc::MOUNT_ATTR_RDONLY,
        attr_clr: 0,
        propagation: 0,
        userns_fd: 0,
    };
    // SAFETY: file and the empty C string remain live; attributes has the kernel ABI layout/size.
    let result = unsafe {
        libc::syscall(
            libc::SYS_mount_setattr,
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_RECURSIVE,
            &attributes,
            std::mem::size_of::<libc::mount_attr>(),
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

struct MaskFileSource {
    directory: PathBuf,
    mount: File,
    file: File,
    detached: std::cell::Cell<bool>,
    original_directory: (u64, u64),
}

impl MaskFileSource {
    fn open() -> io::Result<Self> {
        let base = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/dev/shm")?;
        if nix::sys::statfs::statfs(fd_path(&base).as_str())?.filesystem_type()
            != nix::sys::statfs::TMPFS_MAGIC
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "masked file staging requires Guest tmpfs",
            ));
        }
        let mut template = b"/dev/shm/.kbox-mask-XXXXXX\0".to_vec();
        // SAFETY: template is a writable NUL-terminated mkdtemp template with six trailing X bytes.
        if unsafe { libc::mkdtemp(template.as_mut_ptr().cast()) }.is_null() {
            return Err(io::Error::last_os_error());
        }
        let directory = PathBuf::from(
            std::str::from_utf8(&template[..template.len() - 1]).map_err(io::Error::other)?,
        );
        let outer = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&directory)?;
        let metadata = outer.metadata()?;
        let original_directory = (metadata.dev(), metadata.ino());
        if let Err(error) = mount(
            Some(FILE_MASK_SOURCE),
            fd_path(&outer).as_str(),
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            Some("size=4k,nr_inodes=2,mode=0700"),
        ) {
            let _ = std::fs::remove_dir(&directory);
            return Err(error.into());
        }
        let prepared = (|| {
            let mounted = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&directory)?;
            if !owns_mask(&mounted, FILE_MASK_SOURCE)? {
                return Err(io::Error::other("masked file source mount changed"));
            }
            drop(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o000)
                    .open(PathBuf::from(fd_path(&mounted)).join("source"))?,
            );
            let file = pin(&mounted, "/source")?;
            set_readonly(&mounted)?;
            if !masked_file(&file)? {
                return Err(io::Error::other("invalid private masked file source"));
            }
            Ok(Self {
                directory: directory.clone(),
                mount: mounted,
                file,
                detached: std::cell::Cell::new(false),
                original_directory,
            })
        })();
        if prepared.is_err() {
            let _ = nix::mount::umount2(
                &directory,
                nix::mount::MntFlags::MNT_DETACH | nix::mount::MntFlags::UMOUNT_NOFOLLOW,
            );
            let _ = std::fs::remove_dir(&directory);
        }
        prepared
    }

    fn cleanup(&self) -> io::Result<()> {
        if self.detached.get() {
            return Ok(());
        }
        nix::mount::umount2(
            fd_path(&self.mount).as_str(),
            nix::mount::MntFlags::MNT_DETACH,
        )?;
        self.detached.set(true);
        let metadata = std::fs::symlink_metadata(&self.directory)?;
        if (metadata.dev(), metadata.ino()) != self.original_directory {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "masked file staging directory replaced",
            ));
        }
        std::fs::remove_dir(&self.directory)
    }
}

impl Drop for MaskFileSource {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            tracing::warn!(%error, "masked file source cleanup failed");
        }
    }
}

fn masked_file(file: &File) -> io::Result<bool> {
    let metadata = file.metadata()?;
    Ok(metadata.is_file()
        && metadata.len() == 0
        && metadata.mode() & 0o777 == 0
        && readonly(file)?
        && owns_mask(file, FILE_MASK_SOURCE)?)
}

fn masked_directory(file: &File) -> io::Result<bool> {
    Ok(owns_mask(file, MASK_SOURCE)?
        && readonly(file)?
        && nix::sys::statfs::statfs(fd_path(file).as_str())?.filesystem_type()
            == nix::sys::statfs::TMPFS_MAGIC
        && std::fs::read_dir(fd_path(file))?
            .next()
            .transpose()?
            .is_none())
}

/// Apply existing CRI masks/readonly paths, refusing setup errors before launch.
/// Only absent targets are skipped; symlink components are explicitly unsupported.
pub(crate) fn apply_container_path_restrictions(
    rootfs: &str,
    masked: &[&str],
    read_only: &[&str],
) -> io::Result<()> {
    if masked.len().saturating_add(read_only.len()) > 128
        || masked.iter().chain(read_only).any(|path| {
            !path.starts_with('/')
                || path.trim_matches('/').is_empty()
                || path.len() > 4096
                || path.contains('\0')
                || path.split('/').any(|part| part == "." || part == "..")
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid or excessive path restrictions",
        ));
    }
    let root = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(rootfs)?;
    for path in masked {
        let target = match pin(&root, path) {
            Ok(target) => target,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if target.metadata()?.is_dir() {
            if masked_directory(&target)? {
                continue;
            }
            mount(
                Some(MASK_SOURCE),
                fd_path(&target).as_str(),
                Some("tmpfs"),
                MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
                Some("size=0k"),
            )?;
            if !masked_directory(&pin(&root, path)?)? {
                return Err(io::Error::other("directory mask verification failed"));
            }
        } else {
            if masked_file(&target)? {
                continue;
            }
            // A private empty tmpfs file remains linked until attachment; unlinked /dev/null roots cannot be cloned.
            let source = MaskFileSource::open()?;
            let applied = (|| {
                let expected = source.file.metadata()?;
                bind(&source.file, &target, false)?;
                let actual = pin(&root, path)?;
                let metadata = actual.metadata()?;
                if (metadata.dev(), metadata.ino()) != (expected.dev(), expected.ino())
                    || !masked_file(&actual)?
                {
                    return Err(io::Error::other("file mask verification failed"));
                }
                Ok(())
            })();
            let cleaned = source.cleanup();
            applied.and(cleaned)?;
        }
    }
    for path in read_only {
        let mut target = match pin(&root, path) {
            Ok(target) => target,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let expected = target.metadata()?;
        if !expected.is_file() && !expected.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "readonly path must be a regular file or directory",
            ));
        }
        let parent = Path::new(path)
            .parent()
            .and_then(Path::to_str)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "restricted path has no parent")
            })?;
        if mount_id(&target)? == mount_id(&pin(&root, parent)?)? {
            bind(&target, &target, true)?;
            target = pin(&root, path)?;
        }
        let rebound = target.metadata()?;
        let parent = pin(&root, parent)?;
        if (rebound.dev(), rebound.ino()) != (expected.dev(), expected.ino())
            || mount_id(&target)? == mount_id(&parent)?
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "readonly path binding changed before enforcement",
            ));
        }
        set_readonly(&target)?;
        let actual = pin(&root, path)?;
        let metadata = actual.metadata()?;
        if (metadata.dev(), metadata.ino()) != (expected.dev(), expected.ino())
            || !readonly(&actual)?
        {
            return Err(io::Error::other("readonly path verification failed"));
        }
    }
    Ok(())
}
