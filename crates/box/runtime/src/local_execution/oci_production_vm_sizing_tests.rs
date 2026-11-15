use super::*;

fn set_resources(record: &mut BoxRecord, cpus: u32, memory: u32) {
    record.cpus = cpus;
    record.memory_mb = memory;
    let resources = &mut record
        .managed_execution
        .as_mut()
        .unwrap()
        .request
        .config
        .resources;
    resources.vcpus = cpus;
    resources.memory_mb = memory;
}

#[test]
fn hvf_vm_resources_admit_profile_values_without_source_or_allocation_mutation() {
    for (cpus, memory) in [(1, 128), (1, 512), (2, 768), (4, 1024), (8, 8192)] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        set_resources(&mut record, cpus, memory);
        let before = std::fs::read(store.path()).unwrap();
        validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf).unwrap();
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[test]
fn hvf_vm_resources_reject_invalid_values_without_truncation_or_allocation() {
    for (cpus, memory) in [
        (0, 512),
        (3, 512),
        (9, 512),
        (256, 512),
        (u32::MAX, 512),
        (1, 0),
        (1, 127),
        (1, 129),
        (1, 8193),
        (1, u32::MAX),
    ] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        set_resources(&mut record, cpus, memory);
        let before = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::InvalidRequest(_))
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[test]
fn hvf_vm_resources_refuse_persisted_count_drift_from_the_immutable_request() {
    for dimension in ["cpu", "memory"] {
        let (_temporary, home, store, mut record) = named_tests::fixture();
        set_resources(&mut record, 2, 768);
        match dimension {
            "cpu" => record.cpus = 1,
            "memory" => record.memory_mb = 512,
            _ => unreachable!(),
        }
        let before = std::fs::read(store.path()).unwrap();
        assert!(matches!(
            validate_dedicated_vm_qualification(&record, DedicatedVmOciProfile::MacosHvf),
            Err(ExecutionManagerError::Internal(_))
        ));
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert!(!home.join("boxes").exists());
    }
}

#[tokio::test]
async fn hvf_vm_resources_require_host_capability_before_planning_allocations() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    set_resources(&mut record, 2, 768);
    record.managed_execution.as_mut().unwrap().runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    std::fs::create_dir_all(&record.box_dir).unwrap();
    let image: crate::oci::OciImageConfig = kbox_core::SnapshotImageConfig {
        volumes: vec!["/state".into()],
        ..Default::default()
    }
    .into();
    crate::resolved_image::persist_resolved_image_config(&record.box_dir, &image).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let provider = MacosHvfOciBundleProvider::new(&home, home.join("runtime"));
    let legacy = koci_sdk::AttachmentCapabilities::base_v2();
    for versions in [None, Some(vec![2])] {
        let capabilities = match versions {
            Some(versions) => legacy
                .clone()
                .with_extension(koci_sdk::VM_RESOURCES_EXTENSION, versions)
                .unwrap(),
            None => legacy.clone(),
        };
        let result = provider.plan_create_resources(&record, &capabilities).await;
        assert!(
            matches!(result,Err(ExecutionManagerError::InvalidRequest(ref reason)) if reason.contains(koci_sdk::VM_RESOURCES_EXTENSION)),
            "{result:?}"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert_eq!(std::fs::read_dir(home.join("volumes")).unwrap().count(), 1);
    }
    let capable = legacy
        .with_extension(
            koci_sdk::VM_RESOURCES_EXTENSION,
            vec![koci_sdk::VM_RESOURCES_VERSION],
        )
        .unwrap();
    assert_eq!(
        provider
            .plan_create_resources(&record, &capable)
            .await
            .unwrap()
            .anonymous_volumes
            .len(),
        1
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_eq!(std::fs::read_dir(home.join("volumes")).unwrap().count(), 1);
}

#[test]
fn vm_sizing_does_not_promote_other_platform_qualification_profiles() {
    for profile in [
        DedicatedVmOciProfile::LinuxKvm,
        DedicatedVmOciProfile::WindowsWhpx,
    ] {
        for (cpus, memory) in [(2, 512), (1, 1024)] {
            let (_temporary, _home, _store, mut record) = named_tests::fixture();
            set_resources(&mut record, cpus, memory);
            assert!(matches!(
                validate_dedicated_vm_qualification(&record, profile),
                Err(ExecutionManagerError::InvalidRequest(_))
            ));
        }
    }
}
