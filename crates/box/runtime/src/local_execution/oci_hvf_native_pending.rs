//! Exact public-SDK reconciliation of sentinel-owned interrupted Create intents.
use super::*;
use koci_sdk::{
    ContainerRecord, ContainerTarget, CreateAttachments, CreateRequest, DeleteMode, DeleteRequest,
    Generation, IsolationRequest, ListRequest, OciBundle, OperationContext, RuntimeClient,
    RuntimeTransportClient,
};

pub(super) async fn drain(root: &Path) -> usize {
    let manager = connect(root, OciMigrationPolicy::MicrovmViaOci).await;
    drop(manager);
    let stream = tokio::net::UnixStream::connect(root.join("service/runtime.sock"))
        .await
        .unwrap();
    let client = RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());
    let state = root.join("service/state");
    let before_starts = start_receipts(&state);
    let mut rows = Vec::new();
    for entry in fs::read_dir(state.join("containers")).unwrap() {
        let directory = entry.unwrap().path();
        let stored: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("record.json")).unwrap()).unwrap();
        assert_eq!(stored["schemaVersion"], "koci.container-record.v1");
        let record: ContainerRecord = serde_json::from_value(stored["record"].clone()).unwrap();
        assert_eq!(record.driver, koci_sdk::DriverKind::LibkHvf);
        assert_eq!(record.isolation, koci_sdk::IsolationClass::DedicatedVm);
        assert_eq!(
            *record.state.status(),
            koci_sdk::oci_spec::runtime::ContainerState::Creating
        );
        assert_eq!(record.generation, Generation(1));
        assert_eq!(directory.file_name().unwrap(), record.state.id().as_str());
        let operation = stored["activeOperation"].as_str().unwrap();
        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(state.join("operations").join(format!("{operation}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(receipt["containerId"], record.state.id().as_str());
        assert_eq!(receipt["generation"], 1);
        assert_eq!(receipt["kind"], "create");
        assert_eq!(receipt["outcome"]["status"], "prepared");
        let config = fs::read_to_string(directory.join("config.json")).unwrap();
        let bundle = OciBundle::from_json(record.state.bundle().clone(), config).unwrap();
        assert_eq!(bundle.config_digest(), record.config_digest);
        let attachments: CreateAttachments =
            serde_json::from_value(stored["attachments"].clone()).unwrap();
        assert_eq!(
            Some(attachments.digest().unwrap()),
            record.attachments_digest
        );
        let target = ContainerTarget::exact(
            koci_sdk::ContainerId::new(record.state.id().clone()).unwrap(),
            record.generation,
        );
        let request = CreateRequest {
            context: OperationContext::new(koci_sdk::OperationId::new(operation).unwrap()),
            id: target.id.clone(),
            bundle,
            isolation: IsolationRequest::DedicatedVm,
            attachments,
        };
        let created = match client.create(request.clone()).await {
            Ok(created) => created,
            Err(error) => {
                assert_eq!(error.code, koci_sdk::ErrorCode::FailedPrecondition);
                assert!(!error.retryable);
                assert_eq!(client.create(request).await.unwrap_err(), error);
                assert_eq!(
                    client
                        .state(koci_sdk::StateRequest {
                            target: target.clone()
                        })
                        .await
                        .unwrap_err()
                        .code,
                    koci_sdk::ErrorCode::NotFound
                );
                let share = root
                    .join("service/runtime/shares")
                    .join(target.id.as_str())
                    .join(record.generation.0.to_string());
                assert!(!share.exists());
                assert!(!share.with_extension("rootfs.ext4").exists());
                assert!(!share.with_extension("files").exists());
                rows.push(json!({"target":target,"originalOperation":operation,"originalRequestDigest":receipt["requestDigest"],"configDigest":record.config_digest,"createResumed":false,"terminalFailureSettled":true,"replayedFailure":error}));
                continue;
            }
        };
        assert_eq!(created.generation, record.generation);
        assert_eq!(
            *created.state.status(),
            koci_sdk::oci_spec::runtime::ContainerState::Created
        );
        client
            .delete(DeleteRequest {
                context: OperationContext::new(
                    koci_sdk::OperationId::new(format!("owned-pending-cleanup-{}", target.id))
                        .unwrap(),
                ),
                target: target.clone(),
                mode: DeleteMode::Force,
            })
            .await
            .unwrap();
        rows.push(json!({"target":target,"originalOperation":operation,"originalRequestDigest":receipt["requestDigest"],"configDigest":record.config_digest,"createResumed":true,"exactForceDelete":true}));
    }
    assert!(!rows.is_empty());
    assert!(client
        .list(ListRequest::default())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        start_receipts(&state),
        before_starts,
        "cleanup must never start the workload"
    );
    fs::write(
        root.join("pending-create-cleanup.json"),
        serde_json::to_vec_pretty(&json!({"status":"PASS","records":rows,"workloadStartsAdded":0}))
            .unwrap(),
    )
    .unwrap();
    rows.len()
}

fn start_receipts(state: &Path) -> usize {
    fs::read_dir(state.join("operations"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.extension()? != "json" {
                return None;
            }
            let receipt: serde_json::Value =
                serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            (receipt["kind"] == "start").then_some(())
        })
        .count()
}

#[test]
#[ignore = "maintenance only: requires sentinel-owned root with known pending Create"]
fn native_reconcile_owned_pending_creates() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
            assert_eq!(root.canonicalize().unwrap(), root);
            assert_eq!(
                fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),
                "owned-hvf-native-test\n"
            );
            let host = Host::start(&root, 100);
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(drain(&root));
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
        })
        .unwrap()
        .join()
        .unwrap();
}
