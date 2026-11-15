use super::*;

#[test]
fn hvf_nproc_ulimits_admit_capability_free_explicit_nonroot_without_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.user = Some("1001:2002".into());
    config.cap_drop = vec!["ALL".into()];
    let before = std::fs::read(store.path()).unwrap();
    for value in ["nproc=16:16", "nproc=0:0", "nproc=-1:-1"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![value.into()];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_sigpending_ulimits_admit_without_allocating_and_refuse_bad_identity() {
    queued_resource_admission("sigpending");
}

#[test]
fn hvf_msgqueue_ulimits_admit_without_allocating_and_refuse_bad_identity() {
    queued_resource_admission("msgqueue");
}

fn queued_resource_admission(resource: &str) {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.user = Some("1001:2002".into());
    config.cap_drop = vec!["ALL".into()];
    let before = std::fs::read(store.path()).unwrap();
    for value in ["16:32", "0:0", "-1:-1"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![format!("{resource}={value}")];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    let qualified = record
        .managed_execution
        .as_ref()
        .unwrap()
        .request
        .config
        .clone();
    for user in [
        None,
        Some("0"),
        Some("root"),
        Some("alice"),
        Some("1001:bad"),
        Some("4294967296"),
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .user = user.map(str::to_owned);
        assert!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err()
        );
    }
    record.managed_execution.as_mut().unwrap().request.config = qualified;
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_drop
        .clear();
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_nproc_ulimits_refuse_root_names_and_privilege_before_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.user = Some("1001:2002".into());
    config.cap_drop = vec!["ALL".into()];
    config.resource_limits.ulimits = vec!["nproc=16:16".into()];
    let qualified = config.clone();
    let before = std::fs::read(store.path()).unwrap();
    for user in [
        None,
        Some("0"),
        Some("root"),
        Some("alice"),
        Some("1001:bad"),
        Some("4294967296"),
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .user = user.map(str::to_owned);
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
    record.managed_execution.as_mut().unwrap().request.config = qualified.clone();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_drop
        .clear();
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_add = vec!["SYS_ADMIN".into()];
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    record.managed_execution.as_mut().unwrap().request.config = qualified;
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_add = vec!["SYS_ADMIN".into()];
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_nice_ulimits_admit_without_allocating_and_refuse_privilege_bypasses() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for value in ["nice=25:30", "nice=0:0", "nice=-1:-1"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![value.into()];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    for capability in ["SYS_NICE", "SYS_RESOURCE"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .cap_add = vec![capability.into()];
        assert!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err()
        );
    }
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_add
        .clear();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .privileged = true;
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_realtime_ulimits_admit_without_allocating_and_refuse_privilege_bypasses() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for value in [
        "rtprio=5:10",
        "rtprio=0:0",
        "rtprio=-1:-1",
        "rttime=20000:80000",
        "rttime=0:0",
        "rttime=-1:-1",
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![value.into()];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    for capability in ["SYS_NICE", "SYS_RESOURCE"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .cap_add = vec![capability.into()];
        assert!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err()
        );
    }
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .cap_add
        .clear();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .privileged = true;
    assert!(validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_memory_ulimits_admit_existing_byte_limits_without_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for values in [
        [
            "data=4194304:4194304",
            "stack=524288:524288",
            "memlock=65536:65536",
        ],
        ["data=0:0", "stack=0:0", "memlock=0:0"],
        ["data=-1:-1", "stack=-1:-1", "memlock=-1:-1"],
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = values.iter().map(|value| value.to_string()).collect();
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_cpu_time_ulimits_admit_existing_intent_without_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for value in ["cpu=1:2", "CPU=1:1", "cpu=1:-1", "cpu=-1:-1"] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = vec![value.into()];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_ulimits_admission_reuses_bounded_oci_process_controls() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .resource_limits
        .ulimits = vec![
        "nofile=64:128".into(),
        "fsize=4096:4096".into(),
        "core=0:0".into(),
    ];
    let before = std::fs::read(store.path()).unwrap();
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_ulimits_refuse_bad_or_unqualified_limits_before_allocating() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let before = std::fs::read(store.path()).unwrap();
    for values in [
        vec!["nofile=0:0"],
        vec!["nofile=64:-1"],
        vec!["nofile=1048577:1048577"],
        vec!["nofile=65:64"],
        vec!["fsize=1:2:3"],
        vec!["core=garbage"],
        vec!["cpu=0:0"],
        vec!["cpu=0:2"],
        vec!["cpu=2:1"],
        vec!["cpu=-1:2"],
        vec!["cpu=1:18446744073709551616"],
        vec!["cpu=1:2", "CPU=1:2"],
        vec!["data=2:1"],
        vec!["stack=1:18446744073709551616"],
        vec!["memlock=+1:2"],
        vec!["data=1:2", "DATA=1:2"],
        vec!["stack=1:2", "STACK=1:2"],
        vec!["memlock=1:2", "MEMLOCK=1:2"],
        vec!["rtprio=2:1"],
        vec!["rttime=2:1"],
        vec!["rtprio=1:2", "RTPRIO=1:2"],
        vec!["rttime=1:2", "RTTIME=1:2"],
        vec!["rtprio=1:18446744073709551616"],
        vec!["rttime=1:18446744073709551616"],
        vec!["nice=2:1"],
        vec!["nice=1:2", "NICE=1:2"],
        vec!["nice=1:18446744073709551616"],
        vec!["msgqueue=2:1"],
        vec!["msgqueue=1:2", "MSGQUEUE=1:2"],
        vec!["msgqueue=1:18446744073709551616"],
        vec!["msgqueue=1:1"],
        vec!["sigpending=2:1"],
        vec!["sigpending=1:2", "SIGPENDING=1:2"],
        vec!["sigpending=1:18446744073709551616"],
        vec!["sigpending=1:1"],
        vec!["rss=100:100"],
        vec!["nproc=1:2"],
        vec!["nofile=64:128", "NOFILE=128:128"],
    ] {
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .resource_limits
            .ulimits = values.iter().map(|value| value.to_string()).collect();
        assert!(
            matches!(
                validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
                Err(ExecutionManagerError::InvalidRequest(_))
            ),
            "{values:?}"
        );
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_ulimits_do_not_promote_other_profiles() {
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
            .resource_limits
            .ulimits
            .clear();
        validate_dedicated_vm_qualification(&record, profile).unwrap();
        for value in [
            "nofile=64:128",
            "cpu=1:2",
            "data=4194304:4194304",
            "stack=524288:524288",
            "memlock=65536:65536",
            "sigpending=16:16",
            "msgqueue=2048:2048",
            "nice=25:30",
            "rtprio=5:10",
            "rttime=20000:80000",
        ] {
            record
                .managed_execution
                .as_mut()
                .unwrap()
                .request
                .config
                .resource_limits
                .ulimits = vec![value.into()];
            assert!(validate_dedicated_vm_qualification(&record, profile).is_err());
        }
    }
}
