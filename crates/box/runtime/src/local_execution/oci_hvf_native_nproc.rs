//! Actual nonroot per-UID process budgets, entry guards and recovery.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};
use sha2::{Digest, Sha256};

fn exec(args: &[&str]) -> ExecRequest {
    let mut request = super::exec(args);
    request.user = Some("1001:2002".into());
    request
}

fn configured(image: &str, label: &str, probe: &Path) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.user = Some("1001:2002".into());
    req.config.cap_drop = vec!["ALL".into()];
    req.config.read_only = true;
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777".into()];
    req.config.resource_limits.ulimits = vec!["nproc=16:16".into(), "core=0:0".into()];
    req.config.volumes = vec![format!("{}:/nproc-probe:ro", probe.display())];
    req.config.cmd = vec![
        "/bin/sh".into(),
        "-c".into(),
        "set -eu; /nproc-probe > /scratch/init-ready.json; mv /scratch/init-ready.json /scratch/init.json; exec /bin/sleep 2147483647".into(),
    ];
    req
}

fn verify(bytes: &[u8], tty: bool) -> serde_json::Value {
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().find(|line| line.starts_with('{')).unwrap();
    let value: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["uid"], 1001);
    assert_eq!(value["gid"], 2002);
    assert_eq!(value["emptyCapabilities"], true);
    assert_eq!(value["noNewPrivileges"], true);
    assert_eq!(value["unlimited"], false);
    assert_eq!(value["forkErrno"], 11);
    assert_eq!(value["hardRaiseErrno"], 1);
    let children = value["children"].as_u64().unwrap();
    assert!((1..16).contains(&children));
    assert_eq!(value["reaped"], children);
    assert_eq!(value["budgetReused"], true);
    assert_eq!(value["threadErrno"], 11);
    let threads = value["threads"].as_u64().unwrap();
    assert!((1..16).contains(&threads));
    assert_eq!(value["joined"], threads);
    assert_eq!(value["threadBudgetReused"], true);
    let ns = &value["userNamespace"];
    if ns["entered"] == true {
        assert_eq!(ns["forkErrno"], 11);
        assert!((1..16).contains(&ns["children"].as_u64().unwrap()));
        assert_eq!(ns["reaped"], ns["children"]);
        assert_eq!(ns["budgetReused"], true);
    } else {
        assert!([1, 13, 22, 28].contains(&ns["errno"].as_i64().unwrap()));
    }
    assert_eq!(value["tty"], tty);
    value
}

async fn probe(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
) {
    use base64::Engine as _;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let initial = loop {
        let mut request = behavior::file(kbox_core::FileOp::Download, "/scratch/init.json", None);
        request.max_bytes = Some(4096);
        let result = manager
            .transfer_file(id, generation, request)
            .await
            .unwrap();
        if result.success {
            break base64::engine::general_purpose::STANDARD
                .decode(result.data.unwrap())
                .unwrap();
        }
        assert!(tokio::time::Instant::now() < deadline, "{result:?}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    fs::write(root.join(format!("nproc-{label}-init.stdout")), &initial).unwrap();
    let mut observations = vec![verify(&initial, false)];
    for (name, args) in [
        ("exec", vec!["/nproc-probe"]),
        ("child", vec!["/bin/sh", "-c", "/nproc-probe"]),
    ] {
        let mut command = exec(&args);
        command.timeout_ns = 30_000_000_000;
        let output = manager.execute(id, generation, command).await.unwrap();
        fs::write(
            root.join(format!("nproc-{label}-{name}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            root.join(format!("nproc-{label}-{name}.stderr")),
            &output.stderr,
        )
        .unwrap();
        assert_eq!(
            output.exit_code,
            0,
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        observations.push(verify(&output.stdout, false));
    }
    for tty in [false, true] {
        let mut process = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/nproc-probe".into()],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: Some("1001:2002".into()),
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut command = exec(&["/nproc-probe", "--stdin"]);
            command.timeout_ns = 30_000_000_000;
            command.stdin_streaming = true;
            manager
                .start_process(id, generation, command)
                .await
                .unwrap()
        };
        if !tty {
            process.input().write_stdin(b"streamed\n").await.unwrap();
            process.input().close_stdin().await.unwrap();
        }
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let name = if tty { "pty" } else { "stream" };
        let exit = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match process.next_event().await.unwrap().unwrap() {
                    ExecEvent::Chunk(chunk) => {
                        if chunk.stream == kbox_core::exec::StreamType::Stdout {
                            output.extend(chunk.data);
                        } else {
                            errors.extend(chunk.data);
                        }
                        fs::write(root.join(format!("nproc-{label}-{name}.stdout")), &output)
                            .unwrap();
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        fs::write(root.join(format!("nproc-{label}-{name}.stderr")), &errors).unwrap();
        assert_eq!(
            exit.exit_code,
            0,
            "{}\n{}",
            String::from_utf8_lossy(&output),
            String::from_utf8_lossy(&errors)
        );
        observations.push(verify(&output, tty));
    }
    fs::write(
        root.join(format!("nproc-{label}-observations.json")),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

async fn full_budget_entry_guards(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let mut command = exec(&["/nproc-probe", "--hold"]);
    command.stdin_streaming = true;
    command.timeout_ns = 60_000_000_000;
    let mut process = manager
        .start_process(id, generation, command)
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), async {
        while !String::from_utf8_lossy(&bytes).contains("nproc-quota-full") {
            match process.next_event().await.unwrap().unwrap() {
                ExecEvent::Chunk(chunk) => bytes.extend(chunk.data),
                ExecEvent::FlushAck => {}
                ExecEvent::Exit(exit) => panic!(
                    "quota holder exited {exit:?}: {}",
                    String::from_utf8_lossy(&bytes)
                ),
            }
        }
    })
    .await
    .unwrap();
    for (label, user) in [
        ("implicit", None),
        ("root", Some("0")),
        ("other", Some("1002:2002")),
    ] {
        let mut request = exec(&[
            "/bin/sh",
            "-c",
            &format!("printf forbidden > /scratch/nproc-{label}-entry"),
        ]);
        request.user = user.map(str::to_owned);
        assert!(matches!(
            manager.execute(id, generation, request).await,
            Err(kbox_core::ExecutionManagerError::InvalidRequest(_))
        ));
    }
    let invalid = manager
        .start_pty(
            id,
            generation,
            PtyRequest {
                cmd: vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "printf forbidden > /scratch/nproc-pty-entry".into(),
                ],
                env: vec![],
                working_dir: None,
                rootfs: None,
                user: None,
                cols: 80,
                rows: 24,
            },
        )
        .await;
    assert!(matches!(
        invalid,
        Err(kbox_core::ExecutionManagerError::InvalidRequest(_))
    ));
    let extra = manager
        .execute(
            id,
            generation,
            exec(&[
                "/bin/sh",
                "-c",
                "printf forbidden > /scratch/nproc-full-entry",
            ]),
        )
        .await;
    fs::write(
        root.join("nproc-full-budget-rejected.txt"),
        format!("{extra:?}\n"),
    )
    .unwrap();
    assert!(!matches!(&extra,Ok(output) if output.exit_code==0));
    let requests = [
        exec(&[
            "/bin/sh",
            "-c",
            "printf forbidden > /scratch/nproc-parallel-a",
        ]),
        exec(&[
            "/bin/sh",
            "-c",
            "printf forbidden > /scratch/nproc-parallel-b",
        ]),
    ];
    let (a, b) = tokio::join!(
        manager.execute(id, generation, requests[0].clone()),
        manager.execute(id, generation, requests[1].clone())
    );
    fs::write(
        root.join("nproc-concurrent-budget-rejected.txt"),
        format!("{a:?}\n{b:?}\n"),
    )
    .unwrap();
    for outcome in [a, b] {
        assert!(!matches!(outcome, Ok(output) if output.exit_code == 0));
    }
    for tty in [false, true] {
        let marker = if tty {
            "/scratch/nproc-pty-full-entry"
        } else {
            "/scratch/nproc-stream-full-entry"
        };
        let script = format!("printf forbidden > {marker}");
        let rejected = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/bin/sh".into(), "-c".into(), script.clone()],
                        env: vec![],
                        working_dir: None,
                        rootfs: None,
                        user: Some("1001:2002".into()),
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
        } else {
            let mut command = exec(&["/bin/sh", "-c", &script]);
            command.stdin_streaming = true;
            manager.start_process(id, generation, command).await
        };
        let diagnostic = match rejected {
            Err(error) => format!("{error:?}"),
            Ok(mut session) => {
                let (exit, output) = tokio::time::timeout(Duration::from_secs(30), async {
                    let mut output = Vec::new();
                    loop {
                        match session.next_event().await.unwrap().unwrap() {
                            ExecEvent::Chunk(chunk) => output.extend(chunk.data),
                            ExecEvent::FlushAck => {}
                            ExecEvent::Exit(exit) => break (exit, output),
                        }
                    }
                })
                .await
                .unwrap();
                assert_ne!(exit.exit_code, 0, "full quota process entered");
                format!("{exit:?}\n{}", String::from_utf8_lossy(&output))
            }
        };
        fs::write(
            root.join(if tty {
                "nproc-pty-full-rejected.txt"
            } else {
                "nproc-stream-full-rejected.txt"
            }),
            diagnostic,
        )
        .unwrap();
    }
    process.input().write_stdin(b"release\n").await.unwrap();
    process.input().close_stdin().await.unwrap();
    let exit = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match process.next_event().await.unwrap().unwrap() {
                ExecEvent::Chunk(chunk) => bytes.extend(chunk.data),
                ExecEvent::FlushAck => {}
                ExecEvent::Exit(exit) => break exit,
            }
        }
    })
    .await
    .unwrap();
    fs::write(root.join("nproc-full-budget.stdout"), &bytes).unwrap();
    assert_eq!(exit.exit_code, 0);
    verify(&bytes, false);
    let checked=manager.execute(id,generation,exec(&["/bin/sh","-c","test ! -e /scratch/nproc-implicit-entry && test ! -e /scratch/nproc-root-entry && test ! -e /scratch/nproc-other-entry && test ! -e /scratch/nproc-pty-entry && test ! -e /scratch/nproc-full-entry && test ! -e /scratch/nproc-parallel-a && test ! -e /scratch/nproc-parallel-b && test ! -e /scratch/nproc-pty-full-entry && test ! -e /scratch/nproc-stream-full-entry"])).await.unwrap();
    assert_eq!(checked.exit_code, 0);
}

async fn guest_rlimit_units(manager: &LocalExecutionManager, root: &Path, image: &str) {
    let binary = PathBuf::from(
        std::env::var_os("K_BOX_TEST_NPROC_GUEST_TESTS").expect("exact Linux Agent test ELF"),
    );
    let digest = format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()));
    let mut req = request(image, "nproc-agent-units");
    req.config.resource_limits.ulimits = vec!["core=0:0".into()];
    req.config.volumes = vec![format!("{}:/nproc-agent-tests:ro", binary.display())];
    let lease = manager
        .create_and_start(req, &OperationId::new("nproc-agent-unit-create").unwrap())
        .await
        .unwrap();
    let output = manager
        .execute(
            &lease.execution_id,
            lease.generation,
            super::exec(&[
                "/nproc-agent-tests",
                "executor::rlimit::",
                "--nocapture",
                "--test-threads=1",
            ]),
        )
        .await
        .unwrap();
    fs::write(root.join("nproc-guest-units.stdout"), &output.stdout).unwrap();
    fs::write(root.join("nproc-guest-units.stderr"), &output.stderr).unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("test result: ok. 5 passed; 0 failed; 0 ignored;"),
        "{text}"
    );
    assert!(
        text.contains("nproc-admission-case=zero transitioned=true soft=0"),
        "{text}"
    );
    assert!(
        text.contains("nproc-admission-case=one transitioned=true soft=1"),
        "{text}"
    );
    fs::write(root.join("nproc-guest-units.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","actualCounts":[5,0,0],"sha256":digest,"nativeGuest":"Linux arm64 on macOS HVF","zeroRefused":true,"lastSlotAdmittedAndHardLimitRestored":true,"noAdmissionChildLeft":true})).unwrap()).unwrap();
    manager
        .kill(&lease.execution_id, lease.generation)
        .await
        .unwrap();
    manager
        .remove(&lease.execution_id, lease.generation)
        .await
        .unwrap();
}

fn cli(root: &Path, image: &str, probe: &Path) -> bool {
    let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_CLI") else {
        return false;
    };
    let run = |args: &[&str]| {
        let output = Command::new(&binary)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", "microvm")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let bind = format!("{}:/nproc-probe:ro", probe.display());
    let id = String::from_utf8(run(&[
        "run",
        "-d",
        "--cpus",
        "1",
        "--memory",
        "512",
        "--network",
        "none",
        "--user",
        "1001:2002",
        "--cap-drop",
        "ALL",
        "--read-only",
        "--tmpfs",
        "/scratch:size=1m,mode=1777",
        "--health-cmd",
        "/nproc-probe --health",
        "--health-interval",
        "1",
        "--ulimit",
        "nproc=16:16",
        "--ulimit",
        "core=0:0",
        "-v",
        &bind,
        image,
        "--",
        "/bin/sleep",
        "2147483647",
    ]))
    .unwrap()
    .trim()
    .to_owned();
    let before: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(before[0]["managed_execution"]["runtime_route"], "oci_sdk");
    verify(&run(&["exec", &id, "--", "/nproc-probe"]), false);
    for user in ["0", "1002:2002"] {
        let script = format!("printf forbidden > /scratch/nproc-cli-{user}-entry");
        let output = Command::new(&binary)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", "microvm")
            .args(["exec", "--user", user, &id, "--", "/bin/sh", "-c", &script])
            .output()
            .unwrap();
        fs::write(
            root.join(format!("nproc-cli-user-{user}.stderr")),
            &output.stderr,
        )
        .unwrap();
        assert!(!output.status.success());
        assert!(run(&[
            "exec",
            &id,
            "--",
            "/bin/sh",
            "-c",
            &format!("test ! -e /scratch/nproc-cli-{user}-entry")
        ])
        .is_empty());
    }

    let health_deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let status: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
        if status[0]["health_status"] == "healthy" {
            break;
        }
        assert!(std::time::Instant::now() < health_deadline, "{status}");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        run(&["exec", &id, "--", "/bin/cat", "/scratch/nproc-health-user"]),
        b"1001:2002\n"
    );

    run(&["restart", &id]);
    let after: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(before[0]["id"], after[0]["id"]);
    assert_eq!(
        after[0]["managed_execution"]["generation"]
            .as_u64()
            .unwrap(),
        before[0]["managed_execution"]["generation"]
            .as_u64()
            .unwrap()
            + 1
    );
    verify(&run(&["exec", &id, "--", "/nproc-probe"]), false);
    let script = PathBuf::from(std::env::var_os("K_BOX_TEST_NPROC_CLI_SCRIPT").unwrap());
    assert!(script.is_absolute() && script.is_file());
    let output = Command::new("/usr/bin/python3")
        .arg(&script)
        .arg(&binary)
        .arg(&id)
        .arg(root.join("nproc-cli-pty"))
        .env("K_HOME", root.join("home"))
        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
        .env("K_BOX_OCI_MIGRATION", "microvm")
        .output()
        .unwrap();
    fs::write(root.join("nproc-cli-pty-driver.stdout"), &output.stdout).unwrap();
    fs::write(root.join("nproc-cli-pty-driver.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pty: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("nproc-cli-pty/result.json")).unwrap()).unwrap();
    assert_eq!(pty["status"], "PASS");
    assert_eq!(pty["cases"].as_array().unwrap().len(), 2);
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned home,installed HVF assets and immutable Linux rlimit probe"]
fn native_nproc_enforces_nonroot_budget_all_process_paths_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let binary=PathBuf::from(std::env::var_os("K_BOX_TEST_NPROC_PROBE").unwrap());assert!(binary.is_absolute() && binary.is_file());
                let first=manager.create_and_start(configured(&image,"nproc-main",&binary),&OperationId::new("nproc-main-create").unwrap()).await.unwrap();let id=&first.execution_id;let mut lease=first.clone();let mut generations=Vec::new();
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();let config:serde_json::Value=serde_json::from_slice(&fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap()).unwrap();fs::write(root.join(format!("nproc-{label}-spec.json")),serde_json::to_vec_pretty(&config).unwrap()).unwrap();
                    let manifest:serde_json::Value=serde_json::from_slice(&fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join(".koci-agent-vm-attachments.json")).unwrap()).unwrap();
                    assert_eq!(manifest["schemaVersion"],"koci.agent-vm-attachments.v10");
                    assert_eq!(manifest["nprocTransportSchema"],"koci.agent-vm-attachments.v6");
                    fs::write(root.join(format!("nproc-{label}-manifest.json")),serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
                    probe(&manager,&root,id,lease.generation,label).await;
                    if wave==0 {
                        guest_rlimit_units(&manager,&root,&image).await;
                        full_budget_entry_guards(&manager,&root,id,lease.generation).await;
                        let mut command=exec(&["/bin/sh","-c","count=$(cat /scratch/replay-count 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay-count; /nproc-probe"]);command.request_id=Some("limit-replay".into());let output=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.exit_code,0);verify(&output.stdout,false);let replay=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.stdout,replay.stdout);assert_eq!(manager.execute(id,lease.generation,exec(&["/bin/cat","/scratch/replay-count"])).await.unwrap().stdout,b"1");command.cmd=vec!["/bin/echo".into(),"changed".into()];assert!(manager.execute(id,lease.generation,command).await.is_err());}
                    if wave<2 {
                        let old=lease.generation;
                        if wave==1 {host.sigkill();assert!(manager.inspect(id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline{tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(id,old,&OperationId::new(format!("nproc-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                let positive=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let control_binary=root.join("nproc-unlimited-probe");
                assert_eq!(fs::copy(&binary,&control_binary).unwrap(),fs::metadata(&binary).unwrap().len());
                let mut req=configured(&image,"nproc-unlimited-control",&control_binary);
                req.config.resource_limits.ulimits=vec!["nproc=-1:-1".into(),"core=0:0".into()];
                req.config.cmd=vec!["/bin/sleep".into(),"2147483647".into()];
                let control=positive.create_and_start(req,&OperationId::new("nproc-unlimited-control").unwrap()).await.unwrap();
                let output=positive.execute(&control.execution_id,control.generation,exec(&["/nproc-probe","--unlimited"])).await.unwrap();
                assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));let unlimited:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(unlimited["status"],"PASS");assert_eq!(unlimited["unlimited"],true);assert_eq!(unlimited["children"],32);assert_eq!(unlimited["reaped"],32);assert_eq!(unlimited["forkErrno"],0);assert_eq!(unlimited["budgetReused"],true);assert_eq!(unlimited["threads"],32);assert_eq!(unlimited["joined"],32);assert_eq!(unlimited["threadErrno"],0);assert_eq!(unlimited["threadBudgetReused"],true);
                fs::write(root.join("nproc-unlimited-control.json"),&output.stdout).unwrap();
                let output=positive.execute(&control.execution_id,control.generation,exec(&["/bin/sh","-c","printf '%s:%s' $(ulimit -Sn) $(ulimit -Hn)"])).await.unwrap();assert_eq!(output.exit_code,0);assert_eq!(output.stdout,b"1024:1024");
                let output=manager.execute(id,lease.generation,exec(&["/nproc-probe"])).await.unwrap();assert_eq!(output.exit_code,0);verify(&output.stdout,false);fs::write(root.join("nproc-after-control.stdout"),&output.stdout).unwrap();
                positive.kill(&control.execution_id,control.generation).await.unwrap();positive.remove(&control.execution_id,control.generation).await.unwrap();
                manager.kill(id,lease.generation).await.unwrap();manager.remove(id,lease.generation).await.unwrap();
                let cli_checked=cli(&root,&image,&binary);fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":id,"generations":generations,"forkEagain":11,"nonrootUid":1001,"childReapingAndReuse":true,"rootImplicitOtherUidAndPtyRefusedBeforeEntry":true,"extraExecAtFullBudgetRefusedBeforeEntry":true,"cliHealthRealPtyAttach":true,"namespaceBypassRefusedOrBounded":true,"unlimitedControl":true,"independentExecutionLimits":true,"hardRaiseEperm":true,"initExecChildStreamRealPty":true,"replayOneEffect":true,"changedRequestRefused":true,"defaultNofileRetained":[1024,1024],"defaultControlRoute":"oci_sdk","cliChecked":cli_checked,"stoppedRecovery":true,"processLimitScope":"configured nonroot real-UID process/thread budget, independent of cgroup pids"})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires exact installed current and old HVF assets in an owned root"]
fn native_old_nproc_assets_refuse_without_workload_or_route_fallback() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host=Some(Host::start(&root,1));
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                if std::env::var("K_BOX_TEST_NPROC_OLD_ASSET").as_deref()==Ok("old-agent") {
                    let proof_path=PathBuf::from(std::env::var_os("K_BOX_TEST_NPROC_OLD_AGENT_PROOF").expect("native old Agent decoder proof"));
                    let proof:serde_json::Value=serde_json::from_slice(&fs::read(&proof_path).unwrap()).unwrap();
                    let old=PathBuf::from(std::env::var_os("K_BOX_TEST_NPROC_OLD_AGENT").unwrap());
                    let image=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_MANIFEST").unwrap());
                    assert_eq!(proof["status"],"PASS");assert_eq!(proof["schema"],"koci.agent-vm-attachments.v10");assert_eq!(proof["guestAgentExitCode"],1);
                    assert_eq!(proof["agentSha256"],format!("{:x}",Sha256::digest(fs::read(old).unwrap())));
                    assert_eq!(proof["oldSystemImageManifestSha256"],format!("{:x}",Sha256::digest(fs::read(image).unwrap())));
                    for field in ["guestDecoderRefusedBeforeToken","socketDirectoryRemoved","workerReaped"] {assert_eq!(proof[field],true);}
                    assert!(proof["console"].as_str().unwrap().contains("unknown field `nprocTransportSchema`"));
                    fs::write(root.join("old-nproc-agent-schema-proof.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
                }
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();
                let binary=PathBuf::from(std::env::var_os("K_BOX_TEST_NPROC_PROBE").unwrap());
                let entry=root.join("compat-entry");fs::create_dir(&entry).unwrap();
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&entry,fs::Permissions::from_mode(0o777)).unwrap();
                let marker=entry.join("entered");
                let mut req=configured(&image,"nproc-compatibility",&binary);
                req.config.volumes.push(format!("{}:/compat:rw",entry.display()));
                req.config.cmd=vec!["/bin/sh".into(),"-c".into(),"printf forbidden > /compat/entered; exec /bin/sleep 2147483647".into()];
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let refused=manager.create_and_start(req,&OperationId::new("nproc-compat-create").unwrap()).await.expect_err("old assets must refuse guarded nproc");
                fs::write(root.join("nproc-compatibility-refusal.txt"),format!("{refused:?}\n")).unwrap();
                assert!(refused.to_string().contains(&std::env::var("K_BOX_TEST_NPROC_EXPECTED_REFUSAL").unwrap()),"{refused}");
                assert!(!marker.exists());
                let records=ManagedExecutionStore::new(root.join("home/boxes.json")).list().unwrap();
                fs::write(root.join("nproc-compatibility-records.json"),serde_json::to_vec_pretty(&records).unwrap()).unwrap();
                if !records.is_empty() {
                    assert_eq!(records.len(),1);let id=ExecutionId::new(records[0].id.clone()).unwrap();
                    assert!(matches!(manager.kill(&id,ExecutionGeneration::INITIAL).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    assert!(manager.remove(&id,ExecutionGeneration::INITIAL).await.is_err());drop(manager);drop(host.take());
                    let shim=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_SHIM").unwrap());
                    let manifest=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_MANIFEST").unwrap());
                    host=Some(Host::start_with_assets(&root,2,&shim,&manifest));assert_eq!(pending::drain(&root).await,1);
                    let proof:serde_json::Value=serde_json::from_slice(&fs::read(root.join("pending-create-cleanup.json")).unwrap()).unwrap();assert_eq!(proof["workloadStartsAdded"],0);
                }
                cleanup(&root).await;assert!(!marker.exists());
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","oldAssetRefused":true,"workloadMarkerAbsent":true,"noRouteOrWritableFallback":true,"cleanupWithoutStart":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;if let Err(error)=result{std::panic::resume_unwind(error);}
        });drop(host);assert!(!root.join("service/runtime.sock").exists());
    }).unwrap().join().unwrap();
}
