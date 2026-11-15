//! Exact shared-memory capacity and Guest syscall effects through installed assets.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;

fn configured(
    root: &Path,
    image: &str,
    label: &str,
    bytes: u64,
    uid: u32,
) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.resources.disk_mb = 128;
    req.config.user = Some(if uid == 0 {
        "0".into()
    } else {
        "1001:2002".into()
    });
    req.config.cap_drop = vec!["ALL".into()];
    req.policy.shm_size = (bytes != 0).then_some(bytes);
    let source = root.join(format!("shm-{label}-probe"));
    fs::copy(std::env::var_os("K_BOX_TEST_SHM_PROBE").unwrap(), &source).unwrap();
    let st = fs::symlink_metadata(&source).unwrap();
    assert!(st.is_file() && st.nlink() == 1);
    fs::write(root.join(format!("shm-source-{label}.json")), serde_json::to_vec_pretty(&json!({"source":source,"uid":st.uid(),"gid":st.gid(),"mode":st.mode()&0o777,"device":st.dev(),"inode":st.ino(),"bytes":st.len(),"nlink":st.nlink(),"sha256":format!("{:x}",Sha256::digest(fs::read(&source).unwrap()))})).unwrap()).unwrap();
    req.config.volumes = vec![format!("{}:/shm-probe:ro", source.display())];
    let inspect = if bytes == 0 || !(1024 * 1024..=2 * 1024 * 1024).contains(&bytes) {
        " --inspect"
    } else {
        ""
    };
    req.config.cmd = vec!["/bin/sh".into(),"-c".into(),format!("set -eu; /shm-probe {bytes} {uid}{inspect} > /dev/shm/shm-ready.tmp; mv /dev/shm/shm-ready.tmp /dev/shm/shm-ready.json; trap 'exit 0' TERM; while :; do /bin/sleep 1 & wait $!; done")];
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

fn verify(data: &[u8], bytes: u64, uid: u32, tty: bool, inspect: bool) -> serde_json::Value {
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(data).trim()).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["uid"], uid);
    assert_eq!(value["tty"], tty);
    assert_eq!(value["inspectOnly"], inspect);
    if bytes != 0 {
        assert_eq!(value["bytes"], bytes);
    } else {
        assert!(value["bytes"].as_u64().unwrap() >= 4096);
    }
    assert_eq!(value["blockBytes"], 4096);
    assert_eq!(value["mode"], 1023);
    assert_eq!(value["ownerUid"], 0);
    assert_eq!(value["ownerGid"], 0);
    for field in [
        "nosuidNodevNoexec",
        "emptyCapabilities",
        "noNewPrivileges",
        "ownedObjectsRemoved",
    ] {
        assert_eq!(value[field], true, "{field}");
    }
    for field in [
        "posixSharedMappingAndUnlinkedFdLifetime",
        "childReaped",
        "threadJoined",
    ] {
        assert_eq!(value[field], !inspect, "{field}");
    }
    assert_eq!(value["noexecErrno"], if inspect { 0 } else { libc::EACCES });
    let full = !inspect && (1..=2 * 1024 * 1024).contains(&bytes);
    assert_eq!(value["enospcErrno"], if full { libc::ENOSPC } else { 0 });
    assert_eq!(value["reuseSucceeded"], full);
    if full {
        let written = value["writtenBytes"].as_u64().unwrap();
        assert!(written > bytes / 2 && written <= bytes);
    }
    value
}

async fn capture(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    bytes: u64,
    uid: u32,
    inspect: bool,
) -> serde_json::Value {
    let size = bytes.to_string();
    let user = uid.to_string();
    let mut args = vec!["/shm-probe", &size, &user];
    if inspect {
        args.push("--inspect");
    }
    let output = manager
        .execute(id, generation, command(&args, uid))
        .await
        .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    verify(&output.stdout, bytes, uid, false, inspect)
}

async fn paths(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
    bytes: u64,
    uid: u32,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        if manager
            .execute(
                id,
                generation,
                command(&["/bin/sh", "-c", "test -s /dev/shm/shm-ready.json"], uid),
            )
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
        .execute(
            id,
            generation,
            command(&["/bin/cat", "/dev/shm/shm-ready.json"], uid),
        )
        .await
        .unwrap();
    assert_eq!(init.exit_code, 0);
    let mut values = vec![
        verify(&init.stdout, bytes, uid, false, false),
        capture(manager, id, generation, bytes, uid, false).await,
    ];
    let script = format!("/shm-probe {bytes} {uid}");
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
    values.push(verify(&child.stdout, bytes, uid, false, false));
    for tty in [false, true] {
        let size = bytes.to_string();
        let user = uid.to_string();
        let mut process = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/shm-probe".into(), size.clone(), user.clone()],
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
            let mut req = command(&["/shm-probe", &size, &user, "--stdin"], uid);
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
        values.push(verify(&stdout, bytes, uid, tty, false));
    }
    assert_eq!(values.len(), 5);
    fs::write(
        root.join(format!("shm-{label}-observations.json")),
        serde_json::to_vec_pretty(&values).unwrap(),
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

fn cli(root: &Path, image: &str) {
    let binary = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_CLI").unwrap());
    let run = |args: &[&str]| {
        let p = Command::new(&binary)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", "microvm")
            .args(args)
            .output()
            .unwrap();
        assert!(
            p.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&p.stderr)
        );
        p.stdout
    };
    let req = configured(root, image, "cli", 1024 * 1024, 0);
    let bind = &req.config.volumes[0];
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
        "--shm-size",
        "1m",
        "-v",
        bind,
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
        inspect[0]["managed_execution"]["request"]["policy"]["shm_size"],
        1024 * 1024
    );
    verify(
        &run(&["exec", &id, "--", "/shm-probe", "1048576", "0"]),
        1024 * 1024,
        0,
        false,
        false,
    );
    let output = root.join("shm-cli-pty");
    let p = Command::new("/usr/bin/python3")
        .arg(std::env::var_os("K_BOX_TEST_SHM_CLI_SCRIPT").unwrap())
        .arg(&binary)
        .arg(root)
        .arg(&id)
        .arg(&output)
        .output()
        .unwrap();
    fs::write(root.join("shm-cli-pty-driver.stdout"), &p.stdout).unwrap();
    fs::write(root.join("shm-cli-pty-driver.stderr"), &p.stderr).unwrap();
    assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["cases"].as_array().unwrap().len(), 2);
    run(&["rm", "-f", &id]);
}

#[test]
#[ignore = "requires sentinel-owned root and fixed installed independent HVF assets"]
fn native_shm_size_enforces_capacity_posix_effects_all_entries_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let store=ManagedExecutionStore::new(root.join("home/boxes.json"));
                for bytes in [0,4095,4097,512*1024*1024+4096,u64::MAX] {let mut req=request(&image,"shm-invalid");req.policy.shm_size=Some(bytes);assert!(matches!(manager.create(req,&OperationId::new(format!("shm-invalid-{bytes}")).unwrap()).await,Err(ExecutionManagerError::InvalidRequest(_))));assert!(store.list().unwrap().is_empty());}
                let mut req=configured(&root,&image,"main",1024*1024,1001);req.config.read_only=true;req.config.tmpfs=vec!["/scratch:size=64k,mode=1777,nosuid,nodev,noexec".into()];
                let op=OperationId::new("shm-main").unwrap();let first=manager.create(req.clone(),&op).await.unwrap();let replay=manager.create(req.clone(),&op).await.unwrap();assert_eq!((first.execution_id.clone(),first.generation),(replay.execution_id,replay.generation));
                req.policy.shm_size=Some(2*1024*1024);assert!(manager.create(req,&op).await.is_err());let first=manager.start(&first.execution_id,first.generation).await.unwrap();
                paths(&manager,&root,&first.execution_id,first.generation,"first",1024*1024,1001).await;
                let sticky=manager.execute(&first.execution_id,first.generation,command(&["/bin/sh","-c","printf owner > /dev/shm/shm-sticky; chmod 600 /dev/shm/shm-sticky; test $(stat -c '%u:%a' /dev/shm/shm-sticky) = 1001:600; test $(stat -c %d /dev/shm) != $(stat -c %d /scratch)"],1001)).await.unwrap();assert_eq!(sticky.exit_code,0);
                let mut other=exec(&["/bin/sh","-c","set -eu; test $(id -u) = 1002; if rm /dev/shm/shm-sticky 2>/tmp/sticky-error; then exit 31; fi; grep -q 'Operation not permitted' /tmp/sticky-error; test -e /dev/shm/shm-sticky; printf other > /dev/shm/shm-other; rm /dev/shm/shm-other"]);other.user=Some("1002:2002".into());
                // Root is readonly; keep the diagnostic inside the writable shared-memory mount.
                other.cmd[2]=other.cmd[2].replace("/tmp/sticky-error","/dev/shm/sticky-error");let other=manager.execute(&first.execution_id,first.generation,other).await.unwrap();assert_eq!(other.exit_code,0,"{}",String::from_utf8_lossy(&other.stderr));
                let mut effect=command(&["/bin/sh","-c","printf effect >> /dev/shm/shm-effects; cat /dev/shm/shm-effects"],1001);effect.request_id=Some("shm-one-effect".into());let one=manager.execute(&first.execution_id,first.generation,effect.clone()).await.unwrap();let two=manager.execute(&first.execution_id,first.generation,effect.clone()).await.unwrap();assert_eq!(one.stdout,b"effect");assert_eq!(two.stdout,one.stdout);effect.cmd=vec!["/bin/sh".into(),"-c".into(),"printf bad >> /dev/shm/shm-effects".into()];assert!(manager.execute(&first.execution_id,first.generation,effect).await.is_err());
                for direct in [false,true] {let label=if direct {"direct"}else{"independent"};let mut req=configured(&root,&image,label,2*1024*1024,0);if direct {req.policy.shm_size=None;req.config.tmpfs=vec!["/dev/shm:size=2m".into()];}let control=manager.create_and_start(req,&OperationId::new(format!("shm-{label}")).unwrap()).await.unwrap();paths(&manager,&root,&control.execution_id,control.generation,label,2*1024*1024,0).await;let absent=manager.execute(&control.execution_id,control.generation,command(&["/bin/sh","-c","test ! -e /dev/shm/shm-effects; test ! -e /dev/shm/shm-sticky"],0)).await.unwrap();assert_eq!(absent.exit_code,0);remove(&manager,&control.execution_id,control.generation).await;}
                let mut controls=Vec::new();for (label,bytes) in [("minimum",4096),("maximum",512*1024*1024),("default",0)] {let control=manager.create_and_start(configured(&root,&image,label,bytes,0),&OperationId::new(format!("shm-{label}")).unwrap()).await.unwrap();controls.push(capture(&manager,&control.execution_id,control.generation,bytes,0,true).await);remove(&manager,&control.execution_id,control.generation).await;}fs::write(root.join("shm-boundary-default-controls.json"),serde_json::to_vec_pretty(&controls).unwrap()).unwrap();
                cli(&root,&image);let restarted=manager.restart(&first.execution_id,first.generation,&OperationId::new("shm-restart").unwrap()).await.unwrap();assert_eq!(restarted.generation.get(),2);paths(&manager,&root,&first.execution_id,restarted.generation,"restart",1024*1024,1001).await;
                assert_eq!(manager.execute(&first.execution_id,restarted.generation,command(&["/bin/sh","-c","test ! -e /dev/shm/shm-effects; test ! -e /dev/shm/shm-sticky"],1001)).await.unwrap().exit_code,0);assert!(manager.execute(&first.execution_id,first.generation,command(&["/bin/touch","/dev/shm/shm-stale"],1001)).await.is_err());
                let metadata=store.get(&first.execution_id).unwrap().unwrap().managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();host.sigkill();assert!(manager.inspect(&first.execution_id).await.is_err());let report=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !report.exists()&&tokio::time::Instant::now()<deadline {tokio::time::sleep(Duration::from_millis(100)).await;}assert!(report.is_file());
                host=Host::start(&root,2);let recovered=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(recovered.inspect(&first.execution_id).await.unwrap().state,ExecutionState::Stopped);let next=recovered.restart(&first.execution_id,restarted.generation,&OperationId::new("shm-owner-restart").unwrap()).await.unwrap();assert_eq!(next.generation.get(),3);paths(&recovered,&root,&first.execution_id,next.generation,"recovery",1024*1024,1001).await;assert_eq!(recovered.execute(&first.execution_id,next.generation,command(&["/bin/sh","-c","test ! -e /dev/shm/shm-effects; test ! -e /dev/shm/shm-stale"],1001)).await.unwrap().exit_code,0);remove(&recovered,&first.execution_id,next.generation).await;
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","cliChecked":true,"generations":[1,2,3],"actualPosixSharedMappingsAndLifetime":true,"actualEnospcAndReuse":true,"stickyOwnershipDeniedOtherUid":true,"noexecDenied":true,"independentInstances":true,"boundariesAndDefaultChecked":true,"rootReadonlyAndSeparateTmpfsPreserved":true,"replayOneEffect":true,"changedRequestRefused":true,"staleGenerationRefused":true,"stoppedRecovery":true,"sharedMemoryResetEachGeneration":true,"cliRealPtyCases":2})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());
            if let Err(error)=result {std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
