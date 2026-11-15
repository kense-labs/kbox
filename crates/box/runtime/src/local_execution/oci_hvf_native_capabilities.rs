//! Actual Guest capability permissions with independent dropped/ALL controls.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;

fn probe_source(root: &Path, label: &str) -> PathBuf {
    let source = root.join(format!("capability-{label}-probe"));
    fs::copy(
        std::env::var_os("K_BOX_TEST_CAPABILITY_PROBE").unwrap(),
        &source,
    )
    .unwrap();
    let st = fs::symlink_metadata(&source).unwrap();
    assert!(st.is_file());
    assert_eq!(st.nlink(), 1);
    let snapshot = json!({"source":source,"uid":st.uid(),"gid":st.gid(),"mode":st.mode()&0o777,"device":st.dev(),"inode":st.ino(),"bytes":st.len(),"nlink":st.nlink(),"sha256":format!("{:x}",Sha256::digest(fs::read(&source).unwrap()))});
    fs::write(
        root.join(format!("capability-source-{label}.json")),
        serde_json::to_vec_pretty(&snapshot).unwrap(),
    )
    .unwrap();
    source
}

fn configured(
    root: &Path,
    image: &str,
    label: &str,
    mode: &str,
    uid: u32,
) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.resources.disk_mb = 128;
    req.config.user = Some(if uid == 0 {
        "0".into()
    } else {
        "1001:2002".into()
    });
    req.config.cap_add = vec!["CAP_MKNOD".into(), "SETFCAP".into(), "AUDIT_WRITE".into()];
    match mode {
        "added" => {}
        "added-admin" => req.config.cap_add.push("SYS_ADMIN".into()),
        "dropped" => {
            req.config.cap_drop = vec!["MKNOD".into(), "SETFCAP".into(), "AUDIT_WRITE".into()]
        }
        "all" => {
            req.config.cap_drop = vec!["ALL".into()];
            req.config.cap_add.push("SYS_ADMIN".into());
        }
        _ => panic!("unknown fixture mode"),
    }
    if uid != 0 && mode == "all" {
        assert_eq!(uid, 1001);
        assert_eq!(mode, "all");
        req.config.resource_limits.ulimits = vec!["nproc=16:16".into()];
    }
    let source = probe_source(root, label);
    req.config.volumes = vec![format!("{}:/capability-probe:ro", source.display())];
    let base = if uid == 0 { "" } else { "/ownership" };
    req.config.cmd=vec!["/bin/sh".into(),"-c".into(),format!("set -eu; /capability-probe {mode} {uid} > {base}/cap-init-ready.json; mv {base}/cap-init-ready.json {base}/cap-init.json; trap 'exit 0' TERM; while :; do /bin/sleep 1 & wait $!; done")];
    req
}

fn command(args: &[&str], uid: u32) -> ExecRequest {
    let mut req = exec(args);
    req.timeout_ns = 30_000_000_000;
    if uid != 0 {
        req.user = Some("1001:2002".into());
    }
    req
}

fn verify(bytes: &[u8], mode: &str, uid: u32, tty: bool) -> serde_json::Value {
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(bytes).trim()).unwrap();
    let allowed = matches!(mode, "added" | "added-admin");
    let mask = match mode {
        "added" => "a80405fb",
        "added-admin" => "a82405fb",
        "dropped" => "405fb",
        "all" => "0",
        _ => unreachable!(),
    };
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["uid"], uid);
    assert_eq!(value["tty"], tty);
    assert_eq!(value["bounding"], mask);
    let retained = if uid == 0 {
        "0"
    } else {
        match mode {
            "added" => "a8000000",
            "added-admin" => "a8200000",
            _ => "0",
        }
    };
    assert_eq!(
        value["effectivePermitted"],
        if uid == 0 { mask } else { retained }
    );
    assert_eq!(value["inheritableAmbient"], retained);
    assert_eq!(value["emptyInheritableAndAmbient"], retained == "0");
    assert_eq!(value["mknodAllowed"], allowed);
    assert_eq!(value["fileCapabilityAllowed"], allowed);
    assert_eq!(value["mknodErrno"], if allowed { 0 } else { 1 });
    assert_eq!(value["fileCapabilityErrno"], if allowed { 0 } else { 1 });
    assert_eq!(value["perUidHardLimitRetained"], uid != 0 && mode == "all");
    assert_eq!(value["sysAdminMountAllowed"], mode == "added-admin");
    assert_eq!(
        value["sysAdminMountErrno"],
        if mode == "added-admin" { 0 } else { 1 }
    );
    for key in [
        "noNewPrivileges",
        "plainWritesAllowed",
        "guestDeviceOpenDenied",
        "childInheritedAndReaped",
        "threadInheritedAndJoined",
        "ownedObjectsRemoved",
    ] {
        assert_eq!(value[key], true, "{key}");
    }
    value
}

async fn capture(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    mode: &str,
    uid: u32,
) -> serde_json::Value {
    let uid_text = uid.to_string();
    let output = manager
        .execute(
            id,
            generation,
            command(&["/capability-probe", mode, &uid_text], uid),
        )
        .await
        .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    verify(&output.stdout, mode, uid, false)
}

async fn paths(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
    mode: &str,
    uid: u32,
) {
    let initial = if uid == 0 {
        "/cap-init.json"
    } else {
        "/ownership/cap-init.json"
    };
    let test = format!("test -s {initial}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        if manager
            .execute(id, generation, command(&["/bin/sh", "-c", &test], uid))
            .await
            .unwrap()
            .exit_code
            == 0
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "init probe was not ready"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let init = manager
        .execute(id, generation, command(&["/bin/cat", initial], uid))
        .await
        .unwrap();
    assert_eq!(init.exit_code, 0);
    let mut observations = vec![
        verify(&init.stdout, mode, uid, false),
        capture(manager, id, generation, mode, uid).await,
    ];
    let script = format!("/capability-probe {mode} {uid}");
    let child = manager
        .execute(id, generation, command(&["/bin/sh", "-c", &script], uid))
        .await
        .unwrap();
    assert_eq!(
        child.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    observations.push(verify(&child.stdout, mode, uid, false));
    for tty in [false, true] {
        let uid_text = uid.to_string();
        let mut process = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/capability-probe".into(), mode.into(), uid_text.clone()],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: if uid == 0 {
                            None
                        } else {
                            Some("1001:2002".into())
                        },
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut req = command(&["/capability-probe", mode, &uid_text, "--stdin"], uid);
            req.stdin_streaming = true;
            manager.start_process(id, generation, req).await.unwrap()
        };
        if !tty {
            process.input().write_stdin(b"streamed\n").await.unwrap();
            process.input().close_stdin().await.unwrap();
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match process.next_event().await.unwrap().unwrap() {
                    ExecEvent::Chunk(chunk) => {
                        if chunk.stream == kbox_core::exec::StreamType::Stdout {
                            stdout.extend(chunk.data)
                        } else {
                            stderr.extend(chunk.data)
                        }
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(exit.exit_code, 0, "{}", String::from_utf8_lossy(&stderr));
        observations.push(verify(&stdout, mode, uid, tty));
    }
    assert_eq!(observations.len(), 5);
    fs::write(
        root.join(format!("capability-{label}-observations.json")),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

async fn remove(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    manager
        .kill_with_options(
            id,
            generation,
            kbox_core::KillExecutionOptions {
                signal: Some(libc::SIGKILL),
                timeout_secs: Some(1),
            },
        )
        .await
        .unwrap();
    manager.remove(id, generation).await.unwrap();
}

fn check_sources(root: &Path) {
    let mut records = Vec::new();
    for path in fs::read_dir(root).unwrap() {
        let path = path.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if !name.starts_with("capability-source-")
            || path.extension().is_none_or(|ext| ext != "json")
        {
            continue;
        }
        let before: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let source = Path::new(before["source"].as_str().unwrap());
        let st = fs::symlink_metadata(source).unwrap();
        assert!(st.is_file() && !st.file_type().is_symlink());
        for (key, value) in [
            ("uid", u64::from(st.uid())),
            ("gid", u64::from(st.gid())),
            ("mode", u64::from(st.mode() & 0o777)),
            ("device", st.dev()),
            ("inode", st.ino()),
            ("bytes", st.len()),
            ("nlink", st.nlink()),
        ] {
            assert_eq!(before[key], value, "{source:?}/{key}");
        }
        assert_eq!(
            before["sha256"],
            format!("{:x}", Sha256::digest(fs::read(source).unwrap()))
        );
        records.push(before);
    }
    fs::write(
        root.join("capability-sources-after.json"),
        serde_json::to_vec_pretty(&records).unwrap(),
    )
    .unwrap();
}

fn cli(root: &Path, image: &str) {
    let binary = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_CLI").unwrap());
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
    let source = probe_source(root, "cli");
    let bind = format!("{}:/capability-probe:ro", source.display());
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
        "--cap-add",
        "MKNOD",
        "--cap-add",
        "SETFCAP",
        "--cap-add",
        "AUDIT_WRITE",
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
    let inspect: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(inspect[0]["managed_execution"]["runtime_route"], "oci_sdk");
    assert_eq!(
        inspect[0]["managed_execution"]["request"]["config"]["cap_add"],
        json!(["MKNOD", "SETFCAP", "AUDIT_WRITE"])
    );
    verify(
        &run(&["exec", &id, "--", "/capability-probe", "added", "0"]),
        "added",
        0,
        false,
    );
    let script = std::env::var_os("K_BOX_TEST_CAPABILITY_CLI_SCRIPT").unwrap();
    let output = root.join("capability-cli-pty");
    let result = Command::new("/usr/bin/python3")
        .arg(script)
        .arg(&binary)
        .arg(root)
        .arg(&id)
        .arg(&output)
        .output()
        .unwrap();
    fs::write(
        root.join("capability-cli-pty-driver.stdout"),
        &result.stdout,
    )
    .unwrap();
    fs::write(
        root.join("capability-cli-pty-driver.stderr"),
        &result.stderr,
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let pty: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("result.json")).unwrap()).unwrap();
    assert_eq!(pty["status"], "PASS");
    assert_eq!(pty["cases"].as_array().unwrap().len(), 2);
    run(&["rm", "-f", &id]);
    let source = probe_source(root, "cli-nonroot");
    let bind = format!("{}:/capability-probe:ro", source.display());
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
        "--cap-add",
        "MKNOD",
        "--cap-add",
        "SETFCAP",
        "--cap-add",
        "AUDIT_WRITE",
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
    let observed = verify(
        &run(&[
            "exec",
            "--user",
            "1001:2002",
            &id,
            "--",
            "/capability-probe",
            "added",
            "1001",
        ]),
        "added",
        1001,
        false,
    );
    fs::write(
        root.join("capability-cli-nonroot.json"),
        serde_json::to_vec_pretty(&observed).unwrap(),
    )
    .unwrap();
    run(&["rm", "-f", &id]);
}

#[test]
#[ignore = "requires sentinel-owned root and fixed installed independent HVF assets"]
fn native_cap_add_enforces_guest_permissions_drop_controls_all_entries_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let store=ManagedExecutionStore::new(root.join("home/boxes.json"));
                for unknown in ["NET_ADMIN","NET_RAW","SYS_RESOURCE","ALL"] {
                    let mut req=request(&image,"capability-unknown");req.config.cap_add=vec![unknown.into()];
                    assert!(matches!(manager.create(req,&OperationId::new(format!("capability-unknown-{unknown}")).unwrap()).await,Err(ExecutionManagerError::InvalidRequest(_))));assert!(store.list().unwrap().is_empty());
                }
                let req=configured(&root,&image,"main","added",0);let op=OperationId::new("capability-main").unwrap();
                let first=manager.create(req.clone(),&op).await.unwrap();let replay=manager.create(req.clone(),&op).await.unwrap();
                assert_eq!((first.execution_id.clone(),first.generation),(replay.execution_id,replay.generation));
                let mut changed=req;changed.config.cap_drop=vec!["ALL".into()];assert!(manager.create(changed,&op).await.is_err());
                let first=manager.start(&first.execution_id,first.generation).await.unwrap();
                paths(&manager,&root,&first.execution_id,first.generation,"first","added",0).await;
                let mut effect=command(&["/bin/sh","-c","printf effect >> /capability-effects; cat /capability-effects"],0);effect.request_id=Some("capability-one-effect".into());
                let one=manager.execute(&first.execution_id,first.generation,effect.clone()).await.unwrap();let two=manager.execute(&first.execution_id,first.generation,effect.clone()).await.unwrap();assert_eq!(one.stdout,b"effect");assert_eq!(two.stdout,one.stdout);
                effect.cmd=vec!["/bin/sh".into(),"-c".into(),"printf illegal >> /capability-effects".into()];assert!(manager.execute(&first.execution_id,first.generation,effect).await.is_err());assert_eq!(manager.execute(&first.execution_id,first.generation,command(&["/bin/cat","/capability-effects"],0)).await.unwrap().stdout,b"effect");
                for (label,mode,uid) in [("specific-drop","dropped",0),("drop-all","all",0),("nonroot-nproc","all",1001),("nonroot-added","added",1001),("nonroot-dropped","dropped",1001),("nonroot-admin","added-admin",1001)] {
                    let control=manager.create_and_start(configured(&root,&image,label,mode,uid),&OperationId::new(format!("capability-{label}")).unwrap()).await.unwrap();
                    paths(&manager,&root,&control.execution_id,control.generation,label,mode,uid).await;remove(&manager,&control.execution_id,control.generation).await;
                }
                let mut ro=configured(&root,&image,"host-ro","added-admin",0);files::configure(&root,"capabilities",&mut ro);ro.config.cap_add.extend(["MKNOD".into(),"SETFCAP".into(),"AUDIT_WRITE".into()]);
                let ro=manager.create_and_start(ro,&OperationId::new("capability-host-ro").unwrap()).await.unwrap();
                capture(&manager,&ro.execution_id,ro.generation,"added-admin",0).await;files::check(&manager,&root,"capabilities",&ro.execution_id,ro.generation).await;
                let ro_next=manager.restart(&ro.execution_id,ro.generation,&OperationId::new("capability-host-ro-restart").unwrap()).await.unwrap();
                capture(&manager,&ro.execution_id,ro_next.generation,"added-admin",0).await;files::check(&manager,&root,"capabilities",&ro.execution_id,ro_next.generation).await;remove(&manager,&ro.execution_id,ro_next.generation).await;
                cli(&root,&image);
                let restarted=manager.restart(&first.execution_id,first.generation,&OperationId::new("capability-normal-restart").unwrap()).await.unwrap();assert_eq!(restarted.generation.get(),2);
                paths(&manager,&root,&first.execution_id,restarted.generation,"restart","added",0).await;
                assert!(manager.execute(&first.execution_id,first.generation,command(&["/bin/touch","/capability-stale"],0)).await.is_err());
                let record=store.get(&first.execution_id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();
                host.sigkill();assert!(manager.inspect(&first.execution_id).await.is_err());
                let report=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);
                while !report.exists()&&tokio::time::Instant::now()<deadline {tokio::time::sleep(Duration::from_millis(100)).await;}assert!(report.is_file());
                host=Host::start(&root,2);let recovered_manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(recovered_manager.inspect(&first.execution_id).await.unwrap().state,ExecutionState::Stopped);
                let recovered=recovered_manager.restart(&first.execution_id,restarted.generation,&OperationId::new("capability-owner-restart").unwrap()).await.unwrap();assert_eq!(recovered.generation.get(),3);
                paths(&recovered_manager,&root,&first.execution_id,recovered.generation,"recovery","added",0).await;
                assert_eq!(recovered_manager.execute(&first.execution_id,recovered.generation,command(&["/bin/sh","-c","test ! -e /capability-effects; test ! -e /capability-stale"],0)).await.unwrap().exit_code,0);
                remove(&recovered_manager,&first.execution_id,recovered.generation).await;
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","cliChecked":true,"generations":[1,2,3],"exactKernelCapabilitySets":true,"actualMknodAndFileCapabilityEffects":true,"specificAndAllDropsRefused":true,"nonrootNprocNoPrivilegeOrBudgetChange":true,"unknownBeforeClaims":true,"initExecChildThreadStreamRealPty":true,"replayOneEffect":true,"changedRequestRefused":true,"staleGenerationRefused":true,"stoppedRecovery":true,"ephemeralRootReset":true,"combinedSysAdminHostRoWaves":2,"cliRealPtyCases":2})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none() {host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());check_sources(&root);
            if let Err(error)=result {std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
