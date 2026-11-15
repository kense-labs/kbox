use super::*;

#[test]
fn hvf_custom_disk_capacity_admits_existing_builder_bounds_without_mutation() {
    for mib in [16, 128, 8192, 65536] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resources
            .disk_mb = mib;
        let before = std::fs::read(store.path()).unwrap();
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[test]
fn hvf_custom_disk_capacity_refuses_out_of_bounds_and_other_controls() {
    for mib in [0, 1, 15, 65537, u32::MAX] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resources
            .disk_mb = mib;
        let before = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
    for control in ["timeout", "ephemeral", "cpu", "memory"] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        let resources = &mut record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resources;
        resources.disk_mb = 128;
        match control {
            "timeout" => resources.timeout += 1,
            "ephemeral" => resources.ephemeral_storage_bytes = Some(1024),
            "cpu" => resources.vcpus = 2,
            "memory" => resources.memory_mb = 1024,
            _ => unreachable!(),
        }
        let before = std::fs::read(store.path()).unwrap();
        let result = validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf);
        if matches!(control, "cpu" | "memory") {
            // These values are now qualified, but the stored counts still name1/512.
            assert!(matches!(result, Err(ExecutionManagerError::Internal(_))));
        } else {
            assert!(matches!(
                result,
                Err(ExecutionManagerError::InvalidRequest(_))
            ));
        }
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[test]
fn custom_disk_capacity_does_not_promote_other_profiles() {
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        let request = &mut record.managed_execution.as_mut().unwrap().request;
        request.config.volumes.clear();
        request.policy.volume_names.clear();
        validate_dedicated_vm_qualification(&record, profile).unwrap();
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resources
            .disk_mb = 128;
        let before = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, profile),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}
