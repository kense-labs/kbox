use super::*;

#[test]
fn hvf_tmpfs_preflight_admits_bounded_intent_without_allocations() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let values = vec![
        "/scratch:size=1048576,mode=1777,rw".into(),
        "/sealed:size=4096,ro".into(),
    ];
    record.tmpfs = values.clone();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .tmpfs = values;
    let before = std::fs::read(store.path()).unwrap();
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_tmpfs_preflight_refuses_unqualified_input_before_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for values in [
        vec!["/scratch:size=0"],
        vec!["/scratch:size=513m"],
        vec!["/scratch:size=18446744073709551615G"],
        vec!["/scratch:mode=888"],
        vec!["/scratch:uid=-1"],
        vec!["/scratch:gid=4294967295"],
        vec!["/scratch:tmpcopyup"],
        vec!["/scratch:huge=never"],
        vec!["/scratch:remount"],
        vec!["/scratch:bind"],
        vec!["/scratch:ro,rw"],
        vec!["/scratch:size=1m,size=2m"],
        vec!["/scratch:noexec,exec"],
        vec!["/scratch:uid=+1"],
        vec!["/scratch:suid,nosuid"],
        vec!["/"],
        vec!["/run/./kbox/boot"],
        vec!["/proc"],
        vec!["/dev/shm"],
        vec!["/workspace"],
        vec!["/data"],
        vec!["/data/child"],
        vec!["/scratch", "/scratch/./"],
        vec!["/scratch", "/scratch/sub"],
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .tmpfs = values.iter().map(|v| v.to_string()).collect();
        assert!(
            matches!(
                validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
                Err(ExecutionManagerError::InvalidRequest(_))
            ),
            "{values:?}"
        );
    }
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .tmpfs = vec!["/scratch".into(); 129];
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_tmpfs_support_does_not_promote_other_profiles() {
    let (_temporary, _home, _store, mut record) = named_tests::fixture();
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.volumes.clear();
    config.tmpfs = vec!["/scratch:size=1m".into()];
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .volume_names
        .clear();
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, profile),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
}
