//! Bind the existing OCI macOS host boundary to Box-owned VM exports.

use std::path::PathBuf;

use kbox_core::error::{BoxError, Result};
use kbox_core::vmm::{InstanceSpec, RootfsSource};
use koci_kbox::MacosReadOnlyMounts;

pub(super) fn prepare(mut spec: InstanceSpec) -> Result<(InstanceSpec, MacosReadOnlyMounts)> {
    if spec.fs_mounts.iter().any(|mount| mount.read_only)
        && (spec.snapshot_mem_file.is_some()
            || spec.snapshot_sock.is_some()
            || spec.restore_from.is_some())
    {
        return Err(BoxError::ConfigError(
            "macOS read-only exports with snapshot boot are unsupported".into(),
        ));
    }
    let mut writable = vec![spec.rootfs.path().to_path_buf()];
    if let RootfsSource::Ext4Disk {
        read_only: true, ..
    } = &spec.rootfs
    {
        writable.clear();
    }
    writable.extend(
        spec.block_devices
            .iter()
            .filter(|disk| !disk.read_only)
            .map(|disk| disk.path.clone()),
    );
    writable.extend(
        spec.fs_mounts
            .iter()
            .filter(|mount| !mount.read_only)
            .map(|mount| mount.host_path.clone()),
    );
    writable.extend(
        [
            &spec.exec_socket_path,
            &spec.pty_socket_path,
            &spec.attest_socket_path,
            &spec.port_forward_socket_path,
        ]
        .into_iter()
        .filter(|path| !path.as_os_str().is_empty())
        .cloned(),
    );
    if let Some(console) = &spec.console_output {
        writable.extend([
            console.clone(),
            console.with_file_name("console.err.log"),
            kbox_core::log::json_log_path(
                console
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            ),
        ]);
    }
    if spec.stdin_open {
        writable.push(spec.exec_socket_path.with_file_name("stdin.sock"));
    }
    if let Some(network) = &spec.network {
        writable.extend(network.net_stats_path.iter().cloned());
        writable.extend(network.bridge_socket_dir.iter().cloned());
    }
    let paths: Vec<_> = spec
        .fs_mounts
        .iter()
        .filter(|mount| mount.read_only)
        .map(|mount| mount.host_path.as_path())
        .collect();
    let writable: Vec<_> = writable.iter().map(PathBuf::as_path).collect();
    let read_only = MacosReadOnlyMounts::open(&paths, &writable).map_err(boundary_error)?;
    for (index, mount) in spec
        .fs_mounts
        .iter_mut()
        .filter(|mount| mount.read_only)
        .enumerate()
    {
        mount.host_path = read_only.pinned_path(index).ok_or_else(|| {
            BoxError::StateError("read-only export lost its pinned descriptor".into())
        })?;
    }
    Ok((spec, read_only))
}

pub(super) fn boundary_error(error: koci_sdk::Error) -> BoxError {
    BoxError::BoxBootError {
        message: format!("macOS host read-only boundary refused VM entry: {error}"),
        hint: None,
    }
}

#[cfg(test)]
#[path = "tests/macos_readonly.rs"]
mod tests;
