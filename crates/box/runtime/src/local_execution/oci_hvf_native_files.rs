//! Native single-file lifecycle and privileged Host PEP matrix using owned sources.
use super::*;
use kbox_core::{ExecutionGeneration, ExecutionId, FileOp};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn caller(root: &Path, label: &str) -> PathBuf {
    root.join(format!("caller-files-{label}"))
}
pub(super) fn configure(root: &Path, label: &str, request: &mut CreateExecutionRequest) {
    let parent = caller(root, label);
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o777)).unwrap();
    fs::write(parent.join("只读 input"), b"initial-file").unwrap();
    fs::write(parent.join("empty rw"), b"").unwrap();
    fs::write(parent.join("canary"), b"private-sibling").unwrap();
    for name in ["只读 input", "empty rw"] {
        fs::set_permissions(parent.join(name), fs::Permissions::from_mode(0o666)).unwrap();
    }
    request.config.volumes.extend([
        format!("{}:/exact-ro:ro", parent.join("只读 input").display()),
        format!("{}:/exact-rw:rw", parent.join("empty rw").display()),
    ]);
    request.config.cap_add = vec!["SYS_ADMIN".into()];
}

pub(super) async fn check(
    manager: &LocalExecutionManager,
    root: &Path,
    label: &str,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let parent = caller(root, label);
    let before = fs::metadata(parent.join("只读 input")).unwrap();
    let before_rw = fs::metadata(parent.join("empty rw")).unwrap();
    let first_read = manager
        .execute(id, generation, exec(&["/bin/cat", "/exact-ro"]))
        .await
        .unwrap();
    assert_eq!(first_read.exit_code, 0);
    assert!(!first_read.stdout.is_empty());
    let value = format!(
        "live host {label} generation {} changed length",
        generation.get()
    );
    fs::write(parent.join("只读 input"), value.as_bytes()).unwrap();
    let read = manager
        .execute(id, generation, exec(&["/bin/cat", "/exact-ro"]))
        .await
        .unwrap();
    assert_eq!(
        read.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert_eq!(read.stdout, value.as_bytes());
    let bytes =
        fs::read(std::env::var_os("K_BOX_TEST_HVF_FILE_PROBE").expect("fixed native file probe"))
            .unwrap();
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                behavior::file(FileOp::Upload, "/file-probe", Some(bytes))
            )
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        manager
            .execute(id, generation, exec(&["/bin/chmod", "755", "/file-probe"]))
            .await
            .unwrap()
            .exit_code,
        0
    );
    let result = manager
        .execute(id, generation, exec(&["/file-probe"]))
        .await
        .unwrap();
    fs::write(
        root.join(format!("files-{label}-g{}-probe.stderr", generation.get())),
        &result.stderr,
    )
    .unwrap();
    assert_eq!(
        result.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let proof: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(proof["hostDenials"], 16);
    assert_eq!(proof["childDenied"], true);
    assert_eq!(proof["freshMounts"], 2);
    assert_eq!(
        fs::read(parent.join("empty rw")).unwrap(),
        b"native-writeback"
    );
    assert_eq!(
        fs::read(parent.join("只读 input")).unwrap(),
        value.as_bytes()
    );
    assert_eq!(fs::read(parent.join("canary")).unwrap(), b"private-sibling");
    for (path, old) in [
        (parent.join("只读 input"), before),
        (parent.join("empty rw"), before_rw),
    ] {
        let now = fs::metadata(&path).unwrap();
        assert_eq!(
            (now.dev(), now.ino(), now.uid(), now.gid(), now.mode()),
            (old.dev(), old.ino(), old.uid(), old.gid(), old.mode())
        );
        assert_eq!(now.nlink(), 2);
    }
    fs::write(root.join(format!("files-{label}-g{}.json",generation.get())),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":id,"generation":generation,"liveRead":value,"rwBytes":"native-writeback","proof":proof})).unwrap()).unwrap();
}

pub(super) async fn exercise(manager: &LocalExecutionManager, root: &Path, image: &str) {
    let mut a = request(image, "file-alpha");
    configure(root, "alpha", &mut a);
    let mut b = request(image, "file-beta");
    configure(root, "beta", &mut b);
    let first = manager
        .create_and_start(a, &OperationId::new("file-alpha-create").unwrap())
        .await
        .unwrap();
    let second = manager
        .create_and_start(b, &OperationId::new("file-beta-create").unwrap())
        .await
        .unwrap();
    check(
        manager,
        root,
        "alpha",
        &first.execution_id,
        first.generation,
    )
    .await;
    check(
        manager,
        root,
        "beta",
        &second.execution_id,
        second.generation,
    )
    .await;
    assert!(manager
        .execute(
            &second.execution_id,
            first.generation,
            exec(&["/bin/cat", "/exact-ro"])
        )
        .await
        .unwrap()
        .stdout
        .starts_with(b"live host beta"));
    let restarted = manager
        .restart(
            &first.execution_id,
            first.generation,
            &OperationId::new("file-alpha-normal-restart").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restarted.generation.get(), first.generation.get() + 1);
    assert!(manager
        .execute(
            &first.execution_id,
            first.generation,
            exec(&["/bin/sh", "-c", "echo stale > /exact-rw"])
        )
        .await
        .is_err());
    check(
        manager,
        root,
        "alpha",
        &first.execution_id,
        restarted.generation,
    )
    .await;
    manager
        .kill(&first.execution_id, restarted.generation)
        .await
        .unwrap();
    manager
        .remove(&first.execution_id, restarted.generation)
        .await
        .unwrap();
    assert_eq!(
        fs::metadata(caller(root, "alpha").join("只读 input"))
            .unwrap()
            .nlink(),
        1
    );
    assert_eq!(
        manager
            .execute(
                &second.execution_id,
                second.generation,
                exec(&["/bin/cat", "/exact-rw"])
            )
            .await
            .unwrap()
            .stdout,
        b"native-writeback"
    );
    // Replacing the source pathname while running never replaces the inode granted
    // to that guest; restart must retain the original reservation and refuse it.
    let parent = caller(root, "beta");
    fs::rename(parent.join("只读 input"), parent.join("retained")).unwrap();
    fs::write(
        parent.join("只读 input"),
        b"replacement-must-not-be-granted",
    )
    .unwrap();
    let old = manager
        .execute(
            &second.execution_id,
            second.generation,
            exec(&["/bin/cat", "/exact-ro"]),
        )
        .await
        .unwrap();
    assert!(old.stdout.starts_with(b"live host beta"));
    assert!(manager
        .restart(
            &second.execution_id,
            second.generation,
            &OperationId::new("file-beta-replacement-refusal").unwrap()
        )
        .await
        .is_err());
    let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
        .get(&second.execution_id)
        .unwrap()
        .unwrap();
    let generation = record.managed_execution.as_ref().unwrap().generation;
    manager
        .remove(&second.execution_id, generation)
        .await
        .unwrap();
    assert_eq!(
        fs::read(parent.join("只读 input")).unwrap(),
        b"replacement-must-not-be-granted"
    );
    assert_eq!(fs::metadata(parent.join("retained")).unwrap().nlink(), 1);
    fs::write(root.join("files-lifecycle.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","executions":2,"normalRestart":true,"staleGenerationRefused":true,"concurrentSourcesIndependent":true,"activeReplacementPinned":true,"replacementRestartRefused":true,"callerFilesRetained":true})).unwrap()).unwrap();
}
