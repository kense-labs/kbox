//! Byte loss around the real public SDK; every execution stays in the real Host.

use super::*;
use crate::{MacosHvfOciBundleProvider, OciLifecycleAdapter, OciLocalExecutionBackend};
use kbox_core::{exec::ExecEvent, ExecutionGeneration, ExecutionId};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

async fn frame(reader: &mut (impl AsyncRead + Unpin)) -> Vec<u8> {
    let length = reader.read_u32().await.unwrap();
    assert!((1..=64 * 1024 * 1024).contains(&length));
    let mut bytes = vec![0; length as usize];
    reader.read_exact(&mut bytes).await.unwrap();
    bytes
}

async fn forward(writer: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) {
    writer.write_u32(bytes.len() as u32).await.unwrap();
    writer.write_all(bytes).await.unwrap();
    writer.flush().await.unwrap();
}

async fn response_loss(root: &Path, id: &ExecutionId, generation: ExecutionGeneration) {
    let (client_io, mut proxy) = tokio::io::duplex(128 * 1024);
    let mut upstream = tokio::net::UnixStream::connect(root.join("service/runtime.sock"))
        .await
        .unwrap();
    let relay = tokio::spawn(async move {
        loop {
            let request = frame(&mut proxy).await;
            forward(&mut upstream, &request).await;
            let response = frame(&mut upstream).await;
            let decoded: serde_json::Value = serde_json::from_slice(&request).unwrap();
            if decoded["request"]["operation"] == "exec" {
                let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
                assert_eq!(response["result"]["status"], "ok");
                return json!({"request":decoded,"committedResponseDropped":response});
            }
            forward(&mut proxy, &response).await;
        }
    });
    let transport = koci_sdk::RuntimeTransportClient::from_io(client_io)
        .await
        .unwrap();
    let backend = OciLocalExecutionBackend::from_client(
        OciRuntimeEndpoint::unix_socket(root.join("service/runtime.sock")).unwrap(),
        koci_sdk::RuntimeClient::new(transport),
        Arc::new(MacosHvfOciBundleProvider::new(
            root.join("home"),
            root.join("service/runtime"),
        )),
    )
    .unwrap();
    let disconnected = LocalExecutionManager::with_oci_migration_backend(
        root.join("home/boxes.json"),
        root.join("home"),
        Arc::new(backend),
        OciMigrationPolicy::MicrovmViaOci,
    );
    let mut command = exec(&[
        "/bin/sh",
        "-c",
        "printf x >> /writable/exec-count; printf replay-marker",
    ]);
    command.request_id = Some("native-committed-exec-loss".into());
    assert!(disconnected
        .execute(id, generation, command.clone())
        .await
        .is_err());
    let observation = relay.await.unwrap();
    fs::write(
        root.join("committed-response-loss.json"),
        serde_json::to_vec_pretty(&observation).unwrap(),
    )
    .unwrap();
    drop(disconnected);
    let manager = connect(root, OciMigrationPolicy::MicrovmViaOci).await;
    let output = manager
        .execute(id, generation, command.clone())
        .await
        .unwrap();
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, b"replay-marker");
    let (left, right) = tokio::join!(
        manager.execute(id, generation, command.clone()),
        manager.execute(id, generation, command)
    );
    for output in [left.unwrap(), right.unwrap()] {
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, b"replay-marker");
    }
    assert_eq!(fs::read(root.join("writable/exec-count")).unwrap(), b"x");
    let other = connect(root, OciMigrationPolicy::LegacyOnly).await;
    let mut concurrent = exec(&[
        "/bin/sh",
        "-c",
        "printf y >> /writable/concurrent-count; printf concurrent",
    ]);
    concurrent.request_id = Some("native-first-concurrent-exec".into());
    let (left, right) = tokio::join!(
        manager.execute(id, generation, concurrent.clone()),
        other.execute(id, generation, concurrent)
    );
    for output in [left.unwrap(), right.unwrap()] {
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, b"concurrent");
    }
    assert_eq!(
        fs::read(root.join("writable/concurrent-count")).unwrap(),
        b"y"
    );
}

pub(super) async fn exercise(
    root: &Path,
    host: &mut Host,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    legacy_id: &ExecutionId,
    legacy_generation: ExecutionGeneration,
) {
    response_loss(root, id, generation).await;
    let manager = connect(root, OciMigrationPolicy::MicrovmViaOci).await;
    let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
    let before = store.get(id).unwrap().unwrap();
    let binding = before
        .managed_execution
        .as_ref()
        .unwrap()
        .oci_runtime
        .clone()
        .unwrap();
    let legacy_before = store.get(legacy_id).unwrap().unwrap();
    let mut command = exec(&[
        "/bin/sh",
        "-c",
        "printf ready; read value; printf '%s' \"$value\" > /writable/late-input",
    ]);
    command.stdin_streaming = true;
    command.timeout_ns = 60_000_000_000;
    let mut process = manager
        .start_process(id, generation, command)
        .await
        .unwrap();
    let input = process.input();
    let mut ready = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready.ends_with(b"ready") {
            match process.next_event().await.unwrap().unwrap() {
                ExecEvent::Chunk(chunk) => ready.extend(chunk.data),
                event => panic!("unexpected pre-crash event: {event:?}"),
            }
        }
    })
    .await
    .unwrap();
    let killed_pid = host.sigkill();
    assert!(manager.inspect(id).await.is_err());
    assert_eq!(
        manager.inspect(legacy_id).await.unwrap().state,
        ExecutionState::Running
    );
    let legacy_after = store.get(legacy_id).unwrap().unwrap();
    assert_eq!(
        (legacy_before.pid, legacy_before.pid_start_time),
        (legacy_after.pid, legacy_after.pid_start_time)
    );
    assert_eq!(
        manager
            .execute(
                legacy_id,
                legacy_generation,
                exec(&["/bin/echo", "legacy-alive"])
            )
            .await
            .unwrap()
            .stdout,
        b"legacy-alive\n"
    );
    // Existing HVF owner-death protocol publishes authenticated stopped evidence.
    let recovery = root.join("service/runtime/recovery").join(format!(
        "{}-{}.json",
        binding.target.id,
        binding.target.generation.unwrap().0
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !recovery.is_file() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(recovery.is_file(), "owner-death report missing");
    fs::copy(&recovery, root.join("owner-death-report.json")).unwrap();
    *host = Host::start(root, 2);
    let replacement_pid = host.child.as_ref().unwrap().id();
    assert_ne!(killed_pid, replacement_pid);
    let replacement = connect(root, OciMigrationPolicy::LegacyOnly).await;
    let client = OciLifecycleAdapter::connect(
        OciRuntimeEndpoint::unix_socket(root.join("service/runtime.sock")).unwrap(),
    )
    .await
    .unwrap()
    .client();
    let recovered = client
        .state(koci_sdk::StateRequest {
            target: binding.target.clone(),
        })
        .await
        .unwrap();
    assert_eq!(recovered.generation, binding.target.generation.unwrap());
    assert_eq!(recovered.driver, binding.driver);
    assert_eq!(recovered.config_digest, binding.config_digest);
    assert_eq!(
        *recovered.state.status(),
        koci_sdk::oci_spec::runtime::ContainerState::Stopped
    );
    let status = client
        .wait(koci_sdk::WaitRequest {
            target: binding.target.clone(),
            timeout_ms: Some(20_000),
        })
        .await
        .unwrap();
    assert_eq!(
        status,
        koci_sdk::ExitStatus::signaled(libc::SIGKILL, false).unwrap()
    );
    fs::write(
        root.join("recovered-sdk.json"),
        serde_json::to_vec_pretty(&recovered).unwrap(),
    )
    .unwrap();
    assert_eq!(
        replacement.inspect(id).await.unwrap().state,
        ExecutionState::Stopped
    );
    let stopped = store.get(id).unwrap().unwrap().managed_execution.unwrap();
    assert_eq!(stopped.generation, generation);
    assert_eq!(stopped.runtime_route, ManagedRuntimeRoute::OciSdk);
    assert!(
        stopped.oci_runtime.is_none(),
        "terminal reconciliation releases the exact runtime"
    );
    assert!(input.write_stdin(b"late\n").await.is_err());
    let operation = OperationId::new("native-restart-after-owner-death").unwrap();
    let restarted = replacement
        .restart(id, generation, &operation)
        .await
        .unwrap();
    assert_eq!(restarted.generation.get(), generation.get() + 1);
    assert_eq!(
        replacement
            .restart(id, generation, &operation)
            .await
            .unwrap()
            .generation,
        restarted.generation
    );
    assert!(replacement
        .execute(id, generation, exec(&["/bin/touch", "/writable/stale"]))
        .await
        .is_err());
    assert!(input.write_stdin(b"redirected\n").await.is_err());
    assert!(!root.join("writable/late-input").exists());
    assert!(!root.join("writable/stale").exists());
    assert_eq!(
        replacement
            .execute(id, restarted.generation, exec(&["/bin/echo", "restarted"]))
            .await
            .unwrap()
            .stdout,
        b"restarted\n"
    );
    super::files::check(&replacement, root, "main", id, restarted.generation).await;
    let ownership = super::ownership::check(&replacement, id, restarted.generation).await;
    fs::write(
        root.join("ownership-after-stopped-recovery.json"),
        serde_json::to_vec_pretty(&ownership).unwrap(),
    )
    .unwrap();
    let repeated = behavior::readonly_probe(&replacement, root, id, restarted.generation).await;
    assert_eq!(repeated["hostDenials"], 14);
    let new_binding = store
        .get(id)
        .unwrap()
        .unwrap()
        .managed_execution
        .unwrap()
        .oci_runtime
        .unwrap();
    assert_eq!(new_binding.target.id, binding.target.id);
    assert_ne!(new_binding.target.generation, binding.target.generation);
    fs::write(root.join("faults.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","committedResponseLoss":true,"clientDisconnect":true,"concurrentReplayEffects":1,"killedHostPid":killed_pid,"replacementHostPid":replacement_pid,"recoveredState":"stopped","legacySurvived":true,"oldInputRefused":true,"restartedBinding":new_binding})).unwrap()).unwrap();
}
