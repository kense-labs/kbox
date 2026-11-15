use super::*;
use crate::engine::{EngineDirectoryGrant, EngineKey};

#[test]
fn compatibility_is_independent_of_instance_and_fences_live_directory_replacement() {
    let engine = EngineKey::parse("engine").unwrap();
    let intent = crate::engine::tests::intent(&engine, 1, DesiredState::Up);
    let spec = intent.bind_service("first".into(), 1).unwrap();
    let mut other = spec.clone();
    other.unit_id = "second".into();
    other.generation = 2;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap().join("外部 files");
    std::fs::create_dir(&path).unwrap();
    let grant = EngineDirectoryGrant::new(path.clone(), false).unwrap();
    let key = |owner, spec, bindings, asset: u8| {
        compatible_scope(owner, spec, bindings, vec![grant.clone()], &[asset; 32]).unwrap()
    };
    let first = key("engine-a:", spec.clone(), "bindings", 1);
    assert_eq!(first, key("engine-a:", other, "bindings", 1));
    assert_ne!(first, key("engine-b:", spec.clone(), "bindings", 1));
    assert_ne!(first, key("engine-a:", spec.clone(), "other-bindings", 1));
    assert_ne!(first, key("engine-a:", spec.clone(), "bindings", 2));
    let mut different = spec.clone();
    different.resources.memory_bytes *= 2;
    assert_ne!(first, key("engine-a:", different, "bindings", 1));
    let mut different = spec.clone();
    different.semantics_profile_digest =
        Some(crate::kbox_contract_driver::NATIVE_GUEST_PROFILE_DIGEST.into());
    assert_ne!(first, key("engine-a:", different, "bindings", 1));
    std::fs::write(path.join("ordinary file"), b"workspace edits remain live").unwrap();
    assert_eq!(first, key("engine-a:", spec.clone(), "bindings", 1));
    std::fs::rename(&path, directory.path().join("original")).unwrap();
    std::fs::create_dir(path).unwrap();
    assert_ne!(first, key("engine-a:", spec, "bindings", 1));
}

#[test]
fn prewarm_is_an_explicit_clean_service_option_independent_of_guest_profile() {
    use kbox_contract::contract::RestartPolicy;
    let mut request =
        crate::engine::tests::intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    let service = request.service.as_mut().unwrap();
    service.process.command = vec!["/bin/sleep".into()];
    service.process.args = vec!["infinity".into()];
    service.process.environment.clear();
    service.health = None;
    service.restart = RestartPolicy::Never;
    assert!(crate::kbox_contract_driver::prewarm::eligible(
        &request.bind_service("idle-service".into(), 1).unwrap()
    ));
    request.prewarm_bindings_digest = Some(format!("sha256:{}", "2".repeat(64)));
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert!(encoded.get("prewarmBindingsDigest").is_some());
    assert_eq!(
        serde_json::from_value::<super::super::EngineIntent>(encoded).unwrap(),
        request
    );
    for digest in ["", "sha256:1", "sha256:../escape"] {
        let mut invalid = request.clone();
        invalid.prewarm_bindings_digest = Some(digest.into());
        assert!(invalid.validate().is_err());
    }
    let mut invalid = request.clone();
    invalid
        .service
        .as_mut()
        .unwrap()
        .process
        .environment
        .insert("TOKEN".into(), "synthetic".into());
    assert!(invalid.validate().is_err());
    invalid = request.clone();
    invalid.service.as_mut().unwrap().process.command = vec!["/bin/application".into()];
    assert!(invalid.validate().is_err());
    invalid = request;
    invalid.desired_state = DesiredState::Down;
    invalid.service = None;
    assert!(invalid.validate().is_err());
}
