use super::*;
use oci_spec::runtime::Capability;

fn compiled(config: &kbox_core::BoxConfig) -> oci_spec::runtime::LinuxCapabilities {
    crate::vm::oci_hvf_spec::workload_capabilities(config).unwrap()
}

#[test]
fn hvf_cap_add_preserves_shared_default_and_explicit_sets() {
    let mut config = kbox_core::BoxConfig::default();
    let defaults = compiled(&config);
    assert_eq!(defaults.bounding().as_ref().unwrap().len(), 10);
    config.cap_add = vec!["cap_mknod".into(), " SETFCAP ".into(), "AUDIT_WRITE".into()];
    let actual = compiled(&config);
    let mut expected = defaults.bounding().clone().unwrap();
    expected.extend([
        Capability::Mknod,
        Capability::Setfcap,
        Capability::AuditWrite,
    ]);
    assert_eq!(expected.len(), 13);
    for set in [actual.bounding(), actual.effective(), actual.permitted()] {
        assert_eq!(set.as_ref().unwrap(), &expected);
    }
    assert!(actual.inheritable().as_ref().unwrap().is_empty());
    assert!(actual.ambient().as_ref().unwrap().is_empty());
    config.cap_add.push("CAP_SYS_ADMIN".into());
    assert!(compiled(&config)
        .effective()
        .as_ref()
        .unwrap()
        .contains(&Capability::SysAdmin));
}

#[test]
fn hvf_cap_add_retains_drop_precedence_and_unknown_refusal() {
    let mut config = kbox_core::BoxConfig {
        cap_add: vec![
            "MKNOD".into(),
            "SETFCAP".into(),
            "AUDIT_WRITE".into(),
            "SYS_ADMIN".into(),
        ],
        ..Default::default()
    };
    config.cap_drop = vec![
        "cap_mknod".into(),
        "SETFCAP".into(),
        "AUDIT_WRITE".into(),
        "SYS_ADMIN".into(),
    ];
    let actual = compiled(&config);
    assert_eq!(actual.bounding().as_ref().unwrap().len(), 10);
    for capability in [
        Capability::Mknod,
        Capability::Setfcap,
        Capability::AuditWrite,
        Capability::SysAdmin,
    ] {
        assert!(
            !actual.effective().as_ref().unwrap().contains(&capability),
            "dropped {capability:?}"
        );
    }
    config.cap_drop = vec!["aLl".into()];
    let actual = compiled(&config);
    for set in [
        actual.bounding(),
        actual.effective(),
        actual.permitted(),
        actual.inheritable(),
        actual.ambient(),
    ] {
        assert!(set.as_ref().unwrap().is_empty());
    }
    for unknown in [
        "NET_ADMIN",
        "NET_RAW",
        "SYS_RESOURCE",
        "ALL",
        "FUTURE_CAPABILITY",
    ] {
        config.cap_add = vec![unknown.into()];
        assert!(
            crate::vm::oci_hvf_spec::workload_capabilities(&config).is_err(),
            "{unknown}"
        );
    }
}

#[test]
fn hvf_cap_add_admission_is_readonly_and_other_profiles_stay_closed() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let config = &mut record.managed_execution.as_mut().unwrap().request.config;
    config.cap_add = vec![
        "CHOWN".into(),
        "MKNOD".into(),
        "SETFCAP".into(),
        "AUDIT_WRITE".into(),
    ];
    let before = std::fs::read(store.path()).unwrap();
    validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
    let request = &mut record.managed_execution.as_mut().unwrap().request;
    request.config.volumes.clear();
    request.policy.volume_names.clear();
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, profile),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}

#[test]
fn hvf_cap_add_preserves_per_uid_unprivileged_admission() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let base = &mut record.managed_execution.as_mut().unwrap().request.config;
    base.user = Some("1001:2002".into());
    base.cap_add = vec![
        "MKNOD".into(),
        "SETFCAP".into(),
        "AUDIT_WRITE".into(),
        "SYS_ADMIN".into(),
    ];
    let before = std::fs::read(store.path()).unwrap();
    for resource in ["nproc", "sigpending", "msgqueue"] {
        let config = &mut record.managed_execution.as_mut().unwrap().request.config;
        config.resource_limits.ulimits = vec![format!("{resource}=16:16")];
        config.cap_drop.clear();
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        record
            .managed_execution
            .as_mut()
            .unwrap()
            .request
            .config
            .cap_drop = vec!["ALL".into()];
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
    }
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!home.join("boxes").exists());
}
