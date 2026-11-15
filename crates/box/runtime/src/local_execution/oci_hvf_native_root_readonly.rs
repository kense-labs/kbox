//! Guest root read_only intent through the existing OCI executor and all access paths.
use super::*;
use kbox_core::{
    exec::{ExecEvent, StreamType},
    pty::PtyRequest,
    volume::VolumeConfig,
    ExecutionGeneration, ExecutionId, FileOp, FilesystemOp, FilesystemRequest,
};
use std::os::unix::fs::MetadataExt;

const ATTACKS: &str = r#"set -eu
test "$(id -u)" = 0
test "$(stat -c '%u:%a' /etc/passwd)" = 0:644
before=$(sha256sum /etc/passwd)
index=0
for command in ': > /root-denied' 'mkdir /root-dir-denied' 'printf mutation >> /etc/passwd' ': > /etc/passwd' 'rm -f /etc/passwd' 'mv /etc/passwd /root-moved' 'ln -s /etc/passwd /root-symlink' 'ln /etc/passwd /root-hardlink' 'sh -c "printf child > /root-child"'; do
  index=$((index + 1))
  if /bin/sh -c "$command" 2>/scratch/root-error; then echo "forbidden:$index"; exit 41; fi
  grep -q 'Read-only file system' /scratch/root-error
done
rm -f /scratch/root-link
ln -s / /scratch/root-link
if (printf symlink > /scratch/root-link/root-denied) 2>/scratch/root-error; then exit 42; fi
grep -q 'Read-only file system' /scratch/root-error
test "$before" = "$(sha256sum /etc/passwd)"
test ! -e /root-denied && test ! -e /root-child && test ! -e /root-moved
printf temporary > /scratch/value
printf 'ROOT_READONLY_EFFECTS_PASS:10\n'
"#;

fn configured(image: &str, label: &str, read_only: bool) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.user = Some("0".into());
    req.config.cap_drop = vec!["ALL".into()];
    req.config.read_only = read_only;
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777,rw,nosuid,nodev".into()];
    if read_only {
        req.config.cmd = vec!["/bin/sh".into(), "-c".into(), "if (printf forbidden > /init-root-denied) 2>/scratch/init-error; then exit 43; fi; grep -q 'Read-only file system' /scratch/init-error && printf INIT_ROOT_RO_PASS > /scratch/init && exec /bin/sleep 2147483647".into()];
    }
    req
}

async fn probe(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
) {
    let initial = manager
        .execute(id, generation, exec(&["/bin/cat", "/scratch/init"]))
        .await
        .unwrap();
    assert_eq!(
        (initial.exit_code, initial.stdout),
        (0, b"INIT_ROOT_RO_PASS".to_vec())
    );
    let result = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", ATTACKS]))
        .await
        .unwrap();
    fs::write(
        root.join(format!("root-{label}-exec.stdout")),
        &result.stdout,
    )
    .unwrap();
    fs::write(
        root.join(format!("root-{label}-exec.stderr")),
        &result.stderr,
    )
    .unwrap();
    assert_eq!(
        result.exit_code,
        0,
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"ROOT_READONLY_EFFECTS_PASS:10\n");
    for tty in [false, true] {
        let script = format!(
            "{}\n{}",
            if tty {
                "test -t 1 || exit 44"
            } else {
                "read line; test \"$line\" = streamed || exit 45"
            },
            ATTACKS
        );
        let mut process = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec!["/bin/sh".into(), "-c".into(), script],
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
            let mut command = exec(&["/bin/sh", "-c", &script]);
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
        let mut bytes = Vec::new();
        let exit = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match process.next_event().await.unwrap().unwrap() {
                    ExecEvent::Chunk(chunk) => {
                        assert_eq!(chunk.stream, StreamType::Stdout);
                        bytes.extend(chunk.data);
                        fs::write(
                            root.join(format!(
                                "root-{label}-{}.stdout",
                                if tty { "pty" } else { "stream" }
                            )),
                            &bytes,
                        )
                        .unwrap();
                    }
                    ExecEvent::Exit(exit) => break exit,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await;
        if exit.is_err() {
            let diagnostic = tokio::time::timeout(
                Duration::from_secs(10),
                manager.execute(
                    id,
                    generation,
                    exec(&[
                        "/bin/sh",
                        "-c",
                        "cat /scratch/root-error; cat /proc/self/mountinfo",
                    ]),
                ),
            )
            .await;
            fs::write(
                root.join(format!("root-{label}-timeout-diagnostic.txt")),
                format!("{diagnostic:?}\n"),
            )
            .unwrap();
        }
        let exit = exit.unwrap();
        fs::write(
            root.join(format!(
                "root-{label}-{}.stdout",
                if tty { "pty" } else { "stream" }
            )),
            &bytes,
        )
        .unwrap();
        assert_eq!(exit.exit_code, 0, "{}", String::from_utf8_lossy(&bytes));
        assert!(String::from_utf8_lossy(&bytes).contains("ROOT_READONLY_EFFECTS_PASS:10"));
    }
    let read = manager
        .transfer_file(
            id,
            generation,
            behavior::file(FileOp::Download, "/etc/passwd", None),
        )
        .await
        .unwrap();
    assert!(read.success);
    let blocked = manager
        .transfer_file(
            id,
            generation,
            behavior::file(
                FileOp::Upload,
                "/file-api-denied",
                Some(b"forbidden".to_vec()),
            ),
        )
        .await;
    fs::write(
        root.join(format!("root-{label}-file-refusal.txt")),
        format!("{blocked:?}\n"),
    )
    .unwrap();
    assert!(match blocked {
        Err(_) => true,
        Ok(value) => !value.success,
    });
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                behavior::file(
                    FileOp::Upload,
                    "/scratch/file-api",
                    Some(b"allowed".to_vec())
                )
            )
            .await
            .unwrap()
            .success
    );
    let mut refusals = Vec::new();
    for (index, op, path, destination) in [
        (0, FilesystemOp::MakeDir, "/fs-api-denied", None),
        (1, FilesystemOp::Move, "/etc/passwd", Some("/fs-api-moved")),
        (2, FilesystemOp::Remove, "/etc/passwd", None),
    ] {
        let output = manager
            .filesystem(
                id,
                generation,
                FilesystemRequest {
                    op,
                    path: path.into(),
                    destination: destination.map(str::to_owned),
                    depth: 0,
                    user: Some("0".into()),
                    request_id: Some(format!("root-{label}-fs-{index}")),
                },
            )
            .await;
        refusals.push(format!("{output:?}"));
        assert!(output.is_err());
    }
    fs::write(
        root.join(format!("root-{label}-fs-refusals.json")),
        serde_json::to_vec_pretty(&refusals).unwrap(),
    )
    .unwrap();
    manager
        .filesystem(
            id,
            generation,
            FilesystemRequest {
                op: FilesystemOp::MakeDir,
                path: "/scratch/fs-allowed".into(),
                destination: None,
                depth: 0,
                user: Some("0".into()),
                request_id: Some(format!("root-{label}-fs-allowed")),
            },
        )
        .await
        .unwrap();
    let post=manager.execute(id,generation,exec(&["/bin/sh","-c","set -eu; test ! -e /file-api-denied && test ! -e /fs-api-denied && test ! -e /fs-api-moved && test ! -e /init-root-denied && test -s /etc/passwd && test $(cat /scratch/file-api) = allowed && test -d /scratch/fs-allowed; cat /proc/self/mountinfo"])).await.unwrap();
    assert_eq!(post.exit_code, 0);
    let text = String::from_utf8(post.stdout).unwrap();
    let line = text
        .lines()
        .find(|line| line.split_whitespace().nth(4) == Some("/"))
        .unwrap();
    assert!(line
        .split_whitespace()
        .nth(5)
        .unwrap()
        .split(',')
        .any(|flag| flag == "ro"));
    fs::write(root.join(format!("root-{label}-mountinfo.stdout")), text).unwrap();
}

fn cli(root: &Path, image: &str) -> bool {
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
        image,
        "--",
        "/bin/sleep",
        "2147483647",
    ]))
    .unwrap()
    .trim()
    .to_owned();
    let before: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(before[0]["read_only"], true);
    assert_eq!(before[0]["managed_execution"]["runtime_route"], "oci_sdk");
    assert_eq!(
        run(&["exec", &id, "--", "/bin/sh", "-c", ATTACKS]),
        b"ROOT_READONLY_EFFECTS_PASS:10\n"
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
    run(&[
        "exec",
        &id,
        "--",
        "/bin/sh",
        "-c",
        "test ! -e /scratch/value",
    ]);
    assert_eq!(
        run(&["exec", &id, "--", "/bin/sh", "-c", ATTACKS]),
        b"ROOT_READONLY_EFFECTS_PASS:10\n"
    );
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned home and fixed installed independent HVF assets"]
fn native_root_readonly_intent_enforces_all_access_paths_and_preserves_inner_mounts() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let volumes=crate::VolumeStore::new(root.join("home/volumes.json"),root.join("home/volumes"));let named=volumes.create(VolumeConfig::new("rootro-retained","")).unwrap();let source=PathBuf::from(&named.mount_point);fs::write(source.join("canary"),b"named-canary").unwrap();let caller=root.join("caller");fs::create_dir(&caller).unwrap();fs::write(caller.join("canary"),b"caller-canary").unwrap();let owner=fs::metadata(&caller).unwrap();
            let result=std::panic::AssertUnwindSafe(async {
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let mut req=configured(&image,"rootro-main",true);req.config.volumes=vec![format!("{}:/named",source.display()),format!("{}:/caller",caller.display())];req.policy.volume_names=vec![named.name.clone()];
                let first=manager.create_and_start(req,&OperationId::new("rootro-main-create").unwrap()).await.unwrap();let id=&first.execution_id;
                let mut lease=first.clone();let mut generations=Vec::new();let store=ManagedExecutionStore::new(root.join("home/boxes.json"));let original=store.get(id).unwrap().unwrap();let anonymous=original.anonymous_volumes.clone();assert_eq!(anonymous.len(),3);
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=store.get(id).unwrap().unwrap();assert!(record.read_only);let metadata=record.managed_execution.as_ref().unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);assert_eq!(record.anonymous_volumes,anonymous);
                    let binding=metadata.oci_runtime.as_ref().unwrap();let config:serde_json::Value=serde_json::from_slice(&fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap()).unwrap();assert_eq!(config["root"]["readonly"],true);fs::write(root.join(format!("root-{label}-spec.json")),serde_json::to_vec_pretty(&config).unwrap()).unwrap();
                    if wave>0 {let clear=manager.execute(id,lease.generation,exec(&["/bin/sh","-c","test ! -e /scratch/value && test $(cat /caller/value) = caller-retained && test $(cat /named/value) = named-retained"])).await.unwrap();assert_eq!(clear.exit_code,0,"{}",String::from_utf8_lossy(&clear.stderr));let mut data=exec(&["/bin/cat","/ownership/root-ro-marker"]);data.user=Some("1001:2002".into());assert_eq!(manager.execute(id,lease.generation,data).await.unwrap().stdout,b"anonymous-retained");}
                    probe(&manager,&root,id,lease.generation,label).await;
                    let mut write=exec(&["/bin/sh","-c","set -eu; printf caller-retained > /caller/value; printf named-retained > /named/value"]);write.user=Some(format!("{}:{}",owner.uid(),owner.gid()));let output=manager.execute(id,lease.generation,write).await.unwrap();assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));
                    let mut data=exec(&["/bin/sh","-c","printf anonymous-retained > /ownership/root-ro-marker"]);data.user=Some("1001:2002".into());let output=manager.execute(id,lease.generation,data).await.unwrap();fs::write(root.join(format!("root-{label}-anonymous-write.stderr")),&output.stderr).unwrap();assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));
                    if wave<2 {
                        let old=lease.generation;
                        if wave==1 {
                            host.sigkill();assert!(manager.inspect(id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline {tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);let reconstructed=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(reconstructed.inspect(id).await.unwrap().state,ExecutionState::Stopped);manager=reconstructed;
                        }
                        lease=manager.restart(id,old,&OperationId::new(format!("rootro-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(id,old,exec(&["/bin/touch","/caller/stale"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));assert!(!caller.join("stale").exists());
                    }
                }
                manager.kill(id,lease.generation).await.unwrap();manager.remove(id,lease.generation).await.unwrap();
                let positive_manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let positive=positive_manager.create_and_start(configured(&image,"rootro-writable-control",false),&OperationId::new("rootro-writable-control").unwrap()).await.unwrap();assert_eq!(store.get(&positive.execution_id).unwrap().unwrap().managed_execution.unwrap().runtime_route,ManagedRuntimeRoute::OciSdk);let output=positive_manager.execute(&positive.execution_id,positive.generation,exec(&["/bin/sh","-c","set -eu; printf writable > /root-positive; printf appended >> /etc/passwd; test $(cat /root-positive) = writable; rm /root-positive; printf ROOT_RW_CONTROL_PASS"])).await.unwrap();fs::write(root.join("root-writable-control.stdout"),&output.stdout).unwrap();fs::write(root.join("root-writable-control.stderr"),&output.stderr).unwrap();assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));assert_eq!(output.stdout,b"ROOT_RW_CONTROL_PASS");positive_manager.kill(&positive.execution_id,positive.generation).await.unwrap();positive_manager.remove(&positive.execution_id,positive.generation).await.unwrap();
                assert_eq!(fs::read(caller.join("value")).unwrap(),b"caller-retained");assert_eq!(fs::read(source.join("value")).unwrap(),b"named-retained");
                let cli_checked=cli(&root,&image);
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":id,"generations":generations,"actualRootErofs":true,"attacksPerExec":10,"initExecStreamPty":true,"fileAndFilesystemRefused":true,"writableInnerTmpfsCallerNamedAnonymous":true,"emptyTmpfsAfterRestart":true,"retainedInnerData":true,"writableRootControl":true,"writableRootRoute":"oci_sdk","sameAnonymousAllocations":anonymous,"stoppedRecovery":true,"cliChecked":cli_checked,"rootControlScope":"Guest mount,not Host raw disk RO or SYS_ADMIN revocation"})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;assert_eq!(fs::read(caller.join("canary")).unwrap(),b"caller-canary");assert_eq!(fs::read(source.join("canary")).unwrap(),b"named-canary");let retained=volumes.get(&named.name).unwrap().unwrap();assert!(retained.in_use_by.is_empty());fs::write(root.join("rootro-retained-storage.json"),serde_json::to_vec_pretty(&retained).unwrap()).unwrap();volumes.remove(&named.name,false).unwrap();assert!(volumes.list().unwrap().is_empty());drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires sentinel-owned home and installed old Host without rootfs-image v2"]
fn native_old_host_refuses_readonly_root_before_preparation_without_fallback() {
    use koci_sdk::{ListRequest, RuntimeClient, RuntimeTransportClient};
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;let image=std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let reserved=manager.create(configured(&image,"old-host-v1-reservation",false),&OperationId::new("old-host-v1-reservation").unwrap()).await.unwrap();
                let store=ManagedExecutionStore::new(root.join("home/boxes.json"));let before=serde_json::to_value(store.list().unwrap()).unwrap();
                let error=manager.create(configured(&image,"old-host-rootro",true),&OperationId::new("old-host-rootro").unwrap()).await.unwrap_err();fs::write(root.join("old-host-refusal.txt"),format!("{error:?}\n")).unwrap();assert!(matches!(error,kbox_core::ExecutionManagerError::InvalidRequest(_)));
                assert_eq!(serde_json::to_value(store.list().unwrap()).unwrap(),before);let record=store.get(&reserved.execution_id).unwrap().unwrap();let metadata=record.managed_execution.as_ref().unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);assert_eq!(metadata.generation,reserved.generation);assert!(!metadata.request.config.read_only && !record.read_only);assert!(metadata.oci_runtime.is_none());
                let volumes=crate::VolumeStore::new(root.join("home/volumes.json"),root.join("home/volumes"));assert!(volumes.list().unwrap().is_empty());let shares=root.join("service/runtime/shares");assert!(!shares.exists() || fs::read_dir(shares).unwrap().next().is_none());
                let stream=tokio::net::UnixStream::connect(root.join("service/runtime.sock")).await.unwrap();let client=RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());assert!(client.list(ListRequest::default()).await.unwrap().is_empty());assert!(!fs::read_dir(root.join("home")).unwrap().any(|entry|entry.unwrap().file_name().to_string_lossy().starts_with(".oci-workload-")));
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","oldHostRefused":true,"refusedDuringCreate":true,"existingV1ReservationUnchanged":true,"noPreparation":true,"noAllocations":true,"noWorkload":true,"noFallback":true,"route":"oci_sdk","generation":reserved.generation})).unwrap()).unwrap();
            }).catch_unwind().await;cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
