//! Opt-in real-runtime proof for the SDK managed lifecycle facade.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use krun_box_sdk::{
    BoxConfig, CreateExecutionRequest, ExecutionIsolation, ExecutionRecordPolicy, ExecutionState,
    KillOutcome, KrunBoxClient, OperationId,
};

#[tokio::test]
#[ignore = "requires a dedicated Krun OS home and Krun OCI Runtime artifacts"]
async fn sdk_create_start_run_and_kill_use_the_canonical_manager() {
    let home = validated_home();
    let client = KrunBoxClient::from_home(&home);
    let image =
        std::env::var("KRUN_BOX_SDK_SMOKE_IMAGE").unwrap_or_else(|_| "alpine:3.20".to_string());

    let create_operation = operation("create");
    let reservation = client
        .create_box(request(&image, "created"), &create_operation)
        .await
        .unwrap();
    assert!(!home
        .join("boxes")
        .join(reservation.execution_id.as_str())
        .exists());
    let lease = client
        .start_box(&reservation.execution_id, reservation.generation)
        .await
        .unwrap();
    assert_eq!(lease.execution_id, reservation.execution_id);
    assert_eq!(
        client
            .inspect_execution(&lease.execution_id)
            .await
            .unwrap()
            .state,
        ExecutionState::Running
    );
    assert_eq!(
        client
            .kill_execution(&lease.execution_id, lease.generation)
            .await
            .unwrap(),
        KillOutcome::Killed
    );
    assert_runtime_removed(&home, lease.execution_id.as_str());

    let run_operation = operation("run");
    let running = client
        .run_box(request(&image, "running"), &run_operation)
        .await
        .unwrap();
    assert_eq!(
        client
            .inspect_execution(&running.execution_id)
            .await
            .unwrap()
            .state,
        ExecutionState::Running
    );
    assert_eq!(
        client
            .kill_execution(&running.execution_id, running.generation)
            .await
            .unwrap(),
        KillOutcome::Killed
    );
    assert_runtime_removed(&home, running.execution_id.as_str());
}

fn validated_home() -> PathBuf {
    assert_eq!(
        std::env::var("KRUN_BOX_SDK_MANAGED_SMOKE").as_deref(),
        Ok("1"),
        "set KRUN_BOX_SDK_MANAGED_SMOKE=1 to acknowledge the destructive smoke test"
    );
    let home = PathBuf::from(std::env::var_os("KRUN_HOME").expect("KRUN_HOME is required"));
    assert!(home.is_absolute());
    assert!(home
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains("sdk-managed-smoke")));
    for (variable, binary) in [
        ("KRUN_BOX_OCI_RUNTIME_PATH", "krun-oci"),
        ("KRUN_BOX_OCI_AGENT_PATH", "krun-oci-agent"),
    ] {
        let configured =
            PathBuf::from(std::env::var_os(variable).expect("runtime path is required"));
        assert_eq!(
            configured.canonicalize().unwrap(),
            home.join("bin").join(binary).canonicalize().unwrap()
        );
    }
    for binary in [
        "krun-oci",
        "krun-oci-agent",
        "krun-box-shim",
        "krun-box-guest-init",
    ] {
        assert!(home.join("bin").join(binary).is_file());
    }
    home
}

fn request(image: &str, suffix: &str) -> CreateExecutionRequest {
    CreateExecutionRequest {
        external_sandbox_id: format!("sdk-smoke-{suffix}"),
        config: BoxConfig {
            image: image.to_string(),
            isolation: ExecutionIsolation::Sandbox,
            cmd: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "while :; do sleep 60; done".to_string(),
            ],
            ..BoxConfig::default()
        },
        labels: BTreeMap::from([("purpose".to_string(), "sdk-managed-smoke".to_string())]),
        policy: ExecutionRecordPolicy {
            name: Some(format!("sdk-managed-smoke-{suffix}")),
            ..ExecutionRecordPolicy::default()
        },
        rootfs_snapshot_id: None,
    }
}

fn operation(suffix: &str) -> OperationId {
    OperationId::new(format!(
        "sdk-managed-smoke-{suffix}-{}",
        uuid::Uuid::new_v4()
    ))
    .unwrap()
}

fn assert_runtime_removed(home: &Path, execution_id: &str) {
    assert!(!home.join("boxes").join(execution_id).exists());
    assert!(!home.join("run/krun-oci").join(execution_id).exists());
    assert!(!Path::new("/tmp/krun-box-sockets")
        .join(execution_id)
        .exists());
}
