use super::*;

#[test]
fn hvf_root_readonly_admission_reuses_supported_guest_control_without_allocations() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    record.read_only = true;
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.read_only = true;
    config.cap_drop = vec!["ALL".into()];
    config.tmpfs = vec!["/scratch:size=1m,mode=1777,rw".into()];
    let before = std::fs::read(store.path()).unwrap();
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_root_readonly_qualification_does_not_promote_other_profiles() {
    let (_temporary, _home, _store, mut record) = named_tests::fixture();
    let request = &mut record.managed_execution.as_mut().unwrap().request;
    request.config.volumes.clear();
    request.policy.volume_names.clear();
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .read_only = false;
        validate_dedicated_vm_qualification(&record, profile).unwrap();
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .read_only = true;
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, profile),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
}
