//! Actual environment consumption from authorized RAM files; hash output only.
use super::*;
use base64::Engine as _;
use kbox_core::{ExecutionGeneration, ExecutionId, FileOp};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

pub(super) const A: &[u8] = "synthetic-memory-env-alpha\n=value-中文".as_bytes();
pub(super) const B: &[u8] = b"synthetic-memory-env-beta";
const PROBE: &[u8] = b"synthetic-private-file-probe";
const REPLACEMENT: &[u8] = b"ungranted-visible-replacement";
pub(super) const HASH: &str =
    "test -z \"${K_BOX_SECRET_ENV_V1+x}\" && printf %s \"$SYNTHETIC_ENV\" | sha256sum";

pub(super) async fn configured(
    root: &Path,
    store: &crate::BoxTransientSecretStore,
    image: &str,
    identity: &str,
    value: &[u8],
    privileged: bool,
) -> (CreateExecutionRequest, Vec<PathBuf>, String) {
    let mut material = vec![(
        "SYNTHETIC_ENV".into(),
        crate::BoxSecretMaterial::new(value.to_vec()).unwrap(),
    )];
    if privileged {
        material.push((
            "PROBE_FILE_ONLY".into(),
            crate::BoxSecretMaterial::new(PROBE.to_vec()).unwrap(),
        ));
    }
    let projection = store
        .materialize_environment(identity, material)
        .await
        .unwrap();
    let sources: Vec<_> = projection
        .volumes
        .iter()
        .map(|volume| {
            crate::vm::VmManager::parse_volume_spec(volume)
                .unwrap()
                .host_path
        })
        .collect();
    let mut bindings: Vec<kbox_core::secret::SecretEnvironmentBinding> =
        serde_json::from_str(&projection.manifest).unwrap();
    bindings.retain(|binding| binding.variable == "SYNTHETIC_ENV");
    let guest = bindings[0].path.clone();
    let mut req = request(image, if privileged { "env-main" } else { "env-reader" });
    req.policy.managed_secret_root = Some(store.root().into());
    req.config.volumes = projection.volumes;
    req.config.extra_env = vec![(
        kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.into(),
        serde_json::to_string(&bindings).unwrap(),
    )];
    req.config.tmpfs = vec!["/scratch:size=1m,mode=1777".into()];
    req.config.cmd = vec![
        "/bin/sh".into(),
        "-c".into(),
        format!("set -eu; ({HASH}) > /scratch/init.hash; exec /bin/sleep 2147483647"),
    ];
    if privileged {
        req.config.user = Some("0".into());
        req.config.cap_add = vec!["SYS_ADMIN".into()];
        req.config.volumes[1] = format!("{}:/exact-ro:ro", sources[1].display());
        let rw = root.join("env-writable-control");
        fs::write(&rw, b"initial-rw").unwrap();
        fs::set_permissions(&rw, fs::Permissions::from_mode(0o666)).unwrap();
        req.config
            .volumes
            .push(format!("{}:/exact-rw:rw", rw.display()));
    } else {
        req.config.user = Some(fs::metadata(&sources[0]).unwrap().uid().to_string());
        req.config.cap_drop = vec!["ALL".into()];
    }
    (req, sources, guest)
}

pub(super) async fn guest_units(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) {
    let binary = fs::read(
        std::env::var_os("K_BOX_TEST_SECRET_GUEST_TESTS").expect("exact Linux Agent unit ELF"),
    )
    .unwrap();
    let digest = format!("{:x}", Sha256::digest(&binary));
    assert!(
        manager
            .transfer_file(
                id,
                generation,
                behavior::file(FileOp::Upload, "/secret-guest-tests", Some(binary))
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
                exec(&["/bin/chmod", "755", "/secret-guest-tests"])
            )
            .await
            .unwrap()
            .exit_code,
        0
    );
    let output = manager
        .execute(
            id,
            generation,
            exec(&[
                "/secret-guest-tests",
                "executor::secret_env::",
                "--nocapture",
                "--test-threads=1",
            ]),
        )
        .await
        .unwrap();
    fs::write(root.join("secret-guest-units.stdout"), &output.stdout).unwrap();
    fs::write(root.join("secret-guest-units.stderr"), &output.stderr).unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("test result: ok. 8 passed; 0 failed; 0 ignored;"));
    fs::write(root.join("secret-guest-units.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","actualCounts":[8,0,0],"sha256":digest,"nativeGuest":"Linux arm64 on macOS HVF"})).unwrap()).unwrap();
}

pub(super) async fn replace_visible_path(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    guest: &str,
    label: &str,
) {
    let parent = Path::new(guest).parent().unwrap().to_str().unwrap();
    let script=format!("set -eu; mv {parent} {parent}.moved; mkdir {parent}; printf %s ungranted-visible-replacement > {guest}; chmod 400 {guest}; sha256sum {guest}");
    let output = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", &script]))
        .await
        .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    secrets::digest(&output.stdout, REPLACEMENT);
    let output = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", HASH]))
        .await
        .unwrap();
    assert_eq!(output.exit_code, 0);
    secrets::digest(&output.stdout, A);
    fs::write(root.join(format!("env-{label}-replacement.json")),serde_json::to_vec_pretty(&json!({"visibleBytesReplaced":true,"protectedSourceStillUsed":true,"noManifestInWorkload":true})).unwrap()).unwrap();
}

pub(super) async fn refusals(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    guest: &str,
    label: &str,
) {
    for case in ["retarget", "literal"] {
        let mut request = exec(&["/bin/sh", "-c", "printf entered > /scratch/refused-entry"]);
        request.request_id = Some(format!("env-refuse-{label}-{case}"));
        request.env = if case == "literal" {
            vec!["SYNTHETIC_ENV=untrusted-literal".into()]
        } else {
            vec![format!(
                "{}={}",
                kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST,
                json!([{"variable":"SYNTHETIC_ENV","path":guest.replace("000.secret","001.secret")}])
            )]
        };
        let result = manager.execute(id, generation, request).await;
        assert!(match &result {
            Ok(output) => output.exit_code != 0,
            Err(_) => true,
        });
        fs::write(
            root.join(format!("env-{label}-{case}-refusal.txt")),
            result
                .err()
                .map_or_else(|| "nonzero process exit".into(), |error| error.to_string()),
        )
        .unwrap();
        let output = manager
            .execute(
                id,
                generation,
                exec(&["/bin/sh", "-c", "test ! -e /scratch/refused-entry"]),
            )
            .await
            .unwrap();
        assert_eq!(output.exit_code, 0);
    }
}

pub(super) fn no_material_in_metadata(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|extension| {
                    ["json", "log", "stdout", "stderr", "txt"].contains(&extension)
                })
            {
                let bytes = fs::read(path).unwrap();
                for material in [A, B] {
                    assert!(!bytes
                        .windows(material.len())
                        .any(|window| window == material));
                    let quoted =
                        serde_json::to_vec(std::str::from_utf8(material).unwrap()).unwrap();
                    let escaped = &quoted[1..quoted.len() - 1];
                    assert!(!bytes.windows(escaped.len()).any(|window| window == escaped));
                    let encoded = base64::engine::general_purpose::STANDARD.encode(material);
                    assert!(!bytes
                        .windows(encoded.len())
                        .any(|window| window == encoded.as_bytes()));
                }
            }
        }
    }
}

#[test]
#[ignore = "requires actual owned RAM and exact installed memory-environment archives"]
fn native_memory_secret_environment_all_process_paths_recovery_and_cleanup() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");assert!(koci_kbox::macos_path_is_on_ram_disk(&root).unwrap());
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap();let store=secrets::store(&root);let a=format!("sha256:{}","a".repeat(64));let b=format!("sha256:{}","b".repeat(64));
                let (req,sources,guest)=configured(&root,&store,&image,&a,A,true).await;let before:Vec<_>=sources.iter().map(|path|fs::metadata(path).unwrap()).collect();
                let (reader,second_sources,_)=configured(&root,&store,&image,&b,B,false).await;
                let mut manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
                let first=manager.create_and_start(req,&OperationId::new("secret-env-main-create").unwrap()).await.unwrap();let second=manager.create_and_start(reader,&OperationId::new("secret-env-reader-create").unwrap()).await.unwrap();
                secrets::reads(&manager,&root,&second.execution_id,second.generation,"env-reader-first",B,&fs::metadata(&second_sources[0]).unwrap().uid().to_string(),Some(HASH)).await;
                let mut lease=first.clone();let mut generations=Vec::new();let mut cli_checked=false;
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());let record=ManagedExecutionStore::new(root.join("home/boxes.json")).get(&lease.execution_id).unwrap().unwrap();let metadata=record.managed_execution.unwrap();assert_eq!(metadata.runtime_route,ManagedRuntimeRoute::OciSdk);let binding=metadata.oci_runtime.unwrap();
                    let config=fs::read(root.join("service/runtime/shares").join(binding.target.id.as_str()).join(binding.target.generation.unwrap().0.to_string()).join("bundle/config.json")).unwrap();let bundle=koci_sdk::OciBundle::from_json(root.join("unused"),String::from_utf8(config.clone()).unwrap()).unwrap();let attachments=koci_sdk::CreateAttachments::from_bundle(&bundle,koci_sdk::ProcessIo::default()).unwrap();let memory=attachments.memory_secret_files(&bundle).unwrap().unwrap();assert_eq!(memory.version(),2);assert_eq!(memory.environment_bindings(&bundle).unwrap().len(),1);fs::write(root.join(format!("env-{label}-spec.json")),config).unwrap();
                    if wave==0 {guest_units(&manager,&root,&lease.execution_id,lease.generation).await;secrets::old_agent_schema_probe(&manager,&root,&lease.execution_id,lease.generation,&binding.target,"koci.agent-vm-attachments.v9","K_BOX_TEST_HVF_OLD_ENV_AGENT").await;}
                    secrets::reads(&manager,&root,&lease.execution_id,lease.generation,label,A,"0",Some(HASH)).await;secrets::attacks(&manager,&root,&lease.execution_id,lease.generation,label).await;refusals(&manager,&root,&lease.execution_id,lease.generation,&guest,label).await;
                    replace_visible_path(&manager,&root,&lease.execution_id,lease.generation,&guest,label).await;cli_checked|=secrets::cli(&root,&lease.execution_id,A,Some(HASH));
                    for (index,path) in sources.iter().enumerate() {assert!(fs::read(path).unwrap()==if index==0 {A} else {PROBE});let now=fs::metadata(path).unwrap();let old=&before[index];assert_eq!((now.dev(),now.ino(),now.uid(),now.gid(),now.mode(),now.nlink()),(old.dev(),old.ino(),old.uid(),old.gid(),old.mode(),2));}
                    if wave==0 {
                        let script=format!("count=$(cat /scratch/replay 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay; {HASH}");let mut command=exec(&["/bin/sh","-c",&script]);command.request_id=Some("env-replay".into());let one=manager.execute(&lease.execution_id,lease.generation,command.clone()).await.unwrap();let two=manager.execute(&lease.execution_id,lease.generation,command.clone()).await.unwrap();assert_eq!(one.stdout,two.stdout);secrets::digest(&two.stdout,A);assert_eq!(manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/cat","/scratch/replay"])).await.unwrap().stdout,b"1");command.cmd=vec!["/bin/true".into()];assert!(manager.execute(&lease.execution_id,lease.generation,command).await.is_err());
                    }
                    no_material_in_metadata(&root);
                    if wave<2 {let old=lease.generation;if wave==1 {host.sigkill();assert!(manager.inspect(&lease.execution_id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline {tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(&lease.execution_id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(&lease.execution_id,old,&OperationId::new(format!("env-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(&lease.execution_id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                manager.kill(&lease.execution_id,lease.generation).await.unwrap();manager.remove(&lease.execution_id,lease.generation).await.unwrap();for path in &sources {assert_eq!(fs::metadata(path).unwrap().nlink(),1);}store.cleanup_identity(&a).await.unwrap();assert!(sources.iter().all(|path|!path.exists()));assert!(fs::read(&second_sources[0]).unwrap()==B);
                let reader=manager.restart(&second.execution_id,second.generation,&OperationId::new("env-reader-restart").unwrap()).await.unwrap();secrets::reads(&manager,&root,&reader.execution_id,reader.generation,"env-reader-recovery",B,&fs::metadata(&second_sources[0]).unwrap().uid().to_string(),Some(HASH)).await;manager.kill(&reader.execution_id,reader.generation).await.unwrap();manager.remove(&reader.execution_id,reader.generation).await.unwrap();assert!(fs::read(&second_sources[0]).unwrap()==B);assert_eq!(fs::metadata(&second_sources[0]).unwrap().nlink(),1);store.cleanup_identity(&b).await.unwrap();no_material_in_metadata(&root);
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","generations":generations,"cliChecked":cli_checked,"initExecChildStreamRealPty":true,"hostDenialsPerWave":16,"freshRwMountsPerWave":2,"protectedSourceAfterVisibleReplacement":true,"retargetAndLiteralRefusedBeforeEntry":true,"manifestConsumed":true,"plaintextAbsentFromMetadata":true,"guestUnitCounts":[8,0,0],"replayOneEffect":true,"staleGenerationRefused":true,"independentMaterial":true,"stoppedRecovery":true,"sourcePreservedUntilOwnerCleanup":true})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none() {host=Host::start(&root,99);}cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result {std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
