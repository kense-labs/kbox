//! Shared actual RAM-image provenance for Box and the native execution boundary.
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{Error, ErrorCode, Result};

/// Verify the longest actual mount against hdiutil's ram:// device inventory.
pub fn macos_path_is_on_ram_disk(path: &Path) -> Result<bool> {
    let table = Command::new("/sbin/mount").output().map_err(io_error)?;
    if !table.status.success() {
        return Err(failure("could not inspect actual macOS mounts"));
    }
    let table = String::from_utf8_lossy(&table.stdout);
    let device = table
        .lines()
        .filter_map(parse_macos_mount_line)
        .filter(|(_, point)| path == Path::new(point) || path.starts_with(point))
        .max_by_key(|(_, point)| point.len())
        .map(|(device, _)| device);
    let Some(device) = device else {
        return Ok(false);
    };
    let inventory = read_plist(Command::new("/usr/bin/hdiutil").args(["info", "-plist"]))?;
    if device_is_ram_image(&inventory, device) {
        return Ok(true);
    }
    let apfs = read_plist(Command::new("/usr/sbin/diskutil").args(["apfs", "list", "-plist"]))?;
    Ok(apfs_device_is_ram(&inventory, &apfs, device))
}

/// Parse a native mount record without treating spaces in its mountpoint as separators.
pub fn parse_macos_mount_line(line: &str) -> Option<(&str, &str)> {
    let (device, rest) = line.split_once(" on ")?;
    let (point, _) = rest.rsplit_once(" (")?;
    (!device.is_empty() && !point.is_empty()).then_some((device, point))
}

fn device_is_ram_image(inventory: &serde_json::Value, device: &str) -> bool {
    inventory
        .get("images")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|images| {
            images.iter().any(|image| {
                image
                    .get("image-path")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|path| path.starts_with("ram://"))
                    && image
                        .get("system-entities")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|entities| {
                            entities.iter().any(|entity| {
                                entity.get("dev-entry").and_then(serde_json::Value::as_str)
                                    == Some(device)
                            })
                        })
            })
        })
}
fn apfs_device_is_ram(
    inventory: &serde_json::Value,
    apfs: &serde_json::Value,
    device: &str,
) -> bool {
    let Some(identifier) = device.strip_prefix("/dev/") else {
        return false;
    };
    apfs.get("Containers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|containers| {
            containers.iter().any(|container| {
                let exact = container
                    .get("Volumes")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|volumes| {
                        volumes.iter().any(|volume| {
                            volume
                                .get("DeviceIdentifier")
                                .and_then(serde_json::Value::as_str)
                                == Some(identifier)
                        })
                    });
                exact
                    && container
                        .get("PhysicalStores")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|stores| {
                            !stores.is_empty()
                                && stores.iter().all(|store| {
                                    store
                                        .get("DeviceIdentifier")
                                        .and_then(serde_json::Value::as_str)
                                        .is_some_and(|physical| {
                                            device_is_ram_image(
                                                inventory,
                                                &format!("/dev/{physical}"),
                                            )
                                        })
                                })
                        })
            })
        })
}

fn read_plist(command: &mut Command) -> Result<serde_json::Value> {
    let plist = command.output().map_err(io_error)?;
    if !plist.status.success() {
        // Retain the OS cause without dumping unbounded output or command arguments.
        let stderr = &plist.stderr[..plist.stderr.len().min(4096)];
        let stderr = String::from_utf8_lossy(stderr)
            .chars()
            .take(1024)
            .collect::<String>();
        return Err(failure(&format!(
            "could not inspect actual macOS disk-image backing: {} returned {}: {}",
            command.get_program().to_string_lossy(),
            plist.status,
            stderr.trim()
        )));
    }
    let mut convert = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(io_error)?;
    let write = convert
        .stdin
        .take()
        .ok_or_else(|| failure("mount conversion stdin unavailable"))?
        .write_all(&plist.stdout);
    let json = convert.wait_with_output().map_err(io_error)?;
    write.map_err(io_error)?;
    if !json.status.success() {
        return Err(failure("could not decode macOS disk-image identities"));
    }
    serde_json::from_slice(&json.stdout).map_err(|_| failure("invalid macOS disk-image inventory"))
}

fn failure(message: &str) -> Error {
    Error::new(ErrorCode::FailedPrecondition, message).for_operation("verify-macos-memory-backing")
}
fn io_error(error: std::io::Error) -> Error {
    failure(&format!("macOS memory-backing inspection failed: {error}"))
}

#[cfg(test)]
#[path = "macos_memory_tests.rs"]
mod tests;
