use std::collections::BTreeMap;
use std::time::Duration;

use kbox_core::exec::{ExecExit, UNLIMITED_STREAM_TIMEOUT_NS};
use tokio::io::AsyncWriteExt;

use super::*;
use crate::engine::process_protocol::MAX_BOOTSTRAP_BYTES;
use crate::engine::process_transport::{
    read_bootstrap, read_process_frame, MAX_PROCESS_DATA_BYTES,
};

fn primary(engine: &EngineKey) -> EnginePrimaryOpen {
    EnginePrimaryOpen {
        schema: "kbox.engine.primary-open.v1".into(),
        engine_id: engine.clone(),
        instance_id: EngineKey::parse("instance").unwrap(),
        request_id: EngineKey::parse("primary-1").unwrap(),
        generation: 1,
        command: vec![
            "/harness/bin/launch".into(),
            "".into(),
            "--private-extension".into(),
        ],
        environment: BTreeMap::from([("MODEL_TOKEN".into(), "synthetic-secret".into())]),
        working_directory: Some("/中文 workspace".into()),
        timeout_ms: None,
        bootstrap: false,
    }
}

#[test]
fn native_process_request_preserves_empty_args_and_unlimited_default_but_rejects_invalid_input() {
    let request = primary(&EngineKey::parse("engine").unwrap());
    let exec = request.exec_request().unwrap();
    assert_eq!(exec.cmd, request.command);
    assert_eq!(exec.timeout_ns, UNLIMITED_STREAM_TIMEOUT_NS);
    assert_eq!(exec.working_dir.as_deref(), Some("/中文 workspace"));
    assert!(!format!("{request:?}").contains("synthetic-secret"));
    assert!(!format!("{request:?}").contains("private-extension"));
    for timeout in [0, u64::MAX] {
        let mut invalid = request.clone();
        invalid.timeout_ms = Some(timeout);
        assert_eq!(
            invalid.validate().unwrap_err().code,
            EngineErrorCode::InvalidRequest
        );
    }
    let mut invalid = request.clone();
    invalid.command[0] = "bad\0exec".into();
    assert!(invalid.validate().is_err());
    let mut invalid = request.clone();
    invalid
        .environment
        .insert("bad=name".into(), "value".into());
    assert!(invalid.validate().is_err());
    let mut invalid = request.clone();
    invalid.working_directory = Some("relative".into());
    assert!(invalid.validate().is_err());
}

#[test]
fn primary_receipt_and_next_attempt_are_atomic_and_do_not_persist_payload_or_attach_token() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.json");
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let engine_id = journal.engine_id().clone();
    let up = intent(&engine_id, 1, DesiredState::Up);
    let accepted = journal.accept(up.clone()).unwrap();
    let request = primary(&engine_id);
    journal.reserve_primary(&request).unwrap();
    assert_eq!(
        journal
            .finish_primary(
                &request.request_id,
                Some(ExecExit {
                    exit_code: 0,
                    oom_killed: false
                })
            )
            .unwrap_err()
            .code,
        EngineErrorCode::Internal
    );
    assert_eq!(
        journal.primary(&request.request_id).unwrap().info.state,
        EngineProcessState::Reserved
    );
    let ticket = EnginePrimaryTicket {
        process: journal.primary(&request.request_id).unwrap().info.clone(),
        attach_token: EngineAttachToken(EngineKey::parse("synthetic-attach-secret").unwrap()),
    };
    assert!(!format!("{ticket:?}").contains("synthetic-attach-secret"));
    assert_eq!(
        journal.reserve_primary(&request).unwrap_err().code,
        EngineErrorCode::AlreadyConnected
    );
    let mut conflict = request.clone();
    conflict.command.push("changed".into());
    assert_eq!(
        journal.reserve_primary(&conflict).unwrap_err().code,
        EngineErrorCode::RequestConflict
    );
    journal
        .primary_started(&request.request_id, "provider-1".into(), 7)
        .unwrap();
    journal
        .finish_primary(
            &request.request_id,
            Some(ExecExit {
                exit_code: 37,
                oom_killed: false,
            }),
        )
        .unwrap();
    journal.finish_primary(&request.request_id, None).unwrap();
    drop(journal);
    let mut journal = EngineJournal::open(path.clone()).unwrap();
    let record = journal.primary(&request.request_id).unwrap();
    assert_eq!(record.info.state, EngineProcessState::Exited);
    assert_eq!(record.info.exit.as_ref().unwrap().exit_code, 37);
    let current = journal.current(&request.instance_id).unwrap();
    assert_eq!(current.generation, 2);
    assert_eq!(current.attempt, 2);
    assert!(current.runtime_request_id.ends_with("-g2-a2"));
    assert_eq!(journal.accept(up).unwrap(), accepted);
    assert_eq!(
        journal.reserve_primary(&request).unwrap_err().code,
        EngineErrorCode::StreamLost
    );
    let disk = std::fs::read_to_string(path).unwrap();
    assert!(!disk.contains("synthetic-secret"));
    assert!(!disk.contains("private-extension"));
    assert!(!disk.contains("synthetic-attach-secret"));
}

#[test]
fn finishing_old_primary_cannot_change_a_newer_stop_or_replacement() {
    for desired in [DesiredState::Down, DesiredState::Up] {
        let root = tempfile::tempdir().unwrap();
        let mut journal = EngineJournal::open(root.path().join("journal.json")).unwrap();
        let id = journal.engine_id().clone();
        journal.accept(intent(&id, 1, DesiredState::Up)).unwrap();
        let process = primary(&id);
        journal.reserve_primary(&process).unwrap();
        journal.accept(intent(&id, 2, desired)).unwrap();
        let before = journal.current(&process.instance_id).unwrap().clone();
        assert_eq!(
            journal
                .require_current_primary(&process.request_id)
                .unwrap_err()
                .code,
            EngineErrorCode::GenerationMismatch
        );
        journal.finish_primary(&process.request_id, None).unwrap();
        let after = journal.current(&process.instance_id).unwrap();
        assert_eq!(after.generation, before.generation);
        assert_eq!(after.attempt, before.attempt);
        assert_eq!(after.runtime_request_id, before.runtime_request_id);
        assert_eq!(after.intent.desired_state, desired);
    }
}

#[test]
fn a_primary_connection_is_exclusive_per_generation_and_not_global() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = EngineJournal::open(root.path().join("journal.json")).unwrap();
    let id = journal.engine_id().clone();
    journal.accept(intent(&id, 1, DesiredState::Up)).unwrap();
    let process = primary(&id);
    journal.reserve_primary(&process).unwrap();
    let mut second = process.clone();
    second.request_id = EngineKey::parse("second").unwrap();
    assert_eq!(
        journal.reserve_primary(&second).unwrap_err().code,
        EngineErrorCode::AlreadyConnected
    );
    let mut other = intent(&id, 1, DesiredState::Up);
    other.instance_id = EngineKey::parse("other-instance").unwrap();
    other.request_id = EngineKey::parse("other-up").unwrap();
    journal.accept(other.clone()).unwrap();
    second.instance_id = other.instance_id;
    journal.reserve_primary(&second).unwrap();
}

#[tokio::test]
async fn process_frames_preserve_fragments_and_reject_oversize_before_body_read() {
    let (mut sender, mut receiver) = tokio::io::duplex(64);
    let writer = tokio::spawn(async move {
        for byte in [3, 0, 0, 0, 0, 255, 0] {
            sender.write_all(&[byte]).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    assert_eq!(
        read_process_frame(&mut receiver).await.unwrap(),
        [0, 255, 0]
    );
    writer.await.unwrap();
    let (mut sender, mut receiver) = tokio::io::duplex(64);
    sender
        .write_all(&((MAX_PROCESS_DATA_BYTES + 2) as u32).to_le_bytes())
        .await
        .unwrap();
    assert_eq!(
        read_process_frame(&mut receiver).await.unwrap_err().code,
        EngineErrorCode::FrameTooLarge
    );
    let (mut sender, mut receiver) = tokio::io::duplex(64);
    sender.write_all(&[2, 0, 0, 0, 0]).await.unwrap();
    drop(sender);
    assert_eq!(
        read_process_frame(&mut receiver).await.unwrap_err().code,
        EngineErrorCode::Transport
    );
}

#[tokio::test]
async fn bootstrap_frame_rejects_invalid_size_without_reading_a_body() {
    for size in [0, MAX_BOOTSTRAP_BYTES + 1] {
        let header = (size as u32).to_le_bytes();
        let error = read_bootstrap(&mut header.as_slice()).await.err().unwrap();
        assert_eq!(error.code, EngineErrorCode::FrameTooLarge);
    }
}

#[tokio::test(start_paused = true)]
async fn idle_stream_has_no_task_deadline_but_incomplete_frame_expires() {
    let (mut sender, mut receiver) = tokio::io::duplex(64);
    let reader = tokio::spawn(async move { read_process_frame(&mut receiver).await });
    tokio::time::advance(Duration::from_secs(60 * 60)).await;
    assert!(!reader.is_finished());
    sender.write_all(&[2]).await.unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(31)).await;
    assert_eq!(
        reader.await.unwrap().unwrap_err().code,
        EngineErrorCode::Transport
    );
}
