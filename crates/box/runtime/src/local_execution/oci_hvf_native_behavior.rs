use super::*;
use base64::Engine as _;
use kbox_core::{
    ExecutionGeneration, ExecutionId, ExecutionResourceUpdate, FileOp, FileRequest, FilesystemOp,
    FilesystemRequest,
};

pub(super) fn configure(root: &Path, request: &mut CreateExecutionRequest) {
    for name in ["readonly", "writable"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    fs::create_dir(root.join("readonly/empty")).unwrap();
    fs::write(root.join("readonly/input"), b"caller-owned").unwrap();
    fs::write(root.join("readonly/replacement"), b"replacement").unwrap();
    fs::write(root.join("ungranted"), b"outside-caller-grants").unwrap();
    std::os::unix::fs::symlink(root.join("ungranted"), root.join("writable/ungranted-link"))
        .unwrap();
    request.config.volumes = vec![
        format!("{}:/readonly:ro", root.join("readonly").display()),
        format!("{}:/writable:rw", root.join("writable").display()),
    ];
    request.config.cap_add = vec!["SYS_ADMIN".into()];
    request.config.resource_limits.cpu_quota = Some(50_000);
    request.config.resource_limits.cpu_period = Some(100_000);
    request.config.resource_limits.cpu_shares = Some(512);
    request.config.resource_limits.sandbox_memory_limit_bytes = Some(128 * 1024 * 1024);
    request.config.resource_limits.memory_reservation = Some(16 * 1024 * 1024);
    request.config.resource_limits.memory_swap = Some(-1);
    request.config.resource_limits.pids_limit = Some(32);
}

pub(super) fn file(op: FileOp, path: &str, data: Option<Vec<u8>>) -> FileRequest {
    FileRequest {
        op,
        guest_path: path.into(),
        data: data.map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes)),
        user: None,
        max_bytes: None,
        request_id: Some(format!("native-file-{path}")),
    }
}

pub(super) async fn readonly_probe(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) -> serde_json::Value {
    let bytes =
        fs::read(std::env::var_os("K_BOX_TEST_HVF_PROBE").expect("fixed native readonly probe"))
            .unwrap();
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                file(FileOp::Upload, "/readonly-probe", Some(bytes))
            )
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        manager
            .execute(
                id,
                generation,
                exec(&["/bin/chmod", "755", "/readonly-probe"])
            )
            .await
            .unwrap()
            .exit_code,
        0
    );
    let result = manager
        .execute(id, generation, exec(&["/readonly-probe"]))
        .await
        .unwrap();
    assert_eq!(
        result.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let probe: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(probe["hostDenials"], 14);
    assert_eq!(probe["guestChildDenied"], true);
    let mounts = manager
        .execute(
            id,
            generation,
            exec(&[
                "/bin/sh",
                "-c",
                "cat /proc/self/mountinfo; ls -la /readonly /writable; sync",
            ]),
        )
        .await
        .unwrap();
    fs::write(
        root.join(format!("guest-mounts-{}.txt", generation.get())),
        &mounts.stdout,
    )
    .unwrap();
    fs::write(
        root.join(format!("readonly-probe-{}.json", generation.get())),
        &result.stdout,
    )
    .unwrap();
    assert_eq!(
        fs::read(root.join("readonly/input")).unwrap(),
        b"caller-owned"
    );
    let outside = manager
        .transfer_file(
            id,
            generation,
            file(FileOp::Download, "/writable/ungranted-link", None),
        )
        .await;
    assert!(match outside {
        Err(_) => true,
        Ok(value) => !value.success,
    });
    assert_eq!(
        fs::read(root.join("ungranted")).unwrap(),
        b"outside-caller-grants"
    );
    assert_eq!(fs::read(root.join("writable/output")).unwrap(), b"writable");
    probe
}

pub(super) async fn exercise(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let probe = readonly_probe(manager, root, id, generation).await;
    let downloaded = manager
        .transfer_file(
            id,
            generation,
            file(FileOp::Download, "/writable/output", None),
        )
        .await
        .unwrap();
    assert!(downloaded.success);
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(downloaded.data.unwrap())
            .unwrap(),
        b"writable"
    );
    let blocked = manager
        .transfer_file(
            id,
            generation,
            file(FileOp::Upload, "/readonly/input", Some(b"denied".to_vec())),
        )
        .await;
    assert!(match blocked {
        Err(_) => true,
        Ok(value) => !value.success,
    });
    assert_eq!(
        fs::read(root.join("readonly/input")).unwrap(),
        b"caller-owned"
    );
    manager
        .filesystem(
            id,
            generation,
            FilesystemRequest {
                op: FilesystemOp::MakeDir,
                path: "/writable/typed 目录".into(),
                destination: None,
                depth: 0,
                user: None,
                request_id: Some("native-fs-mkdir".into()),
            },
        )
        .await
        .unwrap();
    assert!(root.join("writable/typed 目录").is_dir());
    let readback = manager.execute(id,generation,exec(&["/bin/sh","-c","cat /sys/fs/cgroup/cpu.max /sys/fs/cgroup/memory.max /sys/fs/cgroup/memory.low /sys/fs/cgroup/pids.max"])).await.unwrap();
    assert_eq!(
        readback.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&readback.stderr)
    );
    assert_eq!(
        String::from_utf8(readback.stdout)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        ["50000", "100000", "134217728", "16777216", "32"]
    );
    manager
        .update_resources(
            id,
            generation,
            &OperationId::new("native-update-pids").unwrap(),
            ExecutionResourceUpdate {
                pids_limit: Some(24),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let updated = manager
        .execute(
            id,
            generation,
            exec(&["/bin/cat", "/sys/fs/cgroup/pids.max"]),
        )
        .await
        .unwrap();
    assert_eq!(updated.exit_code, 0);
    assert_eq!(updated.stdout, b"24\n");
    let mut command = exec(&[
        "/bin/sh",
        "-c",
        "read line; printf 'received:%s' \"$line\"; printf stream-stderr >&2; exit 7",
    ]);
    command.stdin_streaming = true;
    let mut process = manager
        .start_process(id, generation, command)
        .await
        .unwrap();
    let input = process.input();
    input.write_stdin(b"hello\n").await.unwrap();
    input.close_stdin().await.unwrap();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let exit = loop {
        match process.next_event().await.unwrap().unwrap() {
            kbox_core::exec::ExecEvent::Chunk(chunk) => match chunk.stream {
                kbox_core::exec::StreamType::Stdout => stdout.extend(chunk.data),
                kbox_core::exec::StreamType::Stderr => stderr.extend(chunk.data),
            },
            kbox_core::exec::ExecEvent::Exit(exit) => break exit,
            kbox_core::exec::ExecEvent::FlushAck => {}
        }
    };
    assert_eq!(stdout, b"received:hello");
    assert_eq!(stderr, b"stream-stderr");
    assert_eq!(exit.exit_code, 7);
    manager.stats(id, generation).await.unwrap();
    let late = ExecutionGeneration::new(generation.get() + 1).unwrap();
    assert!(manager
        .execute(id, late, exec(&["/bin/touch", "/writable/old-generation"]))
        .await
        .is_err());
    assert!(!root.join("writable/old-generation").exists());
    let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
    let original = store.get(id).unwrap().unwrap();
    for variant in 0..4 {
        let mut invalid = original.clone();
        let metadata = invalid.managed_execution.as_mut().unwrap();
        match variant {
            0 => metadata.oci_runtime = None,
            1 => metadata.oci_runtime.as_mut().unwrap().target.generation = None,
            2 => {
                metadata.oci_runtime.as_mut().unwrap().target.id =
                    koci_sdk::ContainerId::new("foreign-runtime").unwrap()
            }
            _ => metadata.oci_runtime.as_mut().unwrap().driver = koci_sdk::DriverKind::LibkWhpx,
        }
        assert!(
            manager.backend.inspect(&invalid).await.is_err(),
            "binding variant {variant} was admitted"
        );
    }
    assert_eq!(
        store
            .get(id)
            .unwrap()
            .unwrap()
            .managed_execution
            .unwrap()
            .oci_runtime,
        original.managed_execution.unwrap().oci_runtime
    );
    fs::write(root.join("behavior.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","readonlyProbe":probe,"fileRoundtrip":true,"readonlyFileApiRefused":true,"filesystemGrant":true,"resourceReadback":true,"resourceUpdate":true,"oldGenerationRefused":true,"bindingVariantsRefused":4})).unwrap()).unwrap();
}
