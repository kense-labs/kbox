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
    std::fs::write(path.join("ordinary file"), b"workspace edits remain live").unwrap();
    assert_eq!(first, key("engine-a:", spec.clone(), "bindings", 1));
    std::fs::rename(&path, directory.path().join("original")).unwrap();
    std::fs::create_dir(path).unwrap();
    assert_ne!(first, key("engine-a:", spec, "bindings", 1));
}
