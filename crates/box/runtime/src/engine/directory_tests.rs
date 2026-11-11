use super::*;
use crate::engine::{DesiredState, EngineKey};

fn intent() -> EngineIntent {
    let mut intent =
        crate::engine::tests::intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    let grant =
        EngineDirectoryGrant::new(PathBuf::from("/workspace external/中文"), false).unwrap();
    intent.service.as_mut().unwrap().mounts = vec![RuntimeMount {
        name: "workspace".into(),
        source: RuntimeMountSource::Directory {
            reference: grant.reference.clone(),
        },
        target: "/work".into(),
        read_only: false,
    }];
    intent.directories = vec![grant];
    intent
}

#[test]
fn grants_bind_exact_paths_and_permissions_and_leave_legacy_wire_unchanged() {
    let request = intent();
    request.validate().unwrap();
    let mut oci = request.clone();
    oci.workload = crate::engine::EngineWorkload::OciService;
    let bound = oci.bind_service("ordinary-oci".into(), 7).unwrap();
    assert!(bound.semantics_profile_digest.is_none());
    assert_eq!(bound.mounts, request.service.as_ref().unwrap().mounts);
    let encoded = serde_json::to_value(&request).unwrap();
    let decoded: EngineIntent = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, request);
    let mut invalid = request.clone();
    invalid.directories.clear();
    assert!(invalid.validate().is_err());
    invalid = request.clone();
    invalid.directories[0].source = "/different".into();
    assert!(invalid.validate().is_err());
    invalid = request.clone();
    invalid.directories[0].read_only = true;
    assert!(invalid.validate().is_err());
    invalid = request.clone();
    invalid.directories.push(invalid.directories[0].clone());
    assert!(invalid.validate().is_err());
    invalid = request.clone();
    invalid.service.as_mut().unwrap().mounts.clear();
    assert!(invalid.validate().is_err());
    let legacy =
        crate::engine::tests::intent(&EngineKey::parse("engine").unwrap(), 1, DesiredState::Up);
    let encoded = serde_json::to_value(&legacy).unwrap();
    assert!(encoded.get("directories").is_none());
    assert_eq!(
        serde_json::from_value::<EngineIntent>(encoded).unwrap(),
        legacy
    );
}

#[tokio::test]
async fn frozen_grants_resolve_exact_spec_through_replace_restart_and_missing_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("intent.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let mut request = intent();
    request.engine_id = journal.engine_id().clone();
    let unit = journal.unit_id(&request.instance_id);
    journal.accept(request.clone()).unwrap();
    let first = request.bind_service(unit.clone(), 1).unwrap();
    let mut next = request.clone();
    next.request_id = EngineKey::parse("replacement").unwrap();
    next.revision = 2;
    next.directories[0] = EngineDirectoryGrant::new(PathBuf::from("/replacement"), false).unwrap();
    next.service.as_mut().unwrap().mounts[0].source = RuntimeMountSource::Directory {
        reference: next.directories[0].reference.clone(),
    };
    journal.accept(next.clone()).unwrap();
    drop(journal);
    let port = EngineDirectories(Arc::new(Mutex::new(EngineJournal::open(path).unwrap())));
    assert_eq!(
        port.mount_path(&first, &first.mounts[0]).await.unwrap(),
        request.directories[0].source
    );
    let second = next.bind_service(unit.clone(), 2).unwrap();
    assert_eq!(
        port.mount_path(&second, &second.mounts[0]).await.unwrap(),
        PathBuf::from("/replacement")
    );
    let mut wrong = first.clone();
    wrong.unit_id = "wrong".into();
    assert!(port.mount_path(&wrong, &first.mounts[0]).await.is_err());
    wrong = first.clone();
    wrong.generation = 2;
    assert!(port.mount_path(&wrong, &first.mounts[0]).await.is_err());
    wrong = first.clone();
    wrong.resources.pids -= 1;
    assert!(port.mount_path(&wrong, &first.mounts[0]).await.is_err());
    port.cleanup_spec("unused").await.unwrap();
    assert_eq!(
        port.mount_path(&first, &first.mounts[0]).await.unwrap(),
        request.directories[0].source
    );
}

#[tokio::test]
async fn previous_attempts_keep_teardown_authority_without_granting_future_generations() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = EngineJournal::open(root.path().join("intent.json")).unwrap();
    let mut request = intent();
    request.engine_id = journal.engine_id().clone();
    let unit = journal.unit_id(&request.instance_id);
    journal.accept(request.clone()).unwrap();
    let mut next = journal.data.clone();
    let record = next.requests.get_mut(&request.request_id).unwrap();
    record.generation = 3;
    record.attempt = 3;
    record.runtime_request_id = format!("engine-{}-g3-a3", request.request_id.as_str());
    journal.publish(next).unwrap();
    let port = EngineDirectories(Arc::new(Mutex::new(journal)));
    for generation in 1..=3 {
        let spec = request.bind_service(unit.clone(), generation).unwrap();
        assert_eq!(
            port.mount_path(&spec, &spec.mounts[0]).await.unwrap(),
            request.directories[0].source
        );
    }
    let future = request.bind_service(unit, 4).unwrap();
    assert!(port.mount_path(&future, &future.mounts[0]).await.is_err());
}
