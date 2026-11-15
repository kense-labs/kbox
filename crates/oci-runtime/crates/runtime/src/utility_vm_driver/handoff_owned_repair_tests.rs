//! Offline repair uses the existing cleanup path and exact failed receipt.
use super::*;
use koci_sdk::{ContainerRecord, Generation, IsolationRequest, OperationContext};

#[tokio::test]
#[ignore = "maintenance only: sentinel-owned failed Create and exclusive durable-root lock"]
async fn native_cleanup_owned_terminal_create_handoff() {
    let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
    assert_eq!(root.canonicalize().unwrap(), root);
    assert_eq!(
        tokio::fs::read_to_string(root.join(".kbox-hvf-native"))
            .await
            .unwrap(),
        "owned-hvf-native-test\n"
    );
    let state = root.join("service/state");
    let _lease = crate::state::DurableStateStore::open(&state).await.unwrap();
    let mut active = tokio::fs::read_dir(state.join("containers")).await.unwrap();
    assert!(active.next_entry().await.unwrap().is_none());
    let processes = std::process::Command::new("/bin/ps")
        .args(["-axo", "command="])
        .output()
        .unwrap();
    assert!(processes.status.success());
    for line in String::from_utf8(processes.stdout).unwrap().lines() {
        let Some((executable, args)) = line.trim().split_once(' ') else {
            continue;
        };
        if ["kbox", "koci", "koci-kshim", "kbox-shim"]
            .contains(&Path::new(executable).file_name().unwrap().to_str().unwrap())
        {
            assert!(
                !args.contains(root.to_str().unwrap()),
                "owned runtime process still active"
            );
        }
    }
    let runtime = root.join("service/runtime");
    let handoff = BundleHandoffStore::new(runtime.clone(), runtime.join("shares"));
    let mut quarantine = tokio::fs::read_dir(state.join("quarantine")).await.unwrap();
    let mut cleaned = Vec::new();
    while let Some(entry) = quarantine.next_entry().await.unwrap() {
        if !entry
            .file_name()
            .to_string_lossy()
            .ends_with(".failed-create")
        {
            continue;
        }
        let stored: serde_json::Value = serde_json::from_slice(
            &tokio::fs::read(entry.path().join("record.json"))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(stored["schemaVersion"], "koci.container-record.v1");
        let record: ContainerRecord = serde_json::from_value(stored["record"].clone()).unwrap();
        assert_eq!(record.driver, koci_core::DriverKind::LibkHvf);
        assert_eq!(record.isolation, koci_core::IsolationClass::DedicatedVm);
        assert_eq!(record.generation, Generation(1));
        let operation = stored["activeOperation"].as_str().unwrap();
        let receipt: serde_json::Value = serde_json::from_slice(
            &tokio::fs::read(state.join("operations").join(format!("{operation}.json")))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(receipt["containerId"], record.state.id().as_str());
        assert_eq!(receipt["generation"], 1);
        assert_eq!(receipt["kind"], "create");
        assert_eq!(receipt["outcome"]["status"], "failed");
        let failure: Error = serde_json::from_value(receipt["outcome"]["error"].clone()).unwrap();
        assert!(!failure.retryable);
        let config = tokio::fs::read_to_string(entry.path().join("config.json"))
            .await
            .unwrap();
        let bundle = OciBundle::from_json(record.state.bundle().clone(), config).unwrap();
        assert_eq!(bundle.config_digest(), record.config_digest);
        let contract: CreateAttachments =
            serde_json::from_value(stored["attachments"].clone()).unwrap();
        assert_eq!(Some(contract.digest().unwrap()), record.attachments_digest);
        let request = DriverCreateRequest {
            context: OperationContext::new(OperationId::new(operation).unwrap()),
            target: ContainerTarget::exact(
                ContainerId::new(record.state.id()).unwrap(),
                record.generation,
            ),
            bundle,
            isolation: IsolationRequest::DedicatedVm,
            io: contract.process_io().clone(),
            attachment_contract: contract,
            tee_launch: None,
            attachments: crate::DriverCreateAttachments::None,
        };
        handoff.cleanup_prepared(&request, true).await.unwrap();
        cleaned.push(serde_json::json!({"target":request.target,"originalFailedOperation":operation,"configDigest":record.config_digest,"attachmentsDigest":record.attachments_digest}));
    }
    assert!(!cleaned.is_empty());
    tokio::fs::write(root.join("terminal-create-repair.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","records":cleaned,"durableRootLocked":true,"activeContainers":0,"ownedRuntimeProcesses":0})).unwrap()).await.unwrap();
}
