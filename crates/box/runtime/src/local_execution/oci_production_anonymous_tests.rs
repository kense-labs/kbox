use super::*;

#[tokio::test]
async fn hvf_anonymous_plan_reserves_normalized_image_volumes_before_materialization() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    record.managed_execution.as_mut().unwrap().runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    std::fs::create_dir_all(&record.box_dir).unwrap();
    let image: crate::oci::OciImageConfig = kbox_core::SnapshotImageConfig {
        volumes: vec!["/data".into(), "/state".into(), "/state/./".into()],
        ..Default::default()
    }
    .into();
    crate::resolved_image::persist_resolved_image_config(&record.box_dir, &image).unwrap();
    let provider = MacosHvfOciBundleProvider::new(&home, home.join("runtime"));
    let manager = provider
        .inner
        .preparer
        .new_oci_preparation_manager(&record)
        .unwrap();
    let expected = manager.plan_anonymous_volumes(&image).unwrap();
    assert_eq!(
        expected.len(),
        1,
        "explicit /data bind overrides the image declaration"
    );
    assert_eq!(expected[0].guest_path, "/state");
    let catalog = std::fs::read(store.path()).unwrap();
    let plan = provider
        .plan_create_resources(&record, &koci_sdk::AttachmentCapabilities::base_v2())
        .await
        .unwrap();
    assert_eq!(plan.anonymous_volumes, vec![expected[0].name.clone()]);
    assert_eq!(std::fs::read(store.path()).unwrap(), catalog);
    assert!(!home.join("volumes").join(&expected[0].name).exists());
    plan.apply_to(&mut record).unwrap();
    assert_eq!(record.anonymous_volumes, vec![expected[0].name.clone()]);
}

#[tokio::test]
async fn hvf_anonymous_plan_refuses_excess_inventory_before_claiming_allocations() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    record.managed_execution.as_mut().unwrap().runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    std::fs::create_dir_all(&record.box_dir).unwrap();
    let image: crate::oci::OciImageConfig = kbox_core::SnapshotImageConfig {
        volumes: (0..126).map(|index| format!("/anon-{index}")).collect(),
        ..Default::default()
    }
    .into();
    crate::resolved_image::persist_resolved_image_config(&record.box_dir, &image).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let provider = MacosHvfOciBundleProvider::new(&home, home.join("runtime"));
    assert!(provider
        .plan_create_resources(&record, &koci_sdk::AttachmentCapabilities::base_v2())
        .await
        .is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_eq!(std::fs::read_dir(home.join("volumes")).unwrap().count(), 1);
}

#[tokio::test]
async fn hvf_raw_secret_plan_requires_combined_capability_before_claiming_allocations() {
    let (_temporary, home, store, mut record) = named_tests::fixture();
    let metadata = record.managed_execution.as_mut().unwrap();
    metadata.runtime_route = crate::ManagedRuntimeRoute::OciSdk;
    metadata.request.policy.managed_secret_root =
        Some(home.join("preflight-validated-secret-root"));
    std::fs::create_dir_all(&record.box_dir).unwrap();
    let image: crate::oci::OciImageConfig = kbox_core::SnapshotImageConfig {
        volumes: vec!["/state".into()],
        ..Default::default()
    }
    .into();
    crate::resolved_image::persist_resolved_image_config(&record.box_dir, &image).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let provider = MacosHvfOciBundleProvider::new(&home, home.join("runtime"));
    for (versions, raw, allowed) in [
        (vec![1, 2], true, false),
        (vec![3], false, false),
        (vec![3], true, true),
    ] {
        let mut capabilities = koci_sdk::AttachmentCapabilities::base_v2()
            .with_extension(koci_sdk::MEMORY_SECRET_FILES_EXTENSION, versions.clone())
            .unwrap();
        if raw {
            capabilities = capabilities
                .with_extension(koci_sdk::RAW_STORAGE_IMAGES_EXTENSION, vec![1])
                .unwrap();
        }
        let result = provider.plan_create_resources(&record, &capabilities).await;
        if allowed {
            assert_eq!(result.unwrap().anonymous_volumes.len(), 1);
        } else {
            assert!(matches!(
                result,
                Err(ExecutionManagerError::InvalidRequest(_))
            ));
        }
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert_eq!(std::fs::read_dir(home.join("volumes")).unwrap().count(), 1);
        assert!(record.anonymous_volumes.is_empty());
    }
}
