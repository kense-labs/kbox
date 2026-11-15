//! Installed real-time scheduling ceilings and CPU watchdog effects.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};

fn exec(args: &[&str]) -> ExecRequest {
    let mut request = super::exec(args);
    request.user = None;
    request
}

fn configured(image: &str, label: &str, probe: &Path) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.user = Some("0".into());
    req.config.cap_drop = vec!["ALL".into()];
    req.config.read_only = true;
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777".into()];
    req.config.resource_limits.ulimits = vec![
        "rtprio=5:5".into(),
        "rttime=100000:300000".into(),
        "core=0:0".into(),
    ];
    req.config.volumes = vec![format!("{}:/realtime-probe:ro", probe.display())];
    req.config.cmd = vec![
        "/bin/sh".into(),
        "-c".into(),
        "set -eu; /realtime-probe > /scratch/init-ready.json; mv /scratch/init-ready.json /scratch/init.json; exec /bin/sleep 2147483647".into(),
    ];
    req
}

fn verify(bytes: &[u8], tty: bool) -> serde_json::Value {
    verify_user(bytes, tty, 0, 0)
}

fn verify_user(bytes: &[u8], tty: bool, uid: u32, gid: u32) -> serde_json::Value {
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().find(|line| line.starts_with('{')).unwrap();
    let value: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["uid"], uid);
    assert_eq!(value["gid"], gid);
    assert_eq!(value["unlimited"], false);
    assert_eq!(value["zero"], false);
    assert_eq!(value["priorityCeiling"], 5);
    assert_eq!(value["priorityErrno"], 1);
    assert_eq!(value["hardRaiseErrno"], 1);
    assert_eq!(value["timeHardRaiseErrno"], 1);
    assert_eq!(value["softSignal"], 24);
    assert_eq!(value["softSignalCode"], 128);
    assert_eq!(value["hardSignal"], 9);
    assert!(value["softCpuMicros"].as_u64().unwrap() >= 80000);
    assert!(value["hardCpuMicros"].as_u64().unwrap() >= 280000);
    for key in ["blockingResetCpuMicros", "normalCpuMicros"] {
        assert!(value[key].as_u64().unwrap() > 300000, "{key}");
    }
    for key in ["childInheritedPolicy", "threadInheritedPolicy"] {
        assert_eq!(value[key], 2, "{key}");
    }
    for key in ["childInheritedPriority", "threadInheritedPriority"] {
        assert_eq!(value[key], 5, "{key}");
    }
    for key in ["childPriorityErrno", "threadPriorityErrno"] {
        assert_eq!(value[key], 1, "{key}");
    }
    for key in [
        "emptyCapabilities",
        "noNewPrivileges",
        "allowedRangeReused",
        "childReaped",
        "threadJoined",
        "parentSchedulerRestored",
        "parentTimeLimitUnchanged",
    ] {
        assert_eq!(value[key], true, "{key}");
    }
    let ns = &value["userNamespace"];
    if ns["entered"] == true {
        assert_eq!(ns["mapped"], uid != 0);
        assert_eq!(ns["mappingErrno"], if uid == 0 { 1 } else { 0 });
        assert_eq!(ns["priority"], 5);
        assert_eq!(ns["priorityErrno"], 1);
        assert_eq!(ns["childReaped"], true);
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
    loop {
        let ready = manager
            .execute(
                id,
                generation,
                exec(&["/bin/sh", "-c", "test -s /scratch/init.json"]),
            )
            .await
            .unwrap();
        if ready.exit_code == 0 {
            break;
        }
        assert_eq!(ready.exit_code, 1, "{ready:?}");
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let mut download = behavior::file(kbox_core::FileOp::Download, "/scratch/init.json", None);
    download.max_bytes = Some(8192);
    let result = manager
        .transfer_file(id, generation, download)
        .await
        .unwrap();
    assert!(result.success);
    let initial = base64::engine::general_purpose::STANDARD
        .decode(result.data.unwrap())
        .unwrap();
    fs::write(root.join(format!("realtime-{label}-init.stdout")), &initial).unwrap();
    let mut observations = vec![verify(&initial, false)];
    for (name, args) in [
        ("exec", vec!["/realtime-probe"]),
        ("child", vec!["/bin/sh", "-c", "/realtime-probe"]),
    ] {
        let mut command = exec(&args);
        command.timeout_ns = 30_000_000_000;
        let output = manager.execute(id, generation, command).await.unwrap();
        fs::write(
            root.join(format!("realtime-{label}-{name}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            root.join(format!("realtime-{label}-{name}.stderr")),
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
                        cmd: vec!["/realtime-probe".into()],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: None,
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut command = exec(&["/realtime-probe", "--stdin"]);
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
                        fs::write(
                            root.join(format!("realtime-{label}-{name}.stdout")),
                            &output,
                        )
                        .unwrap();
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        fs::write(
            root.join(format!("realtime-{label}-{name}.stderr")),
            &errors,
        )
        .unwrap();
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
        root.join(format!("realtime-{label}-observations.json")),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

async fn zero_control(manager: &LocalExecutionManager, root: &Path, image: &str, probe: &Path) {
    for (label, limits, argument) in [
        ("priority", ["rtprio=0:0", "rttime=100000:300000"], "--zero"),
        ("time", ["rtprio=5:5", "rttime=0:0"], "--zero-time"),
    ] {
        let binary = root.join(format!("realtime-zero-{label}-probe"));
        fs::copy(probe, &binary).unwrap();
        let mut request = configured(image, &format!("realtime-zero-{label}"), &binary);
        request.config.resource_limits.ulimits = limits
            .iter()
            .map(|value| value.to_string())
            .chain(["core=0:0".into()])
            .collect();
        request.config.cmd = vec!["/bin/sleep".into(), "2147483647".into()];
        let lease = manager
            .create_and_start(
                request,
                &OperationId::new(format!("realtime-zero-{label}-create")).unwrap(),
            )
            .await
            .unwrap();
        let record = ManagedExecutionStore::new(root.join("home/boxes.json"))
            .get(&lease.execution_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            record.managed_execution.unwrap().runtime_route,
            ManagedRuntimeRoute::OciSdk
        );
        let output = manager
            .execute(
                &lease.execution_id,
                lease.generation,
                exec(&["/realtime-probe", argument]),
            )
            .await
            .unwrap();
        fs::write(
            root.join(format!("realtime-zero-{label}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            root.join(format!("realtime-zero-{label}.stderr")),
            &output.stderr,
        )
        .unwrap();
        assert_eq!(
            output.exit_code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["status"], "PASS");
        if label == "priority" {
            assert_eq!(value["zero"], true);
            assert_eq!(value["priorityCeiling"], 0);
            assert_eq!(value["priorityErrno"], 1);
            assert_eq!(value["hardRaiseErrno"], 1);
            assert!(value["normalCpuMicros"].as_u64().unwrap() > 300000);
        } else {
            assert_eq!(value["zeroTime"], true);
            assert_eq!(value["hardSignal"], 9);
            assert!(value["cpuMicros"].as_u64().unwrap() > 0);
            assert_eq!(value["childReaped"], true);
        }
        manager
            .kill(&lease.execution_id, lease.generation)
            .await
            .unwrap();
        manager
            .remove(&lease.execution_id, lease.generation)
            .await
            .unwrap();
    }
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
    let bind = format!("{}:/realtime-probe:ro", probe.display());
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
        "0",
        "--cap-drop",
        "ALL",
        "--read-only",
        "--tmpfs",
        "/scratch:size=1m,mode=1777",
        "--health-cmd",
        "/realtime-probe --health",
        "--health-interval",
        "1",
        "--ulimit",
        "rtprio=5:5",
        "--ulimit",
        "rttime=100000:300000",
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
    verify(&run(&["exec", &id, "--", "/realtime-probe"]), false);
    verify_user(
        &run(&[
            "exec",
            "--user",
            "1001:2002",
            &id,
            "--",
            "/realtime-probe",
            "--nonroot",
        ]),
        false,
        1001,
        2002,
    );

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
        run(&[
            "exec",
            &id,
            "--",
            "/bin/cat",
            "/scratch/realtime-health-user"
        ]),
        b"0:0\n"
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
    verify(&run(&["exec", &id, "--", "/realtime-probe"]), false);
    let script = PathBuf::from(std::env::var_os("K_BOX_TEST_REALTIME_CLI_SCRIPT").unwrap());
    assert!(script.is_absolute() && script.is_file());
    let output = Command::new("/usr/bin/python3")
        .arg(&script)
        .arg(&binary)
        .arg(&id)
        .arg(root.join("realtime-cli-pty"))
        .env("K_HOME", root.join("home"))
        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
        .env("K_BOX_OCI_MIGRATION", "microvm")
        .output()
        .unwrap();
    fs::write(root.join("realtime-cli-pty-driver.stdout"), &output.stdout).unwrap();
    fs::write(root.join("realtime-cli-pty-driver.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pty: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("realtime-cli-pty/result.json")).unwrap())
            .unwrap();
    assert_eq!(pty["status"], "PASS");
    assert_eq!(pty["cases"].as_array().unwrap().len(), 2);
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned home,installed HVF assets and immutable Linux rlimit probe"]
fn native_realtime_limits_enforce_scheduler_budget_all_entries_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let binary=PathBuf::from(std::env::var_os("K_BOX_TEST_REALTIME_PROBE").unwrap());assert!(binary.is_absolute() && binary.is_file());
                let first=manager.create_and_start(configured(&image,"realtime-main",&binary),&OperationId::new("realtime-main-create").unwrap()).await.unwrap();let id=&first.execution_id;let mut lease=first.clone();let mut generations=Vec::new();
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();let config:serde_json::Value=serde_json::from_slice(&fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap()).unwrap();fs::write(root.join(format!("realtime-{label}-spec.json")),serde_json::to_vec_pretty(&config).unwrap()).unwrap();
                    probe(&manager,&root,id,lease.generation,label).await;
                    if wave==0 {
                        let mut command=exec(&["/realtime-probe","--nonroot"]);command.user=Some("1001:2002".into());let output=manager.execute(id,lease.generation,command).await.unwrap();assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));verify_user(&output.stdout,false,1001,2002);fs::write(root.join("realtime-nonroot.stdout"),&output.stdout).unwrap();

                        let mut command=exec(&["/bin/sh","-c","count=$(cat /scratch/replay-count 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay-count; /realtime-probe"]);command.request_id=Some("limit-replay".into());let output=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.exit_code,0);verify(&output.stdout,false);let replay=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.stdout,replay.stdout);assert_eq!(manager.execute(id,lease.generation,exec(&["/bin/cat","/scratch/replay-count"])).await.unwrap().stdout,b"1");command.cmd=vec!["/bin/echo".into(),"changed".into()];assert!(manager.execute(id,lease.generation,command).await.is_err());}
                    if wave<2 {
                        let old=lease.generation;
                        if wave==1 {host.sigkill();assert!(manager.inspect(id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline{tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(id,old,&OperationId::new(format!("realtime-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                let positive=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let control_binary=root.join("realtime-unlimited-probe");
                assert_eq!(fs::copy(&binary,&control_binary).unwrap(),fs::metadata(&binary).unwrap().len());
                let mut req=configured(&image,"realtime-unlimited-control",&control_binary);
                req.config.resource_limits.ulimits=vec!["rtprio=-1:-1".into(),"rttime=-1:-1".into(),"core=0:0".into()];
                req.config.cmd=vec!["/bin/sleep".into(),"2147483647".into()];
                let control=positive.create_and_start(req,&OperationId::new("realtime-unlimited-control").unwrap()).await.unwrap();
                let output=positive.execute(&control.execution_id,control.generation,exec(&["/realtime-probe","--unlimited"])).await.unwrap();
                assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));let unlimited:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(unlimited["status"],"PASS");assert_eq!(unlimited["unlimited"],true);assert_eq!(unlimited["priorityCeiling"],99);assert!(unlimited["unlimitedCpuMicros"].as_u64().unwrap()>300000);assert_eq!(unlimited["priorityErrno"],0);assert_eq!(unlimited["allowedRangeReused"],true);
                fs::write(root.join("realtime-unlimited-control.json"),&output.stdout).unwrap();
                let output=positive.execute(&control.execution_id,control.generation,exec(&["/bin/sh","-c","printf '%s:%s' $(ulimit -Sn) $(ulimit -Hn)"])).await.unwrap();assert_eq!(output.exit_code,0);assert_eq!(output.stdout,b"1024:1024");
                let output=manager.execute(id,lease.generation,exec(&["/realtime-probe"])).await.unwrap();assert_eq!(output.exit_code,0);verify(&output.stdout,false);fs::write(root.join("realtime-after-control.stdout"),&output.stdout).unwrap();
                positive.kill(&control.execution_id,control.generation).await.unwrap();positive.remove(&control.execution_id,control.generation).await.unwrap();
                manager.kill(id,lease.generation).await.unwrap();manager.remove(id,lease.generation).await.unwrap();
                zero_control(&positive,&root,&image,&binary).await;
                let cli_checked=cli(&root,&image,&binary);fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":id,"generations":generations,"priorityCeiling":5,"priorityEperm":1,"softSignal":24,"hardSignal":9,"blockingReset":true,"zeroRuntimeControl":true,"childAndThreadInheritance":true,"namespaceBypassRefusedOrBounded":true,"explicitNonroot":true,"zeroLimitControl":true,"unlimitedControl":true,"independentExecutionLimits":true,"hardRaiseEperm":true,"initExecChildStreamRealPty":true,"replayOneEffect":true,"changedRequestRefused":true,"defaultNofileRetained":[1024,1024],"defaultControlRoute":"oci_sdk","cliChecked":cli_checked,"stoppedRecovery":true,"processLimitScope":"per-process real-time priority ceiling and per-thread continuous real-time CPU microseconds, not reservation or wall time"})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
