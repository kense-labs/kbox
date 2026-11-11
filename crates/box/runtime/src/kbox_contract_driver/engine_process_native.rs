use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use kbox_contract::contract::{
    RuntimeApplyRequest, RuntimeExecRequest, RuntimeUnitClass, RuntimeUnitState,
};
use kbox_contract::{
    FileRuntimeStateStore, ManagedRuntimeClient, RuntimeClient, RuntimeStateStore,
};
use kbox_core::exec::{ExecEvent, ExecRequest, StreamType, UNLIMITED_STREAM_TIMEOUT_NS};
use sha2::{Digest, Sha256};

use super::test_support::{action, runtime_spec};
use super::{BoxRuntimeDriver, BoxRuntimeDriverConfig, NATIVE_GUEST_PROFILE_DIGEST};

#[test]
#[ignore = "requires an owned home, the Linux guest-init artifact, signed HVF/KVM assets and a cached Python OCI image"]
fn native_process_stream_backpressure_timeout_and_cleanup() {
    run_native(false);
}

#[test]
#[ignore = "requires an owned native test home and the same signed VM assets as the stream case"]
fn native_process_stream_with_persistent_volume() {
    run_native(true);
}

fn run_native(with_volume: bool) {
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter("kbox_runtime=debug,kbox_core=debug")
        .try_init();
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(run(with_volume));
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn run(with_volume: bool) {
    use std::os::unix::fs::PermissionsExt;
    let home = PathBuf::from(std::env::var_os("K_HOME").expect("owned native test home"));
    assert_eq!(home.canonicalize().unwrap(), home);
    assert_eq!(home.metadata().unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(
        std::fs::read_to_string(home.join(".krun-stream-native")).unwrap(),
        "owned-krun-stream-test\n"
    );
    let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").expect("digest-pinned OCI image");
    let digest = image.rsplit_once('@').unwrap().1;
    assert!(digest.starts_with("sha256:"));
    let driver = Arc::new(
        BoxRuntimeDriver::new(BoxRuntimeDriverConfig {
            home_dir: home.clone(),
            control_timeout: Duration::from_secs(60),
            secret_root: home.join("runtime-secrets"),
            ..Default::default()
        })
        .unwrap(),
    );
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    let state = Arc::new(FileRuntimeStateStore::new(home.join("stream-runtime")));
    let client = Arc::new(ManagedRuntimeClient::new(state.clone(), driver.clone()));
    let mut spec = runtime_spec(
        &format!("native-stream-{}", uuid::Uuid::new_v4().simple()),
        1,
        RuntimeUnitClass::Service,
    );
    spec.artifact.uri = format!("oci://{image}");
    spec.artifact.digest = digest.into();
    spec.process.command = vec!["/bin/sleep".into()];
    spec.process.args = vec!["2147483647".into()];
    spec.process.working_directory = Some("/".into());
    spec.resources.cpu_millis = 1000;
    spec.resources.memory_bytes = 512 * 1024 * 1024;
    spec.resources.pids = u32::MAX;
    spec.semantics_profile_digest = Some(NATIVE_GUEST_PROFILE_DIGEST.into());
    if with_volume {
        spec.mounts.push(kbox_contract::contract::RuntimeMount {
            name: "native-data".into(),
            source: kbox_contract::contract::RuntimeMountSource::Volume {
                volume_id: "native-stream-data".into(),
            },
            target: "/data".into(),
            read_only: false,
        });
    }
    let result = std::panic::AssertUnwindSafe(async {
        let observation = client.apply(&RuntimeApplyRequest {
            schema: RuntimeApplyRequest::SCHEMA.into(), request_id: "native-stream-up".into(),
            deadline_at_ms: None, spec: spec.clone(), restore_from_snapshot_operation_id: None,
        }).await.unwrap();
        assert_eq!(observation.state, RuntimeUnitState::Running);
        let unit = state.load(&spec.unit_id).await.unwrap();
        let lease = state.acquire_operation_lease(&spec.unit_id).await.unwrap();
        let program = "import pathlib,sys,time; sys.stdout.buffer.write(b'A'*(8*1024*1024)); sys.stdout.buffer.flush(); pathlib.Path('/tmp/krun-stream-produced').write_text('ready'); data=sys.stdin.buffer.read(); time.sleep(6); sys.stdout.buffer.write(b'\\xff\\x00tail:'+str(len(data)).encode()); sys.stdout.buffer.flush(); sys.stderr.buffer.write('尾部stderr\\n'.encode()); sys.stderr.buffer.flush()";
        let started = std::time::Instant::now();
        let (execution_id, execution_generation, mut process) = driver.start_runtime_process(&unit, ExecRequest {
            request_id: None, cmd: vec!["python3".into(),"-c".into(),program.into()],
            timeout_ns: UNLIMITED_STREAM_TIMEOUT_NS, env: Vec::new(), working_dir: None,
            rootfs: None, stdin: None, stdin_streaming: true, user: None, streaming: true,
        }).await.unwrap();
        drop(lease);
        assert_eq!(Some(execution_id.as_str()), observation.provider_resource_id.as_deref());
        tokio::time::sleep(Duration::from_millis(500)).await;
        let checkpoint = client.exec(&RuntimeExecRequest {
            schema: RuntimeExecRequest::SCHEMA.into(), request_id: "native-stream-checkpoint".into(),
            unit_id: spec.unit_id.clone(), generation: spec.generation, timeout_ms: 2000,
            deadline_at_ms: None, command: vec!["python3".into(), "-c".into(),
                "from pathlib import Path; print(int(Path('/tmp/krun-stream-produced').exists()))".into()],
        }).await.unwrap();
        assert_eq!(checkpoint.exit_code, 0);
        assert_eq!(checkpoint.stdout.trim(), "0", "slow-consumer backpressure did not reach the guest process");
        let input = process.input();
        let writer = tokio::spawn(async move {
            for _ in 0..128 { input.write_stdin(&vec![123;16 * 1024]).await.unwrap(); }
            input.close_stdin().await.unwrap();
        });
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit = None;
        tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(event) = process.next_event().await.unwrap() {
                match event {
                    ExecEvent::Chunk(chunk) => match chunk.stream {
                        StreamType::Stdout => stdout.extend(chunk.data),
                        StreamType::Stderr => stderr.extend(chunk.data),
                    },
                    ExecEvent::Exit(result) => { assert!(exit.is_none()); exit = Some(result.exit_code); }
                    ExecEvent::FlushAck => {},
                }
            }
        }).await.unwrap();
        writer.await.unwrap();
        assert_eq!(exit, Some(0));
        assert!(started.elapsed() >= Duration::from_secs(6));
        assert_eq!(stdout.len(), 8*1024*1024 + b"\xff\x00tail:2097152".len());
        assert!(stdout[..8*1024*1024].iter().all(|byte| *byte == b'A'));
        assert_eq!(&stdout[8*1024*1024..], b"\xff\x00tail:2097152");
        assert_eq!(stderr, "尾部stderr\n".as_bytes());
        serde_json::json!({"status":"PASS","executionId":execution_id.as_str(),
            "executionGeneration":execution_generation.get(),"runtimeGeneration":spec.generation,
            "stdoutBytes":stdout.len(),"stdinBytes":2097152,"stderrTailPreserved":true,
            "backpressureReachedGuest":true,"explicitUnlimitedSurvivedDefault":true,
            "guestInitSha256":format!("{:x}",Sha256::digest(std::fs::read(home.join("bin/kbox-guest-init")).unwrap()))})
    }).catch_unwind().await;
    if result.is_err() {
        let records = driver.manager.managed_records().await.unwrap();
        for record in &records {
            if let Ok(bytes) = std::fs::read(&record.console_log) {
                std::fs::write(
                    home.parent()
                        .unwrap()
                        .join(format!("failure-{}.console", record.id)),
                    bytes,
                )
                .unwrap();
            }
        }
        std::fs::write(
            home.parent().unwrap().join("failure-records.json"),
            serde_json::to_vec_pretty(&records).unwrap(),
        )
        .unwrap();
    }
    let stopped = client.stop(&action("native-stream-stop", &spec)).await;
    let removed = client.remove(&action("native-stream-remove", &spec)).await;
    assert!(stopped.is_ok(), "native stop failed: {stopped:?}");
    assert!(removed.is_ok(), "native removal failed: {removed:?}");
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    match result {
        Ok(mut report) => {
            report["zeroPrivateExecutions"] = true.into();
            std::fs::write(
                home.parent().unwrap().join("result.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        Err(error) => std::panic::resume_unwind(error),
    }
}
