use super::*;
use serde_json::json;

#[test]
fn failed_native_tool_preserves_program_status_and_service_context() {
    let error = read_plist(Command::new("/bin/sh").args([
        "-c",
        "printf 'DiskManagement service unavailable' >&2; exit 7",
    ]))
    .unwrap_err();

    assert_eq!(error.code, ErrorCode::FailedPrecondition);
    assert_eq!(
        error.operation.as_deref(),
        Some("verify-macos-memory-backing")
    );
    assert!(error.message.contains("/bin/sh"), "{error}");
    assert!(error.message.contains("exit status: 7"), "{error}");
    assert!(
        error.message.contains("DiskManagement service unavailable"),
        "{error}"
    );
}

#[test]
fn failed_native_tool_bounds_unicode_stderr_without_exposing_arguments() {
    let stderr = "界".repeat(2048);
    let error = read_plist(Command::new("/bin/sh").args([
        "-c",
        "printf '%s' \"$1\" >&2; exit 7",
        "unused-script-name",
        &stderr,
        "synthetic-private-command-argument",
    ]))
    .unwrap_err();

    assert_eq!(error.code, ErrorCode::FailedPrecondition);
    assert_eq!(error.message.matches('界').count(), 1024);
    assert!(error.message.len() < 4096);
    assert!(!error.message.contains("synthetic-private-command-argument"));
}

#[test]
fn memory_backing_requires_exact_ram_image_and_device() {
    let inventory = json!({"images":[
        {"image-path":"/private/disk.dmg","system-entities":[{"dev-entry":"/dev/disk1"}]},
        {"image-path":"ram://65536","system-entities":[{"dev-entry":"/dev/disk9"}]}
    ]});
    assert!(device_is_ram_image(&inventory, "/dev/disk9"));
    assert!(!device_is_ram_image(&inventory, "/dev/disk1"));
    assert!(!device_is_ram_image(&inventory, "/dev/disk90"));
    assert!(!device_is_ram_image(&json!({}), "/dev/disk9"));
    assert_eq!(
        parse_macos_mount_line("/dev/disk9 on /private/space 中文 (hfs, local)"),
        Some(("/dev/disk9", "/private/space 中文"))
    );
    assert_eq!(parse_macos_mount_line("invalid"), None);
}

#[test]
fn ordinary_disk_is_not_memory_backing() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(!macos_path_is_on_ram_disk(&temporary.path().canonicalize().unwrap()).unwrap());
}

#[test]
fn apfs_requires_every_actual_physical_store_to_be_ram() {
    let inventory = json!({"images":[{"image-path":"ram://1048576","system-entities":[{"dev-entry":"/dev/disk13"}]}]});
    let apfs = json!({"Containers":[{"ContainerReference":"disk14","PhysicalStores":[{"DeviceIdentifier":"disk13"}],"Volumes":[{"DeviceIdentifier":"disk14s1"}]}]});
    assert!(apfs_device_is_ram(&inventory, &apfs, "/dev/disk14s1"));
    assert!(!apfs_device_is_ram(&inventory, &apfs, "/dev/disk14s10"));
    assert!(!apfs_device_is_ram(&inventory, &apfs, "/dev/disk14"));
    for stores in [
        json!([]),
        json!([{"DeviceIdentifier":"disk0s2"}]),
        json!([{"DeviceIdentifier":"disk13"},{"DeviceIdentifier":"disk0s2"}]),
        json!([{}]),
    ] {
        let mut changed = apfs.clone();
        changed["Containers"][0]["PhysicalStores"] = stores;
        assert!(!apfs_device_is_ram(&inventory, &changed, "/dev/disk14s1"));
    }
    assert!(!apfs_device_is_ram(&inventory, &json!({}), "/dev/disk14s1"));
}
