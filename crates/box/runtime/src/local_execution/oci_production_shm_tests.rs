use super::*;

#[test]
fn hvf_shm_size_admits_bounded_policy_without_claims() {
    for bytes in [4096, 1024 * 1024, 512 * 1024 * 1024] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        record.shm_size = Some(bytes);
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .policy
            .shm_size = Some(bytes);
        let before = std::fs::read(store.path()).unwrap();
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[test]
fn hvf_shm_size_rejects_bad_sizes_and_conflicting_tmpfs_before_claims() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for bytes in [0, 1, 4095, 4097, 512 * 1024 * 1024 + 4096, u64::MAX] {
        record.shm_size = Some(bytes);
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .policy
            .shm_size = Some(bytes);
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
    record.shm_size = Some(1024 * 1024);
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.request.policy.shm_size = record.shm_size;
    for value in [
        "/dev/shm:size=1m",
        "/dev/./shm:size=2m",
        "/dev/shm/child:size=1m",
        "/dev:size=1m",
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .tmpfs = vec![value.into()];
        assert!(
            matches!(
                validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
                Err(ExecutionManagerError::InvalidRequest(_))
            ),
            "{value}"
        );
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_shm_size_only_tmpfs_admission_preserves_reserved_boundaries() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .tmpfs = vec!["/dev/shm:size=2m".into()];
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    for value in [
        "/dev/shm",
        "/dev/shm:mode=1777",
        "/dev/shm:size=1m,exec",
        "/dev/shm:size=1m,ro",
        "/dev/shm:size=1m,uid=1001",
        "/dev/shm:size=1m,size=2m",
        "/dev/shm:size=4097",
        "/dev/shm/sub:size=1m",
        "/dev/pts:size=1m",
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .tmpfs = vec![value.into()];
        assert!(
            matches!(
                validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
                Err(ExecutionManagerError::InvalidRequest(_))
            ),
            "{value}"
        );
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn shm_size_does_not_promote_other_profiles_or_oom_overrides() {
    let (_temporary, _home, _store, mut record) = named_tests::fixture();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .volumes
        .clear();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .volume_names
        .clear();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .shm_size = Some(1024 * 1024);
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, profile),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .oom_kill_disable = true;
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .oom_kill_disable = false;
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .policy
        .oom_score_adj = Some(100);
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
}
