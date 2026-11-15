//! Actual RAM-backed file grants; only synthetic material and digest output.
use super::*;
use kbox_core::{exec::ExecEvent, pty::PtyRequest, ExecutionGeneration, ExecutionId, FileOp};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const A: &[u8] = b"synthetic-private-ram-file-alpha";
const B: &[u8] = b"synthetic-private-ram-file-beta";

pub(super) fn store(root: &Path) -> crate::BoxTransientSecretStore {
    let path = std::env::var_os("K_BOX_TEST_SECRET_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("secrets"));
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let store = crate::BoxTransientSecretStore::new(path.canonicalize().unwrap());
    store.require_ready_sync().unwrap();
    store
}

async fn configured(
    root: &Path,
    store: &crate::BoxTransientSecretStore,
    image: &str,
    label: &str,
    identity: &str,
    bytes: &[u8],
    privileged: bool,
) -> (CreateExecutionRequest, PathBuf) {
    let projection = store
        .materialize_environment(
            identity,
            vec![(
                "SYNTHETIC_FILE".into(),
                crate::BoxSecretMaterial::new(bytes.to_vec()).unwrap(),
            )],
        )
        .await
        .unwrap();
    let source = crate::vm::VmManager::parse_volume_spec(&projection.volumes[0])
        .unwrap()
        .host_path;
    let mut req = request(image, label);
    req.policy.managed_secret_root = Some(store.root().into());
    req.config.volumes = vec![format!("{}:/exact-ro:ro", source.display())];
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777".into()];
    req.config.cmd = vec![
        "/bin/sh".into(),
        "-c".into(),
        "sha256sum /exact-ro > /scratch/init.hash; exec /bin/sleep 2147483647".into(),
    ];
    if privileged {
        req.config.user = Some("0".into());
        req.config.cap_add = vec!["SYS_ADMIN".into()];
        let rw = root.join(format!("{label}-rw"));
        fs::write(&rw, b"initial-rw").unwrap();
        fs::set_permissions(&rw, fs::Permissions::from_mode(0o666)).unwrap();
        req.config
            .volumes
            .push(format!("{}:/exact-rw:rw", rw.display()));
    } else {
        req.config.user = Some(fs::metadata(&source).unwrap().uid().to_string());
        req.config.cap_drop = vec!["ALL".into()];
    }
    (req, source)
}

pub(super) fn digest(output: &[u8], expected: &[u8]) {
    let text = String::from_utf8_lossy(output);
    let token = text.split_whitespace().next().unwrap();
    assert_eq!(token, format!("{:x}", Sha256::digest(expected)));
}

#[expect(
    clippy::too_many_arguments,
    reason = "native file and environment matrices share actual process paths"
)]
pub(super) async fn reads(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
    expected: &[u8],
    user: &str,
    script: Option<&str>,
) {
    let hash_script = script.unwrap_or("sha256sum /exact-ro");
    let captured = script
        .map(|value| vec!["/bin/sh", "-c", value])
        .unwrap_or_else(|| vec!["/usr/bin/sha256sum", "/exact-ro"]);
    let child_script = script
        .map(|value| format!("( {value} ); status=$?; exit \"$status\""))
        .unwrap_or_else(|| "sha256sum /exact-ro".into());
    for (method, args) in [
        ("init", vec!["/bin/cat", "/scratch/init.hash"]),
        ("exec", captured),
        ("child", vec!["/bin/sh", "-c", &child_script]),
    ] {
        let mut command = exec(&args);
        command.user = Some(user.into());
        let output = manager.execute(id, generation, command).await.unwrap();
        assert_eq!(
            output.exit_code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        digest(&output.stdout, expected);
        fs::write(
            root.join(format!("secret-{label}-{method}.stdout")),
            &output.stdout,
        )
        .unwrap();
    }
    for tty in [false, true] {
        let mut process = if tty {
            manager
                .start_pty(
                    id,
                    generation,
                    PtyRequest {
                        cmd: vec![
                            "/bin/sh".into(),
                            "-c".into(),
                            format!("test -t 1; {hash_script}"),
                        ],
                        env: vec![],
                        working_dir: Some("/".into()),
                        rootfs: None,
                        user: Some(user.into()),
                        cols: 80,
                        rows: 24,
                    },
                )
                .await
                .unwrap()
        } else {
            let mut command = exec(&[
                "/bin/sh",
                "-c",
                &format!("read line; test \"$line\" = streamed; {hash_script}"),
            ]);
            command.user = Some(user.into());
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
        let mut errors = Vec::new();
        let exit = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                match process.next_event().await.unwrap().unwrap() {
                    ExecEvent::Chunk(chunk) => {
                        if chunk.stream == kbox_core::exec::StreamType::Stdout {
                            bytes.extend(chunk.data)
                        } else {
                            errors.extend(chunk.data)
                        }
                    }
                    ExecEvent::Exit(status) => break status.exit_code,
                    ExecEvent::FlushAck => {}
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&errors));
        digest(&bytes, expected);
        fs::write(
            root.join(format!(
                "secret-{label}-{}.stdout",
                if tty { "pty" } else { "stream" }
            )),
            &bytes,
        )
        .unwrap();
    }
}

pub(super) async fn attacks(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
) {
    let bytes = fs::read(std::env::var_os("K_BOX_TEST_HVF_FILE_PROBE").unwrap()).unwrap();
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
    let output = manager
        .execute(id, generation, exec(&["/file-probe"]))
        .await
        .unwrap();
    fs::write(
        root.join(format!("secret-{label}-host-pep.stderr")),
        &output.stderr,
    )
    .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let proof: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(proof["hostDenials"], 16);
    assert_eq!(proof["childDenied"], true);
    assert_eq!(proof["freshMounts"], 2);
    assert_eq!(proof["writableRemount"], true);
    fs::write(
        root.join(format!("secret-{label}-host-pep.json")),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

pub(super) async fn old_agent_schema_probe(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    target: &koci_sdk::ContainerTarget,
    schema: &str,
    agent_env: &str,
) {
    let agent = PathBuf::from(std::env::var_os(agent_env).expect("exact old Agent"));
    let binary = fs::read(&agent).unwrap();
    let manifest = fs::read(
        root.join("service/runtime/shares")
            .join(target.id.as_str())
            .join(target.generation.unwrap().0.to_string())
            .join(".koci-agent-vm-attachments.json"),
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(parsed["schemaVersion"], schema);
    let digest = format!("sha256:{:x}", Sha256::digest(&manifest));
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                behavior::file(FileOp::Upload, "/old-agent-probe", Some(binary.clone()))
            )
            .await
            .unwrap()
            .success
    );
    let script = format!("set -u; chmod 755 /old-agent-probe; mkdir -p /run/koci-runtime; K_OCI_AGENT_RUNTIME_SHARE=koci-runtime K_OCI_AGENT_VM_ATTACHMENT_MANIFEST_SHA256={digest} timeout 10 /old-agent-probe; status=$?; umount /run/koci-runtime || exit 99; exit \"$status\"");
    let output = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", &script]))
        .await
        .unwrap();
    fs::write(root.join("old-agent-schema-probe.stderr"), &output.stderr).unwrap();
    assert_eq!(
        output.exit_code,
        1,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reason = String::from_utf8_lossy(&output.stderr);
    assert!(
        reason.contains(&format!(
            "unsupported utility-VM attachment manifest schema {schema}"
        )),
        "{reason}"
    );
    fs::write(root.join("old-agent-schema-proof.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","agent":agent,"agentSha256":format!("{:x}",Sha256::digest(&binary)),"actualManifestSha256":digest,"schema":schema,"nativeGuest":"linux arm64 on macOS HVF","exitCode":output.exit_code,"stderr":reason,"temporaryRuntimeMountRemoved":true})).unwrap()).unwrap();
}

pub(super) fn cli(root: &Path, id: &ExecutionId, expected: &[u8], script: Option<&str>) -> bool {
    cli_with_home(root, &root.join("home"), id, expected, script)
}

pub(super) fn cli_with_home(
    root: &Path,
    home: &Path,
    id: &ExecutionId,
    expected: &[u8],
    script: Option<&str>,
) -> bool {
    let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_CLI") else {
        return false;
    };
    let mut args = vec!["exec", id.as_str(), "--"];
    if let Some(script) = script {
        args.extend(["/bin/sh", "-c", script]);
    } else {
        args.extend(["/usr/bin/sha256sum", "/exact-ro"]);
    }
    let output = Command::new(binary)
        .env("K_HOME", home)
        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
        .env("K_BOX_OCI_MIGRATION", "off")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    digest(&output.stdout, expected);
    true
}

#[test]
#[ignore = "requires owned RAM roots and exact installed HVF archives"]
fn native_memory_secret_files_enforce_host_ro_owner_recovery_and_cleanup() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(||{
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        assert!(koci_kbox::macos_path_is_on_ram_disk(&root).unwrap());let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let store=store(&root);let a=format!("sha256:{}","a".repeat(64));let b=format!("sha256:{}","b".repeat(64));
                let (req,source)=configured(&root,&store,&image,"secret-main",&a,A,true).await;let before=fs::metadata(&source).unwrap();
                let (reader,second_source)=configured(&root,&store,&image,"secret-reader",&b,B,false).await;
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let first=manager.create_and_start(req,&OperationId::new("secret-main-create").unwrap()).await.unwrap();let second=manager.create_and_start(reader,&OperationId::new("secret-reader-create").unwrap()).await.unwrap();
                reads(&manager,&root,&second.execution_id,second.generation,"reader-first",B,&fs::metadata(&second_source).unwrap().uid().to_string(),None).await;
                fs::write(source.parent().unwrap().join("canary"),b"ungranted-private-sibling").unwrap();
                let mut lease=first.clone();let mut generations=Vec::new();let mut cli_checked=false;
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(&lease.execution_id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();
                    if wave==0 {old_agent_schema_probe(&manager,&root,&lease.execution_id,lease.generation,&binding.target,"koci.agent-vm-attachments.v8","K_BOX_TEST_HVF_OLD_AGENT").await;}
                    let config=fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap();assert!(!config.windows(A.len()).any(|v|v==A));
                    let bundle=koci_sdk::OciBundle::from_json(root.join("unused"),String::from_utf8(config.clone()).unwrap()).unwrap();let attachments=koci_sdk::CreateAttachments::from_bundle(&bundle,koci_sdk::ProcessIo::default()).unwrap();assert_eq!(attachments.memory_secret_files(&bundle).unwrap().unwrap().mount_indices().len(),1);fs::write(root.join(format!("secret-{label}-spec.json")),config).unwrap();
                    reads(&manager,&root,&lease.execution_id,lease.generation,label,A,"0",None).await;attacks(&manager,&root,&lease.execution_id,lease.generation,label).await;
                    assert_eq!(fs::read(&source).unwrap(),A);let now=fs::metadata(&source).unwrap();assert_eq!((now.dev(),now.ino(),now.uid(),now.gid(),now.mode()),(before.dev(),before.ino(),before.uid(),before.gid(),before.mode()));assert_eq!(now.nlink(),2);
                    cli_checked|=cli(&root,&lease.execution_id,A,None);
                    if wave==0 {let mut command=exec(&["/bin/sh","-c","count=$(cat /scratch/replay 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay; sha256sum /exact-ro"]);command.request_id=Some("secret-replay".into());let one=manager.execute(&lease.execution_id,lease.generation,command.clone()).await.unwrap();let two=manager.execute(&lease.execution_id,lease.generation,command.clone()).await.unwrap();assert_eq!(one.stdout,two.stdout);digest(&two.stdout,A);assert_eq!(manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/cat","/scratch/replay"])).await.unwrap().stdout,b"1");command.cmd=vec!["/bin/true".into()];assert!(manager.execute(&lease.execution_id,lease.generation,command).await.is_err());}
                    if wave<2 {let old=lease.generation;if wave==1 {host.sigkill();assert!(manager.inspect(&lease.execution_id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline{tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(&lease.execution_id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(&lease.execution_id,old,&OperationId::new(format!("secret-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(&lease.execution_id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                manager.kill(&lease.execution_id,lease.generation).await.unwrap();manager.remove(&lease.execution_id,lease.generation).await.unwrap();assert_eq!(fs::read(&source).unwrap(),A);assert_eq!(fs::metadata(&source).unwrap().nlink(),1);store.cleanup_identity(&a).await.unwrap();assert!(!source.exists());assert_eq!(fs::read(&second_source).unwrap(),B);
                // Host owner death stops both VMs; restart the independent retained reader explicitly.
                let reader=manager.restart(&second.execution_id,second.generation,&OperationId::new("secret-reader-restart").unwrap()).await.unwrap();let owner=fs::metadata(&second_source).unwrap();reads(&manager,&root,&reader.execution_id,reader.generation,"owner",B,&owner.uid().to_string(),None).await;
                let stat=manager.execute(&reader.execution_id,reader.generation,exec(&["/bin/stat","-c","%u:%g:%a","/exact-ro"])).await.unwrap();assert_eq!(stat.exit_code,0);assert_eq!(String::from_utf8(stat.stdout).unwrap().trim(),format!("{}:{}:400",owner.uid(),owner.gid()));
                manager.kill(&reader.execution_id,reader.generation).await.unwrap();manager.remove(&reader.execution_id,reader.generation).await.unwrap();assert_eq!(fs::read(&second_source).unwrap(),B);assert_eq!(fs::metadata(&second_source).unwrap().nlink(),1);store.cleanup_identity(&b).await.unwrap();assert!(!second_source.exists());
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","execution":first.execution_id,"generations":generations,"initExecChildStreamRealPty":true,"hostDenialsPerWave":16,"freshRwMountsPerWave":2,"childHostRo":true,"actualRamBacking":true,"fileOwnershipAndModePreserved":true,"replayOneEffect":true,"changedRequestRefused":true,"independentMaterial":true,"sourcePreservedUntilExplicitOwnerCleanup":true,"stoppedRecovery":true,"cliChecked":cli_checked})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none(){host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires owned RAM Secret source and exact old Host or shim assets"]
fn native_memory_secret_unsupported_transport_refuses_and_preserves_source() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(||{
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");let mut host=Some(Host::start(&root,1));
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let store=store(&root);let identity=format!("sha256:{}","c".repeat(64));
                let (mut req,source)=if std::env::var("K_BOX_TEST_SECRET_ENVIRONMENT").ok().as_deref()==Some("1") {
                    let (req,sources,_)=super::secret_environment::configured(&root,&store,&image,&identity,A,false).await;(req,sources[0].clone())
                } else {configured(&root,&store,&image,"secret-refusal",&identity,A,false).await};let before=fs::metadata(&source).unwrap();
                let marker=root.join("entry-marker");fs::write(&marker,b"not-entered").unwrap();fs::set_permissions(&marker,fs::Permissions::from_mode(0o666)).unwrap();req.config.volumes.push(format!("{}:/entry-marker:rw",marker.display()));req.config.cmd=vec!["/bin/sh".into(),"-c".into(),"printf entered > /entry-marker; exec /bin/sleep 2147483647".into()];
                let manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let result=manager.create_and_start(req,&OperationId::new("secret-refusal-create").unwrap()).await;assert!(result.is_err(),"unsupported Secret transport entered a workload");
                let error=result.unwrap_err();let reason=error.to_string();
                let expected=std::env::var("K_BOX_TEST_SECRET_EXPECTED_REFUSAL").expect("exact refusal boundary");
                assert!(reason.contains(&expected),"expected {expected:?}, actual {reason}");
                fs::write(root.join("expected-refusal.txt"),reason).unwrap();assert_eq!(fs::read(&marker).unwrap(),b"not-entered");
                let records=ManagedExecutionStore::new(root.join("home/boxes.json")).list().unwrap();
                if !records.is_empty() {
                    assert_eq!(records.len(),1);let id=ExecutionId::new(records[0].id.clone()).unwrap();
                    assert!(matches!(manager.kill(&id,ExecutionGeneration::INITIAL).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    assert!(manager.remove(&id,ExecutionGeneration::INITIAL).await.is_err());drop(manager);drop(host.take());
                    let shim=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_SHIM").expect("compatible recovery shim"));
                    let image=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RECOVERY_MANIFEST").expect("compatible recovery image"));
                    host=Some(Host::start_with_assets(&root,2,&shim,&image));assert_eq!(pending::drain(&root).await,1);
                    let proof:serde_json::Value=serde_json::from_slice(&fs::read(root.join("pending-create-cleanup.json")).unwrap()).unwrap();assert_eq!(proof["workloadStartsAdded"],0);
                }
                cleanup(&root).await;assert_eq!(fs::read(&marker).unwrap(),b"not-entered");assert_eq!(fs::read(&source).unwrap(),A);let after=fs::metadata(&source).unwrap();assert_eq!((after.dev(),after.ino(),after.uid(),after.gid(),after.mode(),after.nlink()),(before.dev(),before.ino(),before.uid(),before.gid(),before.mode(),1));store.cleanup_identity(&identity).await.unwrap();
                assert!(!root.join("service/runtime/shares").exists() || fs::read_dir(root.join("service/runtime/shares")).unwrap().all(|entry| fs::read_dir(entry.unwrap().path()).unwrap().next().is_none()));
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","unsupportedTransportRefused":true,"sourceIdentityModeAndBytesPreserved":true,"noWritableFallback":true,"zeroWorkloadAfterCleanup":true,"entryMarkerUnchanged":true})).unwrap()).unwrap();
            }).catch_unwind().await;cleanup(&root).await;if let Err(error)=result{std::panic::resume_unwind(error);}
        });drop(host);assert!(!root.join("service/runtime.sock").exists());
    }).unwrap().join().unwrap();
}
