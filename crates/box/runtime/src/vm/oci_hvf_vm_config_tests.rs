use super::*;

#[test]
fn vm_resource_projection_preserves_legacy_and_binds_custom_intent() {
    for (cpus, memory) in [(1, 512), (1, 128), (2, 768), (8, 8192)] {
        let mut spec = Spec::default();
        let requested = koci_sdk::VmConfig::new(cpus, memory).unwrap();
        complete_vm_resources(&mut spec, requested).unwrap();
        let bundle = koci_sdk::OciBundle::from_spec(
            std::env::temp_dir().join("vm-resource-projection"),
            spec,
        )
        .unwrap();
        let actual = koci_sdk::VmConfig::from_bundle(&bundle).unwrap();
        assert_eq!(
            actual,
            if requested == koci_sdk::VmConfig::default() {
                None
            } else {
                Some(requested)
            }
        );
        let attachments =
            koci_sdk::CreateAttachments::from_bundle(&bundle, koci_sdk::ProcessIo::default())
                .unwrap();
        assert_eq!(attachments.vm_config(&bundle).unwrap(), actual);
    }
}

#[test]
fn unowned_annotation_cannot_override_box_resource_authority() {
    for (cpus, memory) in [(1, 512), (2, 768)] {
        let mut spec = Spec::default();
        spec.annotations_mut().get_or_insert_default().insert(
            koci_sdk::VM_RESOURCES_EXTENSION.into(),
            koci_sdk::VmConfig::new(8, 8192)
                .unwrap()
                .annotation_value()
                .unwrap(),
        );
        let before = serde_json::to_value(&spec).unwrap();
        assert!(
            complete_vm_resources(&mut spec, koci_sdk::VmConfig::new(cpus, memory).unwrap())
                .is_err()
        );
        assert_eq!(serde_json::to_value(&spec).unwrap(), before);
    }
}
