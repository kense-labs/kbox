use super::*;
use serde_json::json;

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
