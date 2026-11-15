use super::*;

#[test]
fn runtime_owner_keeps_worker_alive_after_the_smoke_deadline() {
    let mut child = Command::new("/bin/sleep").arg("0.1").spawn().unwrap();
    let result = wait_for_agent_worker(
        &mut child,
        WorkerLifetime::RuntimeOwner,
        Duration::from_millis(1),
    )
    .unwrap();
    assert!(!result.timed_out);
    assert!(result.status.success());
}

#[test]
fn standalone_smoke_still_terminates_and_reaps_a_timed_out_worker() {
    let mut child = Command::new("/bin/sleep").arg("60").spawn().unwrap();
    let result =
        wait_for_agent_worker(&mut child, WorkerLifetime::Smoke, Duration::from_millis(1)).unwrap();
    assert!(result.timed_out);
    assert!(!result.status.success());
    assert!(child.try_wait().unwrap().is_some());
}
