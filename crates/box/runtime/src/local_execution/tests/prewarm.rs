use kbox_core::ExecutionResourceUpdate;

use super::*;

fn prewarm_harness() -> (tempfile::TempDir, LocalExecutionManager, Arc<FakeBackend>) {
    let (directory, manager, backend) = harness();
    *backend.anonymous_volumes_override.lock().unwrap() = Some(Vec::new());
    (directory, manager, backend)
}

fn idle_request(name: &str) -> CreateExecutionRequest {
    let mut value = request(name);
    value.config.image = format!("base@sha256:{}", "1".repeat(64));
    value.config.entrypoint_override = Some(vec!["/bin/sleep".into()]);
    value.config.cmd = vec!["infinity".into()];
    value
}

#[tokio::test]
async fn stopping_a_reserved_environment_fences_a_late_boot_worker() {
    let (_directory, manager, backend) = prewarm_harness();
    let warm = manager
        .ensure_prewarmed_reservation(
            idle_request("pool"),
            &operation("warm"),
            "engine-build",
            "workspace",
        )
        .await
        .unwrap();
    assert!(manager
        .claim_prewarmed(idle_request("run"), &operation("run"), "engine-build")
        .await
        .unwrap()
        .is_none());
    manager
        .kill(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    manager
        .remove(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    assert!(matches!(
        manager
            .start_prewarmed(&warm.execution_id, warm.generation)
            .await,
        Err(ExecutionManagerError::NotFound(_))
    ));
    assert_eq!(backend.starts.load(Ordering::Relaxed), 0);
    assert!(manager.managed_records().await.unwrap().is_empty());
}

#[tokio::test]
async fn prewarm_claim_rebinds_the_same_running_vm_once_and_fences_the_old_generation() {
    let (_directory, manager, backend) = prewarm_harness();
    let warm = manager
        .ensure_prewarmed_reservation(
            idle_request("pool"),
            &operation("warm"),
            "engine-build",
            "workspace",
        )
        .await
        .unwrap();
    manager
        .start(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    let bound = manager
        .managed_record(&warm.execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(backend.starts.load(Ordering::Relaxed), 1);
    assert!(matches!(
        bound.require_claimed(),
        Err(ExecutionManagerError::Conflict { execution_id, .. }) if execution_id == warm.execution_id
    ));
    assert!(matches!(
        manager
            .update_resources(
                &warm.execution_id,
                warm.generation,
                &operation("premature-update"),
                ExecutionResourceUpdate {
                    pids_limit: Some(128),
                    ..ExecutionResourceUpdate::default()
                },
            )
            .await,
        Err(ExecutionManagerError::Conflict { .. })
    ));
    #[cfg(unix)]
    assert!(matches!(
        manager
            .start_process(
                &warm.execution_id,
                warm.generation,
                ExecRequest {
                    request_id: None,
                    cmd: vec!["true".into()],
                    timeout_ns: 0,
                    env: vec![],
                    working_dir: None,
                    rootfs: None,
                    stdin: None,
                    stdin_streaming: false,
                    user: None,
                    streaming: true
                }
            )
            .await,
        Err(ExecutionManagerError::Conflict { .. })
    ));
    let mut intended = idle_request("harness-independent-instance");
    intended.policy.name = Some("new-owner".into());
    intended
        .labels
        .insert("runtime-spec".into(), "new-generation".into());
    let claimed = manager
        .claim_prewarmed(intended.clone(), &operation("run"), "engine-build")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.execution_id, warm.execution_id);
    assert_eq!(claimed.generation.get(), warm.generation.get() + 1);
    let after = manager
        .managed_record(&claimed.execution_id)
        .await
        .unwrap()
        .unwrap();
    after.require_claimed().unwrap();
    assert_eq!(
        (after.pid, after.pid_start_time, after.started_at),
        (bound.pid, bound.pid_start_time, bound.started_at)
    );
    assert_eq!(
        after
            .managed_execution
            .as_ref()
            .unwrap()
            .request
            .external_sandbox_id,
        "harness-independent-instance"
    );
    manager
        .start(&claimed.execution_id, claimed.generation)
        .await
        .unwrap();
    assert_eq!(backend.starts.load(Ordering::Relaxed), 1);
    assert!(manager
        .start(&warm.execution_id, warm.generation)
        .await
        .is_err());
    assert!(manager
        .kill(&warm.execution_id, warm.generation)
        .await
        .is_err());
    assert!(manager
        .remove(&warm.execution_id, warm.generation)
        .await
        .is_err());
    let replay = manager
        .claim_prewarmed(intended, &operation("run"), "engine-build")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay.execution_id, claimed.execution_id);
    assert!(manager
        .claim_prewarmed(idle_request("second"), &operation("second"), "engine-build")
        .await
        .unwrap()
        .is_none());
    assert!(manager
        .create(idle_request("pool"), &operation("warm"))
        .await
        .is_err());
    assert!(manager
        .ensure_prewarmed_reservation(
            idle_request("pool"),
            &operation("warm"),
            "engine-build",
            "workspace"
        )
        .await
        .is_err());
    manager
        .kill(&claimed.execution_id, claimed.generation)
        .await
        .unwrap();
    manager
        .remove(&claimed.execution_id, claimed.generation)
        .await
        .unwrap();
    assert!(manager.managed_records().await.unwrap().is_empty());
}

#[tokio::test]
async fn prewarm_requires_exact_scope_resources_mounts_and_a_live_provider() {
    let (_directory, manager, backend) = prewarm_harness();
    let warm = manager
        .ensure_prewarmed_reservation(
            idle_request("pool"),
            &operation("warm"),
            "trust-one",
            "workspace",
        )
        .await
        .unwrap();
    manager
        .start(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    assert!(manager
        .claim_prewarmed(
            idle_request("run"),
            &operation("different-trust"),
            "trust-two"
        )
        .await
        .unwrap()
        .is_none());
    for change in 0..4 {
        let mut value = idle_request("run");
        match change {
            0 => value.config.resources.memory_mb += 1,
            1 => value.config.volumes.push("/external:/external:rw".into()),
            2 => value.config.network = NetworkMode::Tsi,
            _ => value.config.privileged = true,
        }
        assert!(manager
            .claim_prewarmed(
                value,
                &operation(&format!("different-{change}")),
                "trust-one"
            )
            .await
            .unwrap()
            .is_none());
    }
    backend.stop_externally(&warm.execution_id, 0);
    assert!(manager
        .claim_prewarmed(idle_request("run"), &operation("dead"), "trust-one")
        .await
        .unwrap()
        .is_none());
    manager
        .kill(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    manager
        .remove(&warm.execution_id, warm.generation)
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_prewarm_reservation_and_claim_have_one_canonical_winner() {
    let (_directory, manager, backend) = prewarm_harness();
    let pool_one = operation("pool-one");
    let pool_two = operation("pool-two");
    let (left, right) = tokio::join!(
        manager.ensure_prewarmed_reservation(idle_request("one"), &pool_one, "engine", "workspace"),
        manager.ensure_prewarmed_reservation(idle_request("two"), &pool_two, "engine", "workspace")
    );
    let warm = left.unwrap();
    assert_eq!(warm.execution_id, right.unwrap().execution_id);
    assert_eq!(manager.managed_records().await.unwrap().len(), 1);
    manager
        .start(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    let run_one = operation("run-one");
    let run_two = operation("run-two");
    let (left, right) = tokio::join!(
        manager.claim_prewarmed(idle_request("run-one"), &run_one, "engine"),
        manager.claim_prewarmed(idle_request("run-two"), &run_two, "engine")
    );
    let winners = [left.unwrap(), right.unwrap()];
    assert_eq!(winners.iter().filter(|value| value.is_some()).count(), 1);
    assert_eq!(backend.starts.load(Ordering::Relaxed), 1);
    let actual = winners.into_iter().flatten().next().unwrap();
    manager
        .kill(&actual.execution_id, actual.generation)
        .await
        .unwrap();
    manager
        .remove(&actual.execution_id, actual.generation)
        .await
        .unwrap();
}

#[tokio::test]
async fn prewarm_cannot_run_hooks_or_mutate_an_unclaimed_environment() {
    let (_directory, manager, backend) = prewarm_harness();
    for change in 0..3 {
        let mut value = idle_request("bad");
        match change {
            0 => value.config.entrypoint_override = Some(vec!["/package/hook".into()]),
            1 => value.config.image = "mutable:tag".into(),
            _ => value
                .config
                .extra_env
                .push(("TOKEN".into(), "synthetic-secret".into())),
        }
        assert!(manager
            .ensure_prewarmed_reservation(value, &operation("bad"), "engine", "owner")
            .await
            .is_err());
    }
    assert_eq!(backend.starts.load(Ordering::Relaxed), 0);
    assert!(manager.managed_records().await.unwrap().is_empty());
    let warm = manager
        .ensure_prewarmed_reservation(idle_request("pool"), &operation("pool"), "engine", "owner")
        .await
        .unwrap();
    manager
        .start(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    assert!(manager
        .pause(&warm.execution_id, warm.generation, true)
        .await
        .is_err());
    assert!(manager
        .restart_with_options(
            &warm.execution_id,
            warm.generation,
            &operation("restart"),
            RestartExecutionOptions::default()
        )
        .await
        .is_err());
    let record = manager
        .managed_record(&warm.execution_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        record.managed_execution.unwrap().generation,
        warm.generation
    );
    manager
        .kill(&warm.execution_id, warm.generation)
        .await
        .unwrap();
    manager
        .remove(&warm.execution_id, warm.generation)
        .await
        .unwrap();
}
