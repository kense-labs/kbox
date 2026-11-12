//! High-level context wrapper for libkrun interactions.
//!
//! Every function here forwards to the `libksys` C ABI. They are `unsafe`
//! because libkrun never re-validates the context handle it is given.
//!
//! # Safety
//!
//! The following invariants apply to every method in this module. Each method's
//! `# Safety` section names any additional requirement of its own:
//!
//! - `self` must hold a context id returned by [`KContext::create`] that has not
//!   been released. The id is a plain `u32`; libkrun cannot detect a stale,
//!   freed or forged one.
//! - Calls on one `KContext` must be serialized by the caller. libkrun keeps
//!   mutable configuration state behind that id and offers no per-context lock.
//! - Paths, strings, file descriptors and named pipes passed to a method must
//!   remain valid and open for as long as the VM uses them. The C side borrows
//!   them; it does not take ownership of guest-visible resources.
//! - [`KContext::init_logging`] mutates process-global libkrun state. Call it
//!   once before any context is created, never concurrently with setup.

use std::{ffi::CString, path::Path, ptr};

use super::check_status;
use kbox_core::error::{BoxError, Result};
use kbox_core::vmm::{RawBlockDevice, RootfsSource};
#[cfg(target_os = "macos")]
use libksys::kadd_net_unixgram;
#[cfg(not(target_os = "windows"))]
use libksys::kset_port_map;
#[cfg(not(target_os = "windows"))]
use libksys::{kadd_disk2, kadd_vsock_port2, kset_root_disk_remount, K_DISK_FORMAT_RAW};
#[cfg(target_os = "windows")]
use libksys::{kadd_net_tcp, kadd_vsock_port_windows, kset_kernel};
#[cfg(target_os = "linux")]
use libksys::{kadd_net_unixstream, ksplit_irqchip};
#[cfg(unix)]
use libksys::{kadd_virtio_console_default, kdisable_implicit_console};
use libksys::{
    kadd_virtiofs, kadd_vsock, kcreate_ctx, kdisable_implicit_vsock, kfree_ctx, kinit_log,
    kset_console_output, kset_env, kset_exec, kset_rlimits, kset_root, kset_vm_config,
    kset_workdir, ksetgid, ksetuid, kstart_enter,
};

fn value_to_cstring(value: &str, description: &str) -> Result<CString> {
    CString::new(value).map_err(|error| BoxError::BoxBootError {
        message: format!("invalid {description}: {error}"),
        hint: None,
    })
}

fn path_to_cstring(path: &Path, description: &str) -> Result<CString> {
    let value = path.to_str().ok_or_else(|| BoxError::BoxBootError {
        message: format!("invalid {description} path: {}", path.display()),
        hint: None,
    })?;
    value_to_cstring(value, description)
}

/// Thin wrapper that owns a libkrun context.
pub struct KContext {
    ctx_id: u32,
}

impl KContext {
    /// Initialize libkrun logging system based on RUST_LOG environment variable.
    ///
    /// # Safety
    ///
    /// Mutates process-global libkrun logging state. Call once before any
    /// [`KContext::create`] and never concurrently with context setup.
    /// Must be called before creating any context.
    pub unsafe fn init_logging() -> Result<()> {
        use libksys::{
            K_LOG_LEVEL_DEBUG, K_LOG_LEVEL_ERROR, K_LOG_LEVEL_INFO, K_LOG_LEVEL_TRACE,
            K_LOG_STYLE_AUTO, K_LOG_TARGET_STDERR,
        };

        // Determine log level from RUST_LOG environment variable
        let log_level = match std::env::var("RUST_LOG").as_deref() {
            Ok("trace") | Ok("kbox=trace") => {
                tracing::debug!("Initializing libkrun with TRACE log level");
                K_LOG_LEVEL_TRACE
            }
            Ok("debug") | Ok("kbox=debug") => {
                tracing::debug!("Initializing libkrun with DEBUG log level");
                K_LOG_LEVEL_DEBUG
            }
            Ok("info") | Ok("kbox=info") => {
                tracing::debug!("Initializing libkrun with INFO log level");
                K_LOG_LEVEL_INFO
            }
            _ => K_LOG_LEVEL_ERROR, // Default: only show errors
        };

        let log_target = K_LOG_TARGET_STDERR;
        let log_style = K_LOG_STYLE_AUTO;
        let flags = 0;

        tracing::trace!(log_target, log_level, log_style, flags, "Calling kinit_log");

        check_status(
            "kinit_log",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kinit_log(log_target, log_level, log_style, flags) },
        )
    }

    /// Create a new libkrun context.
    ///
    /// # Safety
    ///
    /// libkrun must already be initialized (see [`KContext::init_logging`]).
    /// The returned context is owned by this value and released once.
    pub unsafe fn create() -> Result<Self> {
        tracing::trace!("Calling kcreate_ctx()");
        // SAFETY: this function's # Safety contract covers this call.
        let ctx = unsafe { kcreate_ctx() };
        if ctx < 0 {
            tracing::error!(status = ctx, "kcreate_ctx failed");
            return Err(BoxError::BoxBootError {
                message: format!("kcreate_ctx failed with status {}", ctx),
                hint: Some("Ensure libkrun is properly installed".to_string()),
            });
        }
        tracing::trace!(ctx_id = ctx, "kcreate_ctx succeeded");
        Ok(Self { ctx_id: ctx as u32 })
    }

    /// Configure VM resources (vCPUs and memory).
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn set_vm_config(&self, cpus: u8, memory_mib: u32) -> Result<()> {
        tracing::debug!(cpus, memory_mib, "Setting VM config");
        check_status(
            "kset_vm_config",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kset_vm_config(self.ctx_id, cpus, memory_mib) },
        )
    }

    /// Configure either a virtio-fs directory root or a guest-native ext4 disk.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. The directory or disk image
    /// behind `rootfs` must exist and stay available for the VM's lifetime.
    pub unsafe fn set_root(&self, rootfs: &RootfsSource) -> Result<()> {
        tracing::debug!(rootfs = %rootfs, "Setting root filesystem source");
        match rootfs {
            RootfsSource::Directory { path } => {
                let rootfs_c = path_to_cstring(path, "rootfs directory")?;
                // SAFETY: this function's # Safety contract covers this call.
                check_status("kset_root", unsafe {
                    kset_root(self.ctx_id, rootfs_c.as_ptr())
                })
            }
            // SAFETY: this function's # Safety contract covers this call.
            RootfsSource::Ext4Disk { path, read_only } => unsafe {
                self.set_ext4_root_disk(path, *read_only)
            },
        }
    }

    /// Configure a raw ext4 image as the guest root disk.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. `path` must be an existing raw
    /// ext4 image that stays available and is never probed after guest use.
    #[cfg(not(target_os = "windows"))]
    unsafe fn set_ext4_root_disk(&self, path: &Path, read_only: bool) -> Result<()> {
        // Kcreated root images are always raw ext4 files with no partition
        // table. They are deliberately never probed after guest access: an
        // untrusted guest could rewrite the bytes as a linked image format.
        const ROOT_DISK_ID: &str = "rootfs";
        const ROOT_FILESYSTEM: &str = "ext4";

        let block_id = value_to_cstring(ROOT_DISK_ID, "root disk id")?;
        let disk_path = path_to_cstring(path, "ext4 root disk")?;
        check_status(
            "kadd_disk2(rootfs)",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kadd_disk2(
                    self.ctx_id,
                    block_id.as_ptr(),
                    disk_path.as_ptr(),
                    K_DISK_FORMAT_RAW,
                    read_only,
                )
            },
        )?;

        let device = value_to_cstring(kbox_core::vmm::GUEST_EXT4_ROOT_DEVICE, "root device")?;
        let filesystem = value_to_cstring(ROOT_FILESYSTEM, "root filesystem")?;
        let mount_options = read_only
            .then(|| value_to_cstring("ro", "root mount options"))
            .transpose()?;
        check_status(
            "kset_root_disk_remount",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kset_root_disk_remount(
                    self.ctx_id,
                    device.as_ptr(),
                    filesystem.as_ptr(),
                    mount_options
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                )
            },
        )
    }

    /// Attach a typed raw auxiliary disk without probing its guest-controlled
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. The referenced raw disk must
    /// exist and stay available; its guest-controlled bytes are never probed.
    /// contents. Root disks are configured first, so list order maps
    /// deterministically onto the remaining `/dev/vd*` device names.
    #[cfg(not(target_os = "windows"))]
    pub unsafe fn add_raw_block_device(&self, disk: &RawBlockDevice) -> Result<()> {
        let block_id = value_to_cstring(&disk.id, "raw block device id")?;
        let disk_path = path_to_cstring(&disk.path, "raw block device")?;
        check_status(
            "kadd_disk2(auxiliary)",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kadd_disk2(
                    self.ctx_id,
                    block_id.as_ptr(),
                    disk_path.as_ptr(),
                    K_DISK_FORMAT_RAW,
                    disk.read_only,
                )
            },
        )
    }

    /// Reject raw auxiliary block devices on the Windows libkrun backend.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. This stub performs no FFI call;
    /// it keeps the `unsafe` signature shared with the Unix implementation.
    #[cfg(target_os = "windows")]
    pub unsafe fn add_raw_block_device(&self, disk: &RawBlockDevice) -> Result<()> {
        Err(BoxError::BoxBootError {
            message: format!(
                "Raw auxiliary block devices are not supported by the Windows libkrun backend: {}",
                disk.path.display()
            ),
            hint: None,
        })
    }

    /// Reject guest-native ext4 root disks on the Windows libkrun backend.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. This stub performs no FFI call;
    /// it keeps the `unsafe` signature shared with the Unix implementation.
    #[cfg(target_os = "windows")]
    unsafe fn set_ext4_root_disk(&self, path: &Path, _read_only: bool) -> Result<()> {
        Err(BoxError::BoxBootError {
            message: format!(
                "guest-native ext4 root disks are not supported by the Windows libkrun backend: {}",
                path.display()
            ),
            hint: Some("Use a directory rootfs on Windows".to_string()),
        })
    }

    /// Set the executable to run inside the VM.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn set_exec(
        &self,
        exec: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<()> {
        let exec_c = CString::new(exec).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid exec path: {}", e),
            hint: None,
        })?;

        tracing::trace!(exec, args_count = args.len(), "Building argv array");
        for (i, arg) in args.iter().enumerate() {
            tracing::trace!(index = i, arg = ?arg, "Entrypoint argument");
        }

        let arg_storage: Vec<CString> = args
            .iter()
            .map(|arg| {
                CString::new(arg.as_str()).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid arg: {}", e),
                    hint: None,
                })
            })
            .collect::<Result<_>>()?;
        let mut arg_ptrs: Vec<*const std::ffi::c_char> =
            arg_storage.iter().map(|arg| arg.as_ptr()).collect();
        arg_ptrs.push(ptr::null());

        tracing::trace!(env_count = env.len(), "Building env array");
        for (k, v) in env.iter() {
            tracing::trace!(key = k, value = v, "Environment variable");
        }

        let env_storage = Self::env_to_cstring(env)?;
        let mut env_ptrs: Vec<*const std::ffi::c_char> =
            env_storage.iter().map(|entry| entry.as_ptr()).collect();
        env_ptrs.push(ptr::null());

        check_status(
            "kset_exec",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kset_exec(
                    self.ctx_id,
                    exec_c.as_ptr(),
                    arg_ptrs.as_ptr(),
                    env_ptrs.as_ptr(),
                )
            },
        )
    }

    /// Set environment variables for the VM.
    ///
    /// Kept as a wrapper for libkrun completeness, but normal boot flow passes
    /// environment through `set_exec` so guest-init metadata is not overwritten.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[allow(dead_code)]
    pub unsafe fn set_env(&self, env: &[(String, String)]) -> Result<()> {
        if env.is_empty() {
            let empty: [*const std::ffi::c_char; 1] = [ptr::null()];
            // SAFETY: this function's # Safety contract covers this call.
            return check_status("kset_env", unsafe { kset_env(self.ctx_id, empty.as_ptr()) });
        }

        let env_storage = Self::env_to_cstring(env)?;
        let mut ptrs: Vec<*const std::ffi::c_char> =
            env_storage.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());

        // SAFETY: this function's # Safety contract covers this call.
        check_status("kset_env", unsafe { kset_env(self.ctx_id, ptrs.as_ptr()) })
    }

    fn env_to_cstring(env: &[(String, String)]) -> Result<Vec<CString>> {
        env.iter()
            .map(|(k, v)| {
                CString::new(format!("{}={}", k, v)).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid env: {}", e),
                    hint: None,
                })
            })
            .collect()
    }

    /// Set the working directory inside the VM.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn set_workdir(&self, workdir: &str) -> Result<()> {
        let workdir_c = CString::new(workdir).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid workdir path: {}", e),
            hint: None,
        })?;
        check_status(
            "kset_workdir",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kset_workdir(self.ctx_id, workdir_c.as_ptr()) },
        )
    }

    /// Set resource limits for the VM.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn set_rlimits(&self, rlimits: &[String]) -> Result<()> {
        tracing::trace!(rlimits = ?rlimits, "Setting rlimits");
        if rlimits.is_empty() {
            let empty: [*const std::ffi::c_char; 1] = [ptr::null()];
            // SAFETY: this function's # Safety contract covers this call.
            return check_status("kset_rlimits", unsafe {
                kset_rlimits(self.ctx_id, empty.as_ptr())
            });
        }

        let entries: Vec<CString> = rlimits
            .iter()
            .map(|rlimit| {
                CString::new(rlimit.as_str()).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid rlimit: {}", e),
                    hint: None,
                })
            })
            .collect::<Result<_>>()?;
        let mut ptrs: Vec<*const std::ffi::c_char> = entries.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());

        // SAFETY: this function's # Safety contract covers this call.
        check_status("kset_rlimits", unsafe {
            kset_rlimits(self.ctx_id, ptrs.as_ptr())
        })
    }

    /// Add a virtiofs mount, sharing a host directory with the guest.
    ///
    /// # Arguments
    /// * `mount_tag` - Tag used by guest to mount this share (e.g., "workspace", "vol0")
    /// * `host_path` - Path to directory on host to share
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn add_virtiofs(&self, mount_tag: &str, host_path: &str) -> Result<()> {
        tracing::debug!(mount_tag, host_path, "Adding virtiofs mount");

        let host_path_c = CString::new(host_path).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid host path: {}", e),
            hint: None,
        })?;
        let mount_tag_c = CString::new(mount_tag).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid mount tag: {}", e),
            hint: None,
        })?;

        check_status(
            "kadd_virtiofs",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kadd_virtiofs(self.ctx_id, mount_tag_c.as_ptr(), host_path_c.as_ptr()) },
        )
    }

    /// Configure vsock port with Unix socket bridge.
    ///
    /// # Arguments
    /// * `port` - Guest vsock port number
    /// * `socket_path` - Host Unix socket path for the bridge
    /// * `listen` - If true, libkrun creates the socket and listens (host connects).
    ///   If false, libkrun connects to an existing socket (host listens).
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. The bound socket path must be
    /// created and owned by the caller before the VM starts.
    #[cfg(not(target_os = "windows"))]
    pub unsafe fn add_vsock_port(&self, port: u32, socket_path: &str, listen: bool) -> Result<()> {
        tracing::debug!(port, socket_path, listen, "Configuring vsock port");
        let socket_path_c = CString::new(socket_path).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid socket path: {}", e),
            hint: None,
        })?;
        check_status(
            "kadd_vsock_port2",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kadd_vsock_port2(self.ctx_id, port, socket_path_c.as_ptr(), listen) },
        )
    }

    /// Retain the explicit vsock device used for Box IPC without enabling TSI.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn disable_tsi(&self) -> Result<()> {
        check_status(
            "kdisable_implicit_vsock",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kdisable_implicit_vsock(self.ctx_id) },
        )?;
        // SAFETY: this function's # Safety contract covers this call.
        check_status("kadd_vsock", unsafe { kadd_vsock(self.ctx_id, 0) })
    }

    /// Configure TSI port mappings for the VM.
    ///
    /// Maps host ports to guest ports via TSI (Transparent Socket Impersonation).
    /// When a guest process listens on a guest port, it becomes accessible on the
    /// corresponding host port.
    ///
    /// # Arguments
    /// * `port_map` - Slice of "host_port:guest_port" strings (e.g., ["8080:80", "3000:3000"])
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(not(target_os = "windows"))]
    #[allow(dead_code)]
    pub unsafe fn set_port_map(&self, port_map: &[String]) -> Result<()> {
        tracing::debug!(port_map = ?port_map, "Setting TSI port mappings");
        let entries: Vec<CString> = port_map
            .iter()
            .map(|entry| {
                CString::new(entry.as_str()).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid port map entry: {}", e),
                    hint: None,
                })
            })
            .collect::<Result<_>>()?;
        let mut ptrs: Vec<*const std::ffi::c_char> = entries.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());

        // SAFETY: this function's # Safety contract covers this call.
        check_status("kset_port_map", unsafe {
            kset_port_map(self.ctx_id, ptrs.as_ptr())
        })
    }

    /// Add a virtio-net device connected to an inherited passt-compatible
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. `fd` must be an open socket
    /// owned by the caller and must not be closed while the VM uses it.
    /// Unix stream descriptor. Path-only attach is refused: it would skip
    /// the egress proxy and connect libkrun straight to passt.
    #[cfg(target_os = "linux")]
    pub unsafe fn add_net_unixstream_fd(&self, fd: i32, mac: &[u8; 6]) -> Result<()> {
        tracing::debug!(fd, mac = ?mac, "Adding virtio-net via inherited passt stream fd");

        let features: u32 = (1 << 0) | (1 << 1) | (1 << 7) | (1 << 10) | (1 << 11) | (1 << 14);
        check_status(
            "kadd_net_unixstream",
            kadd_net_unixstream(self.ctx_id, ptr::null(), fd, mac.as_ptr(), features, 0),
        )
    }

    /// Add a virtio-net device connected to an inherited Unix datagram fd (macOS).
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. `fd` must be an open socket
    /// owned by the caller and must not be closed while the VM uses it.
    /// Path-only attach is refused: it would skip the egress proxy.
    #[cfg(target_os = "macos")]
    pub unsafe fn add_net_unixgram_fd(&self, fd: i32, mac: &[u8; 6]) -> Result<()> {
        tracing::debug!(fd, mac = ?mac, "Adding virtio-net via inherited unixgram fd");

        let features: u32 = 0;

        check_status(
            "kadd_net_unixgram",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kadd_net_unixgram(self.ctx_id, ptr::null(), fd, mac.as_ptr(), features, 0) },
        )
    }

    /// Set the user ID for the VM process.
    ///
    /// The UID is applied right before the microVM starts.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(not(target_os = "windows"))]
    pub unsafe fn set_uid(&self, uid: libc::uid_t) -> Result<()> {
        tracing::debug!(uid, "Setting VM uid");
        // SAFETY: this function's # Safety contract covers this call.
        check_status("ksetuid", unsafe { ksetuid(self.ctx_id, uid) })
    }

    /// Set the user ID for the VM process (Windows).
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(target_os = "windows")]
    pub unsafe fn set_uid(&self, uid: u32) -> Result<()> {
        tracing::debug!(uid, "Setting VM uid");
        // SAFETY: this function's # Safety contract covers this call.
        check_status("ksetuid", unsafe { ksetuid(self.ctx_id, uid) })
    }

    /// Set the group ID for the VM process.
    ///
    /// The GID is applied right before the microVM starts.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(not(target_os = "windows"))]
    pub unsafe fn set_gid(&self, gid: libc::gid_t) -> Result<()> {
        tracing::debug!(gid, "Setting VM gid");
        // SAFETY: this function's # Safety contract covers this call.
        check_status("ksetgid", unsafe { ksetgid(self.ctx_id, gid) })
    }

    /// Set the group ID for the VM process (Windows).
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(target_os = "windows")]
    pub unsafe fn set_gid(&self, gid: u32) -> Result<()> {
        tracing::debug!(gid, "Setting VM gid");
        // SAFETY: this function's # Safety contract covers this call.
        check_status("ksetgid", unsafe { ksetgid(self.ctx_id, gid) })
    }

    /// Redirect VM console output to a file.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    pub unsafe fn set_console_output(&self, filepath: &str) -> Result<()> {
        tracing::debug!(filepath, "Setting console output path");
        let filepath_c = CString::new(filepath).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid console output path: {}", e),
            hint: None,
        })?;
        check_status(
            "kset_console_output",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kset_console_output(self.ctx_id, filepath_c.as_ptr()) },
        )
    }

    /// Replace the implicit console with a virtio-console whose stdout and
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    /// stderr go to SEPARATE host fds (so the guest's stderr can be tagged).
    /// `input_fd` may be -1 (no stdin). Caller owns the fds for the VM lifetime.
    #[cfg(unix)]
    pub unsafe fn add_split_console(
        &self,
        input_fd: i32,
        output_fd: i32,
        err_fd: i32,
    ) -> Result<()> {
        check_status(
            "kdisable_implicit_console",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kdisable_implicit_console(self.ctx_id) },
        )?;
        check_status(
            "kadd_virtio_console_default",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kadd_virtio_console_default(self.ctx_id, input_fd, output_fd, err_fd) },
        )
    }

    /// Enable split IRQ chip mode (required for TEE VMs).
    ///
    /// This must be called before starting a TEE-enabled VM.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(target_os = "linux")]
    pub unsafe fn enable_split_irqchip(&self) -> Result<()> {
        tracing::debug!("Enabling split IRQ chip for TEE");
        let ret = ksplit_irqchip(self.ctx_id, true);
        if ret < 0 {
            return Err(BoxError::TeeConfig(format!(
                "Failed to enable split IRQ chip: error code {}",
                ret
            )));
        }
        Ok(())
    }

    /// Set the TEE configuration file path.
    ///
    /// This configures the VM to run in a Trusted Execution Environment
    /// using the specified configuration file (JSON format).
    ///
    /// # Arguments
    /// * `config_path` - Path to the TEE configuration JSON file
    ///
    /// # Note
    /// This function is only available on Linux when libkrun is built with SEV support.
    /// Call `enable_split_irqchip()` before this function.
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. `config_path` must reference
    /// a readable TEE configuration file for the duration of the call.
    #[cfg(target_os = "linux")]
    pub unsafe fn set_tee_config(&self, config_path: &str) -> Result<()> {
        tracing::debug!(config_path, "Setting TEE configuration file");
        let path_c = CString::new(config_path)
            .map_err(|e| BoxError::TeeConfig(format!("Invalid TEE config path: {}", e)))?;
        let ret = libksys::kset_tee_config_file(self.ctx_id, path_c.as_ptr());
        if ret < 0 {
            return Err(BoxError::TeeConfig(format!(
                "Failed to set TEE config file '{}': error code {}",
                config_path, ret
            )));
        }
        Ok(())
    }

    /// Set the kernel image for the microVM (required on Windows).
    ///
    /// On Windows, this **must** be called before `start_enter()`.
    ///
    /// # Arguments
    /// * `kernel_path` - Path to the kernel image file
    /// * `kernel_format` - One of the `K_KERNEL_FORMAT_*` constants
    /// * `initramfs` - Optional path to initramfs image
    /// * `cmdline` - Optional kernel command line string
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. The kernel image and format
    /// must match the guest architecture configured for this context.
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub unsafe fn set_kernel(
        &self,
        kernel_path: &str,
        kernel_format: u32,
        initramfs: Option<&str>,
        cmdline: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(kernel_path, kernel_format, "Setting kernel");
        let kernel_c = CString::new(kernel_path).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid kernel path: {}", e),
            hint: None,
        })?;
        let initramfs_c = initramfs
            .map(|s| {
                CString::new(s).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid initramfs path: {}", e),
                    hint: None,
                })
            })
            .transpose()?;
        let cmdline_c = cmdline
            .map(|s| {
                CString::new(s).map_err(|e| BoxError::BoxBootError {
                    message: format!("invalid cmdline: {}", e),
                    hint: None,
                })
            })
            .transpose()?;
        check_status(
            "kset_kernel",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kset_kernel(
                    self.ctx_id,
                    kernel_c.as_ptr(),
                    kernel_format,
                    initramfs_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                    cmdline_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                )
            },
        )
    }

    /// Add a virtio-net device backed by a TCP connection (Windows only).
    ///
    /// # Arguments
    /// * `iface_id` - Interface identifier string
    /// * `mac` - MAC address as 6 bytes
    /// * `tcp_addr` - Optional `"host:port"` string for the TCP backend; `None` for disconnected
    ///
    /// # Safety
    ///
    /// See the module-level safety contract: `self` must hold a live context,
    /// calls on it must be serialized, and borrowed arguments must stay valid
    /// for the duration of the call.
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub unsafe fn add_net_tcp(
        &self,
        iface_id: &str,
        mac: &[u8; 6],
        tcp_addr: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(iface_id, tcp_addr, "Adding virtio-net via TCP");
        let iface_c = CString::new(iface_id)
            .map_err(|e| BoxError::NetworkError(format!("invalid iface_id: {}", e)))?;
        let tcp_c = tcp_addr
            .map(|s| {
                CString::new(s)
                    .map_err(|e| BoxError::NetworkError(format!("invalid tcp_addr: {}", e)))
            })
            .transpose()?;
        check_status(
            "kadd_net_tcp",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe {
                kadd_net_tcp(
                    self.ctx_id,
                    iface_c.as_ptr(),
                    mac.as_ptr(),
                    tcp_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                )
            },
        )
    }

    /// Map a guest vsock port to a Windows Named Pipe (Windows only).
    ///
    /// # Arguments
    /// * `port` - Guest vsock port number
    /// * `pipe_name` - Named Pipe name (e.g., `"myservice"` → `\\.\pipe\myservice`)
    ///
    /// # Safety
    ///
    /// See the module-level safety contract. The named pipe must be created
    /// and owned by the caller before the VM starts.
    #[cfg(target_os = "windows")]
    pub unsafe fn add_vsock_port_windows(&self, port: u32, pipe_name: &str) -> Result<()> {
        tracing::debug!(port, pipe_name, "Adding vsock port via Named Pipe");
        let pipe_c = CString::new(pipe_name).map_err(|e| BoxError::BoxBootError {
            message: format!("invalid pipe name: {}", e),
            hint: None,
        })?;
        check_status(
            "kadd_vsock_port_windows",
            // SAFETY: this function's # Safety contract covers this call.
            unsafe { kadd_vsock_port_windows(self.ctx_id, port, pipe_c.as_ptr()) },
        )
    }

    /// Start the VM and enter it (process takeover).
    ///
    /// # Returns
    /// * On success: Never returns (process becomes VM)
    /// * On failure: Returns negative error code
    /// * On guest exit: Returns the guest's exit status (non-negative)
    ///
    /// # Safety
    ///
    /// This function performs process takeover - on success, it never returns.
    /// The current process becomes the VM. This should only be called from
    /// a subprocess (shim) to isolate the VM from the host application.
    pub unsafe fn start_enter(&self) -> i32 {
        tracing::trace!(ctx_id = self.ctx_id, "Calling kstart_enter");
        // SAFETY: this function's # Safety contract covers this call.
        let status = unsafe { kstart_enter(self.ctx_id) };
        tracing::trace!(status, "kstart_enter returned");
        if status < 0 {
            tracing::error!(status, "kstart_enter failed");
        }
        status
    }
}

impl Drop for KContext {
    fn drop(&mut self) {
        // SAFETY: `ctx_id` came from `KContext::create` and has not been freed.
        // `KContext` derives neither `Clone` nor `Copy`, and `kfree_ctx` is
        // called only from this `Drop`, so each id is released exactly once.
        // All C calls on this context happen on the same thread beforehand; on
        // Unix `kstart_enter` takes over the process, and on Windows it returns
        // only after guest shutdown, so no call is in flight here. The C ABI
        // consumes only the scalar id and retains no pointer.
        unsafe {
            let _ = kfree_ctx(self.ctx_id);
        }
    }
}
