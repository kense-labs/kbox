//! Actual static process limits across init, captured/streaming exec, PTY and restart.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};

fn configured(image: &str, label: &str, probe: &Path) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.user = Some("0".into());
    req.config.cap_drop = vec!["ALL".into()];
    req.config.read_only = true;
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777".into()];
    req.config.resource_limits.ulimits = vec![
        "nofile=64:128".into(),
        "fsize=4096:4096".into(),
        "core=0:0".into(),
    ];
    req.config.volumes = vec![format!("{}:/rlimit-probe:ro", probe.display())];
    req.config.cmd = vec![
        "/bin/sh".into(),
        "-c".into(),
        "set -eu; /rlimit-probe > /scratch/init.json; exec /bin/sleep 2147483647".into(),
    ];
    req
}

fn verify(bytes: &[u8], tty: bool) -> serde_json::Value {
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().find(|line| line.starts_with('{')).unwrap();
    let value: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["nofile"], json!([64, 128]));
    assert_eq!(value["fsize"], json!([4096, 4096]));
    assert_eq!(value["core"], json!([0, 0]));
    assert_eq!(value["fileBytes"], 4096);
    assert_eq!(value["fileSizeErrno"], 27);
    assert_eq!(value["fdErrno"], 24);
    assert_eq!(value["raiseHardErrno"], 1);
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
    let mut observations = Vec::new();
    for (name, args) in [
        ("init", vec!["/bin/cat", "/scratch/init.json"]),
        ("exec", vec!["/rlimit-probe"]),
        ("child", vec!["/bin/sh", "-c", "/rlimit-probe"]),
    ] {
        let output = manager.execute(id, generation, exec(&args)).await.unwrap();
        fs::write(
            root.join(format!("limits-{label}-{name}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            root.join(format!("limits-{label}-{name}.stderr")),
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
                        cmd: vec!["/rlimit-probe".into()],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: Some("0".into()),
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut command = exec(&["/rlimit-probe", "--stdin"]);
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
                        fs::write(root.join(format!("limits-{label}-{name}.stdout")), &output)
                            .unwrap();
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        fs::write(root.join(format!("limits-{label}-{name}.stderr")), &errors).unwrap();
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
        root.join(format!("limits-{label}-observations.json")),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
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
    let bind = format!("{}:/rlimit-probe:ro", probe.display());
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
        "--ulimit",
        "nofile=64:128",
        "--ulimit",
        "fsize=4096:4096",
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
    verify(&run(&["exec", &id, "--", "/rlimit-probe"]), false);
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
    verify(&run(&["exec", &id, "--", "/rlimit-probe"]), false);
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned home,installed HVF assets and immutable Linux rlimit probe"]
fn native_ulimits_enforce_kernel_effects_all_process_paths_replay_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let binary=PathBuf::from(std::env::var_os("K_BOX_TEST_LIMIT_PROBE").unwrap());assert!(binary.is_absolute() && binary.is_file());
                let first=manager.create_and_start(configured(&image,"ulimit-main",&binary),&OperationId::new("ulimit-main-create").unwrap()).await.unwrap();let id=&first.execution_id;let mut lease=first.clone();let mut generations=Vec::new();
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();let config:serde_json::Value=serde_json::from_slice(&fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap()).unwrap();fs::write(root.join(format!("limits-{label}-spec.json")),serde_json::to_vec_pretty(&config).unwrap()).unwrap();
                    probe(&manager,&root,id,lease.generation,label).await;
                    if wave==0 {let mut command=exec(&["/bin/sh","-c","count=$(cat /scratch/replay-count 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay-count; /rlimit-probe"]);command.request_id=Some("limit-replay".into());let output=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.exit_code,0);verify(&output.stdout,false);let replay=manager.execute(id,lease.generation,command.clone()).await.unwrap();assert_eq!(output.stdout,replay.stdout);assert_eq!(manager.execute(id,lease.generation,exec(&["/bin/cat","/scratch/replay-count"])).await.unwrap().stdout,b"1");command.cmd=vec!["/bin/echo".into(),"changed".into()];assert!(manager.execute(id,lease.generation,command).await.is_err());}
                    if wave<2 {
                        let old=lease.generation;
                        if wave==1 {host.sigkill();assert!(manager.inspect(id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline{tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(id,old,&OperationId::new(format!("limits-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                manager.kill(id,lease.generation).await.unwrap();manager.remove(id,lease.generation).await.unwrap();
                let positive=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let mut req=request(&image,"ulimit-default-control");req.config.user=Some("0".into());req.config.cap_drop=vec!["ALL".into()];let control=positive.create_and_start(req,&OperationId::new("ulimit-default-control").unwrap()).await.unwrap();let output=positive.execute(&control.execution_id,control.generation,exec(&["/bin/sh","-c","printf '%s:%s' $(ulimit -Sn) $(ulimit -Hn)"])).await.unwrap();assert_eq!(output.exit_code,0);assert_eq!(output.stdout,b"1024:1024");fs::write(root.join("limits-default-control.stdout"),&output.stdout).unwrap();positive.kill(&control.execution_id,control.generation).await.unwrap();positive.remove(&control.execution_id,control.generation).await.unwrap();
                let cli_checked=cli(&root,&image,&binary);fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":id,"generations":generations,"kernelNofileEmfile":true,"kernelFsizeEfbig":true,"coreReadbackZero":true,"hardRaiseEperm":true,"initExecChildStreamRealPty":true,"replayOneEffect":true,"changedRequestRefused":true,"defaultNofileRetained":[1024,1024],"defaultControlRoute":"oci_sdk","cliChecked":cli_checked,"stoppedRecovery":true,"processLimitScope":"trusted management filesystem helper is not a workload process"})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
