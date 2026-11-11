use super::*;
use crate::engine::EngineJournal;
use crate::kbox_contract_driver::test_support::{accepted, unit};

fn intent(engine: &EngineKey) -> EngineIntent {
    let mut value = crate::engine::tests::intent(engine, 1, DesiredState::Up);
    value.assembly = Some(EngineAssemblyPlan {
        prewarm_bindings_digest: None,
        required_guest_protocol: None,
        required_guest_protocols: Vec::new(),
        archive: "/packages/中文 package.zip".into(),
        digest: format!("sha256:{}", "1".repeat(64)),
    });
    value.service.as_mut().unwrap().process.command = vec!["/bin/sleep".into()];
    value.service.as_mut().unwrap().process.args = vec!["2147483647".into()];
    value
}

#[test]
fn assembly_is_optional_and_only_reserves_its_own_control_path() {
    let engine = EngineKey::parse("engine").unwrap();
    let legacy = crate::engine::tests::intent(&engine, 1, DesiredState::Up);
    assert!(serde_json::to_value(&legacy)
        .unwrap()
        .get("assembly")
        .is_none());
    let valid = intent(&engine);
    valid.validate().unwrap();
    for digest in ["sha256:1", "unknown", ""] {
        let mut value = valid.clone();
        value.assembly.as_mut().unwrap().digest = digest.into();
        assert!(value.validate().is_err());
    }
    let mut value = valid.clone();
    value.workload = EngineWorkload::OciService;
    assert!(value.validate().is_err());
    value = valid.clone();
    value.assembly.as_mut().unwrap().archive = "relative".into();
    assert!(value.validate().is_err());
    for target in ["/opt", "/opt/krun", "/opt/krun/bin"] {
        let mut value = valid.clone();
        value.service.as_mut().unwrap().mounts = vec![kbox_contract::contract::RuntimeMount {
            name: "overlay".into(),
            source: kbox_contract::contract::RuntimeMountSource::Tmpfs { size_bytes: 1024 },
            target: target.into(),
            read_only: false,
        }];
        assert!(value.validate().is_err());
        value.assembly = None;
        value.validate().unwrap();
    }
}

#[test]
fn prewarm_requires_a_clean_idle_bootstrap_and_preserves_legacy_assembly() {
    let engine = EngineKey::parse("engine").unwrap();
    let mut value = intent(&engine);
    assert!(serde_json::to_value(value.assembly.as_ref().unwrap())
        .unwrap()
        .get("prewarmBindingsDigest")
        .is_none());
    value.assembly.as_mut().unwrap().prewarm_bindings_digest =
        Some(format!("sha256:{}", "2".repeat(64)));
    assert!(value.validate().is_err());
    let service = value.service.as_mut().unwrap();
    service.process.args = vec!["infinity".into()];
    service.process.environment.clear();
    service.health = None;
    service.restart = kbox_contract::contract::RestartPolicy::Never;
    value.validate().unwrap();
    value
        .service
        .as_mut()
        .unwrap()
        .process
        .environment
        .insert("TOKEN".into(), "synthetic".into());
    assert!(value.validate().is_err());
}

#[test]
fn assembly_enforces_an_explicit_guest_protocol_before_receipting() {
    let mut intent = intent(&EngineKey::parse("engine").unwrap());
    let plan = intent.assembly.as_mut().unwrap();
    let mut old: GuestInstallation = serde_json::from_value(serde_json::json!({
        "schema": "krun.guest-installation.v1", "digest": plan.digest, "root": plan.root()
    }))
    .unwrap();
    old.validate(plan).unwrap();
    assert!(serde_json::to_value(&plan)
        .unwrap()
        .get("requiredGuestProtocol")
        .is_none());
    assert!(serde_json::to_value(&plan)
        .unwrap()
        .get("requiredGuestProtocols")
        .is_none());
    let required = EngineKey::parse("secret-launch.v1").unwrap();
    plan.required_guest_protocol = Some(required.clone());
    assert_eq!(
        old.validate(plan).unwrap_err().code,
        EngineErrorCode::UnsupportedCapability
    );
    old.protocols.push(required);
    old.validate(plan).unwrap();
    plan.required_guest_protocols = vec![EngineKey::parse("runtime-requirements.v1").unwrap()];
    assert_eq!(
        old.validate(plan).unwrap_err().code,
        EngineErrorCode::UnsupportedCapability
    );
    old.protocols.extend(plan.required_guest_protocols.clone());
    old.validate(plan).unwrap();
    plan.required_guest_protocols
        .push(EngineKey::parse("secret-launch.v1").unwrap());
    assert!(intent.validate().is_err());
}

#[test]
fn guest_install_failure_requires_the_exact_archive_and_requested_protocol() {
    let mut intent = intent(&EngineKey::parse("engine").unwrap());
    let plan = intent.assembly.as_mut().unwrap();
    plan.required_guest_protocol = Some(EngineKey::parse("runtime-requirements.v1").unwrap());
    let mut reply = serde_json::json!({
        "schema": "krun.guest-installation-error.v1", "digest": plan.digest,
        "protocol": "runtime-requirements.v1", "code": "RUNTIME_REQUIREMENTS_UNMET"
    });
    let decode = |value: &serde_json::Value| {
        GuestInstallationFailure::decode(&serde_json::to_vec(value).unwrap(), plan)
    };
    assert_eq!(
        decode(&reply).unwrap().code,
        EngineErrorCode::RuntimeRequirementsUnmet
    );
    reply["code"] = "UNSUPPORTED_CAPABILITY".into();
    assert_eq!(
        decode(&reply).unwrap().code,
        EngineErrorCode::UnsupportedCapability
    );
    for (key, other) in [
        ("digest", "sha256:other"),
        ("protocol", "secret-launch.v1"),
        ("code", "STALE_INTENT"),
        ("schema", "other"),
    ] {
        let mut invalid = reply.clone();
        invalid[key] = other.into();
        assert!(decode(&invalid).is_none());
    }
}

#[test]
fn archive_copy_is_immutable_digest_bound_and_rejects_links_and_oversize() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("资源.zip");
    let bytes = vec![83; 100_000];
    std::fs::write(&path, &bytes).unwrap();
    let mut plan = EngineAssemblyPlan {
        prewarm_bindings_digest: None,
        required_guest_protocol: None,
        required_guest_protocols: Vec::new(),
        archive: path.clone(),
        digest: format!("sha256:{:x}", Sha256::digest(&bytes)),
    };
    let cancelled = AtomicBool::new(false);
    cancelled.store(true, Ordering::Relaxed);
    assert_eq!(
        verified_archive(&plan, &cancelled).unwrap_err().code,
        EngineErrorCode::StaleIntent
    );
    cancelled.store(false, Ordering::Relaxed);
    let mut copy = verified_archive(&plan, &cancelled).unwrap();
    std::fs::write(&path, b"changed").unwrap();
    let mut received = Vec::new();
    copy.read_to_end(&mut received).unwrap();
    assert_eq!(received, bytes);
    assert!(verified_archive(&plan, &cancelled).is_err());
    let alias = root.path().canonicalize().unwrap().join("alias");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    plan.archive = alias;
    assert!(verified_archive(&plan, &cancelled).is_err());
    plan.archive = path;
    File::options()
        .write(true)
        .open(&plan.archive)
        .unwrap()
        .set_len(MAX_ARCHIVE_BYTES + 1)
        .unwrap();
    assert!(verified_archive(&plan, &cancelled).is_err());
}

#[test]
fn assembly_receipts_survive_restart_and_bind_exact_runtime_provider_and_intent() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("intent.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let up = intent(journal.engine_id());
    journal.accept(up.clone()).unwrap();
    let record = journal.current(&up.instance_id).unwrap().clone();
    let spec = up
        .bind_service(journal.unit_id(&up.instance_id), 1)
        .unwrap();
    let mut observation = accepted(&spec);
    observation.state = RuntimeUnitState::Running;
    observation.provider_resource_id = Some("provider-1".into());
    let unit = unit(spec.clone(), observation);
    assert_eq!(
        journal.require_assembly(&unit, &record).unwrap_err().code,
        EngineErrorCode::CleanupPending
    );
    let plan = up.assembly.as_ref().unwrap();
    let receipt = EngineAssemblyReceipt {
        schema: "kbox.engine.assembly.v1".into(),
        accepted: record.accepted.clone(),
        runtime_request_id: record.runtime_request_id.clone(),
        generation: 1,
        spec_digest: spec.digest().unwrap(),
        execution_id: "provider-1".into(),
        execution_generation: 7,
        package_digest: plan.digest.clone(),
        root: plan.root(),
    };
    journal.publish_assembly(&record, receipt.clone()).unwrap();
    journal.publish_assembly(&record, receipt.clone()).unwrap();
    journal.require_assembly(&unit, &record).unwrap();
    let mut changed = unit.clone();
    changed.observation.provider_resource_id = Some("replacement".into());
    assert!(!journal.assembly_matches(&changed, &record).unwrap());
    drop(journal);
    let mut journal = EngineJournal::open(path).unwrap();
    journal.require_assembly(&unit, &record).unwrap();
    let mut wrong = receipt.clone();
    wrong.execution_generation = 8;
    assert!(journal.publish_assembly(&record, wrong).is_err());
    let mut down = crate::engine::tests::intent(journal.engine_id(), 2, DesiredState::Down);
    down.instance_id = up.instance_id.clone();
    journal.accept(down).unwrap();
    assert_eq!(
        journal.publish_assembly(&record, receipt).unwrap_err().code,
        EngineErrorCode::StaleIntent
    );
}

#[test]
fn corrupt_receipt_cannot_become_ready_after_reopening_the_journal() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("intent.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let up = intent(journal.engine_id());
    journal.accept(up.clone()).unwrap();
    let record = journal.current(&up.instance_id).unwrap().clone();
    let spec = up
        .bind_service(journal.unit_id(&up.instance_id), 1)
        .unwrap();
    let plan = up.assembly.as_ref().unwrap();
    let receipt = EngineAssemblyReceipt {
        schema: "kbox.engine.assembly.v1".into(),
        accepted: record.accepted.clone(),
        runtime_request_id: record.runtime_request_id.clone(),
        generation: 1,
        spec_digest: spec.digest().unwrap(),
        execution_id: "provider-1".into(),
        execution_generation: 1,
        package_digest: plan.digest.clone(),
        root: plan.root(),
    };
    journal.publish_assembly(&record, receipt).unwrap();
    drop(journal);
    let mut data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    data["assemblies"][&record.runtime_request_id]["root"] = "/outside".into();
    std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
    assert!(EngineJournal::open(path).is_err());
}
