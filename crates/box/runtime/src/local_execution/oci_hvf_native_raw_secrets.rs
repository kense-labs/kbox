//! RAM-only Secrets alongside separately owned persistent raw anonymous disks.
use super::*;
use kbox_core::{ExecutionGeneration, ExecutionId};
use std::os::unix::fs::MetadataExt;

fn inventory(
    root: &Path,
    home: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    nproc: bool,
) -> Vec<serde_json::Value> {
    let record = ManagedExecutionStore::new(home.join("boxes.json"))
        .get(id)
        .unwrap()
        .unwrap();
    let metadata = record.managed_execution.as_ref().unwrap();
    assert_eq!(metadata.runtime_route, ManagedRuntimeRoute::OciSdk);
    assert_eq!(metadata.generation, generation);
    let binding = metadata.oci_runtime.as_ref().unwrap();
    let share = root
        .join("service/runtime/shares")
        .join(binding.target.id.as_str())
        .join(binding.target.generation.unwrap().0.to_string());
    assert!(koci_kbox::macos_path_is_on_ram_disk(&share).unwrap());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(share.join(".koci-agent-vm-attachments.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["schemaVersion"],
        if nproc {
            "koci.agent-vm-attachments.v10"
        } else {
            "koci.agent-vm-attachments.v11"
        }
    );
    if nproc {
        assert_eq!(
            manifest["nprocTransportSchema"],
            "koci.agent-vm-attachments.v11"
        );
    }
    assert_eq!(
        manifest["target"],
        serde_json::to_value(&binding.target).unwrap()
    );
    let config = fs::read(share.join("bundle/config.json")).unwrap();
    let bundle =
        koci_sdk::OciBundle::from_json(root.join("unused"), String::from_utf8(config).unwrap())
            .unwrap();
    let memory = koci_sdk::MemorySecretFiles::from_bundle(&bundle)
        .unwrap()
        .unwrap();
    assert_eq!(memory.version(), 3);
    assert_eq!(
        memory.environment_bindings(&bundle).unwrap().len(),
        usize::from(!nproc)
    );
    assert!(koci_sdk::raw_storage_images_requested(&bundle).unwrap());
    let volumes = crate::VolumeStore::new(home.join("volumes.json"), home.join("volumes"));
    assert_eq!(record.anonymous_volumes.len(), 3);
    assert_eq!(manifest["storage"].as_array().unwrap().len(), 3);
    let mut result = Vec::new();
    for name in &record.anonymous_volumes {
        let volume = volumes.require_anonymous(name, id.as_str()).unwrap();
        let source = crate::volume::anonymous_ext4_disk_path(Path::new(&volume.mount_point));
        assert!(!koci_kbox::macos_path_is_on_ram_disk(&source).unwrap());
        let stat = fs::metadata(&source).unwrap();
        let entries: Vec<_> = manifest["storage"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["hostSource"] == source.to_string_lossy().as_ref())
            .collect();
        assert_eq!(entries.len(), 1);
        let entry = entries[0];
        assert_eq!(entry["sourceIdentity"]["device"], stat.dev());
        assert_eq!(entry["sourceIdentity"]["inode"], stat.ino());
        assert_eq!(
            entry["sourceIdentity"]["size"],
            u64::from(ResourceConfig::default().disk_mb) * 1024 * 1024
        );
        assert_eq!(entry["ownership"], "caller");
        assert_eq!(entry["cleanup"], "detach-only");
        assert_eq!(entry["accessMode"], "read-write");
        let allocation =
            uuid::Uuid::parse_str(&volume.labels[crate::volume::ALLOCATION_ID_LABEL]).unwrap();
        assert_eq!(
            entry["identity"],
            format!("kbox.volume.v1.{}", allocation.simple())
        );
        assert_eq!(stat.uid(), fs::metadata(home).unwrap().uid());
        assert_eq!(stat.mode() & 0o777, 0o600);
        result.push(json!({"name":name,"identity":entry["identity"],"source":source,"sourceIdentity":entry["sourceIdentity"]}));
    }
    fs::write(
        root.join(format!(
            "raw-secret-manifest-{id}-g{}.json",
            generation.get()
        )),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    result
}

async fn volume_data(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    initialize: bool,
) {
    let script = if initialize {
        "set -eu; test \"$(cat /ownership/secret)\" = seed; test /ownership/secret -ef /ownership/alias; test \"$(readlink /ownership/link)\" = secret; test \"$(stat -c '%u:%g:%a' /ownership/secret)\" = 1001:2002:600; printf persistent-raw-data > '/空 卷/raw-secret-marker'; sync"
    } else {
        "set -eu; test \"$(cat '/空 卷/raw-secret-marker')\" = persistent-raw-data; test \"$(cat /ownership/secret)\" = seed; test /ownership/secret -ef /ownership/alias; test \"$(stat -c '%u:%g:%a' /ownership/secret)\" = 1001:2002:600"
    };
    let output = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", script]))
        .await
        .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn file_only_control(
    root: &Path,
    home: &Path,
    store: &crate::BoxTransientSecretStore,
    image: &str,
    other_disks: &[serde_json::Value],
) {
    let manager = connect_with_home(root, home, OciMigrationPolicy::MicrovmViaOci).await;
    let identity = format!("sha256:{}", "c".repeat(64));
    let (mut req, sources, _) =
        secret_environment::configured(root, store, image, &identity, secret_environment::B, false)
            .await;
    let uid = fs::metadata(&sources[0]).unwrap().uid().to_string();
    req.config.extra_env.clear();
    req.config.volumes[0] = format!("{}:/exact-ro:ro", sources[0].display());
    req.config.resource_limits.ulimits = vec!["nproc=16:16".into()];
    req.config.cmd = vec!["/bin/sh".into(),"-c".into(),"set -eu; test -z \"${SYNTHETIC_ENV+x}\"; sha256sum /exact-ro > /scratch/init.hash; exec /bin/sleep 2147483647".into()];
    let lease = manager
        .create_and_start(req, &OperationId::new("raw-secret-file-control").unwrap())
        .await
        .unwrap();
    let disks = inventory(root, home, &lease.execution_id, lease.generation, true);
    for original in other_disks {
        for disk in &disks {
            assert_ne!(original["identity"], disk["identity"]);
            assert_ne!(original["sourceIdentity"], disk["sourceIdentity"]);
        }
    }
    secrets::reads(
        &manager,
        root,
        &lease.execution_id,
        lease.generation,
        "raw-file-control",
        secret_environment::B,
        &uid,
        None,
    )
    .await;
    let command = format!("set -eu; test \"$(id -u)\" = {uid}; test \"$(ulimit -Su)\" = 16; test \"$(ulimit -Hu)\" = 16; test -z \"${{SYNTHETIC_ENV+x}}\"; test -r '/空 卷'; test -x '/空 卷'; test ! -e '/空 卷/raw-secret-marker'; printf file-nproc-bound");
    let mut req = exec(&["/bin/sh", "-c", &command]);
    req.user = Some(uid);
    let output = manager
        .execute(&lease.execution_id, lease.generation, req)
        .await
        .unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"file-nproc-bound");
    manager
        .kill(&lease.execution_id, lease.generation)
        .await
        .unwrap();
    assert!(disks
        .iter()
        .all(|disk| Path::new(disk["source"].as_str().unwrap()).is_file()));
    manager
        .remove(&lease.execution_id, lease.generation)
        .await
        .unwrap();
    assert!(disks
        .iter()
        .all(|disk| !Path::new(disk["source"].as_str().unwrap()).exists()));
    assert_eq!(fs::metadata(&sources[0]).unwrap().nlink(), 1);
    assert_eq!(fs::read(&sources[0]).unwrap(), secret_environment::B);
    store.cleanup_identity(&identity).await.unwrap();
    fs::write(root.join("raw-file-control.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","nprocEnvelope":true,"allFivePaths":true,"numericUid":fs::metadata(home).unwrap().uid(),"rawDisksRemoved":true,"sourcePreservedUntilOwnerCleanup":true})).unwrap()).unwrap();
}

fn cli(root: &Path, home: &Path, id: &ExecutionId) {
    assert!(secrets::cli_with_home(
        root,
        home,
        id,
        secret_environment::A,
        Some(secret_environment::HASH)
    ));
    let script = PathBuf::from(std::env::var_os("K_BOX_TEST_RAW_SECRET_CLI_SCRIPT").unwrap());
    let binary = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_CLI").unwrap());
    let output = Command::new("/usr/bin/python3")
        .arg(script)
        .arg(binary)
        .arg(id.as_str())
        .arg(root.join("raw-secret-cli-pty"))
        .env("K_HOME", home)
        .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
        .env("K_BOX_OCI_MIGRATION", "off")
        .output()
        .unwrap();
    fs::write(
        root.join("raw-secret-cli-pty-driver.stdout"),
        &output.stdout,
    )
    .unwrap();
    fs::write(
        root.join("raw-secret-cli-pty-driver.stderr"),
        &output.stderr,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("raw-secret-cli-pty/result.json")).unwrap())
            .unwrap();
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["cases"].as_array().unwrap().len(), 2);
}

#[test]
#[ignore = "requires actual owned RAM control root, disk home and installed combined-v3/v11 assets"]
fn native_raw_secret_files_environment_disks_entries_and_recovery() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());let home=PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(home.canonicalize().unwrap(),home);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");assert!(koci_kbox::macos_path_is_on_ram_disk(&root).unwrap());assert!(!koci_kbox::macos_path_is_on_ram_disk(&home).unwrap());
        let mut host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let image=std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();let store=secrets::store(&root);let identity=format!("sha256:{}","a".repeat(64));
                let (req,sources,guest)=secret_environment::configured(&root,&store,&image,&identity,secret_environment::A,true).await;let before:Vec<_>=sources.iter().map(|p|fs::metadata(p).unwrap()).collect();
                let mut manager=connect_with_home(&root,&home,OciMigrationPolicy::MicrovmViaOci).await;
                let first=manager.create(req.clone(),&OperationId::new("raw-secret-create").unwrap()).await.unwrap();let replay=manager.create(req,&OperationId::new("raw-secret-create").unwrap()).await.unwrap();assert_eq!((first.execution_id.clone(),first.generation),(replay.execution_id,replay.generation));
                let volumes=crate::VolumeStore::new(home.join("volumes.json"),home.join("volumes"));assert!(volumes.list().unwrap().is_empty());let first=manager.start(&first.execution_id,first.generation).await.unwrap();
                let original=inventory(&root,&home,&first.execution_id,first.generation,false);volume_data(&manager,&first.execution_id,first.generation,true).await;
                let mut lease=first.clone();let mut generations=Vec::new();
                for wave in 0..3 {
                    let label=["first","restart","recovery"][wave];generations.push(lease.generation.get());assert_eq!(inventory(&root,&home,&lease.execution_id,lease.generation,false),original);volume_data(&manager,&lease.execution_id,lease.generation,false).await;
                    let record=ManagedExecutionStore::new(home.join("boxes.json")).get(&lease.execution_id).unwrap().unwrap();let binding=record.managed_execution.unwrap().oci_runtime.unwrap();
                    if wave==0 {secret_environment::guest_units(&manager,&root,&lease.execution_id,lease.generation).await;secrets::old_agent_schema_probe(&manager,&root,&lease.execution_id,lease.generation,&binding.target,"koci.agent-vm-attachments.v11","K_BOX_TEST_HVF_OLD_ENV_AGENT").await;}
                    secrets::reads(&manager,&root,&lease.execution_id,lease.generation,label,secret_environment::A,"0",Some(secret_environment::HASH)).await;secrets::attacks(&manager,&root,&lease.execution_id,lease.generation,label).await;secret_environment::refusals(&manager,&root,&lease.execution_id,lease.generation,&guest,label).await;secret_environment::replace_visible_path(&manager,&root,&lease.execution_id,lease.generation,&guest,label).await;
                    for (index,path) in sources.iter().enumerate(){let old=&before[index];let now=fs::metadata(path).unwrap();assert_eq!((now.dev(),now.ino(),now.uid(),now.gid(),now.mode(),now.nlink()),(old.dev(),old.ino(),old.uid(),old.gid(),old.mode(),2));assert!(koci_kbox::macos_path_is_on_ram_disk(path).unwrap());}
                    if wave==0 {cli(&root,&home,&lease.execution_id);let script=format!("count=$(cat /scratch/replay 2>/dev/null || printf 0); printf %s $((count+1)) > /scratch/replay; {}",secret_environment::HASH);let mut request=exec(&["/bin/sh","-c",&script]);request.request_id=Some("raw-secret-replay".into());let one=manager.execute(&lease.execution_id,lease.generation,request.clone()).await.unwrap();let two=manager.execute(&lease.execution_id,lease.generation,request.clone()).await.unwrap();assert_eq!(one.stdout,two.stdout);secrets::digest(&two.stdout,secret_environment::A);assert_eq!(manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/cat","/scratch/replay"])).await.unwrap().stdout,b"1");request.cmd=vec!["/bin/true".into()];assert!(manager.execute(&lease.execution_id,lease.generation,request).await.is_err());}
                    secret_environment::no_material_in_metadata(&root);secret_environment::no_material_in_metadata(&home);
                    if wave<2 {let old=lease.generation;if wave==1 {host.sigkill();assert!(manager.inspect(&lease.execution_id).await.is_err());let recovery=root.join("service/runtime/recovery").join(format!("{}-{}.json",binding.target.id,binding.target.generation.unwrap().0));let deadline=tokio::time::Instant::now()+Duration::from_secs(20);while !recovery.exists() && tokio::time::Instant::now()<deadline{tokio::time::sleep(Duration::from_millis(100)).await;}assert!(recovery.is_file());host=Host::start(&root,2);manager=connect_with_home(&root,&home,OciMigrationPolicy::LegacyOnly).await;assert_eq!(manager.inspect(&lease.execution_id).await.unwrap().state,ExecutionState::Stopped);}
                        lease=manager.restart(&lease.execution_id,old,&OperationId::new(format!("raw-secret-restart-{wave}")).unwrap()).await.unwrap();assert_eq!(lease.generation.get(),old.get()+1);assert!(matches!(manager.execute(&lease.execution_id,old,exec(&["/bin/true"])).await,Err(kbox_core::ExecutionManagerError::Conflict{..})));
                    }
                }
                file_only_control(&root,&home,&store,&image,&original).await;volume_data(&manager,&lease.execution_id,lease.generation,false).await;
                manager.kill(&lease.execution_id,lease.generation).await.unwrap();assert!(original.iter().all(|disk|Path::new(disk["source"].as_str().unwrap()).is_file()));
                manager.remove(&lease.execution_id,lease.generation).await.unwrap();assert!(original.iter().all(|disk|!Path::new(disk["source"].as_str().unwrap()).exists()));
                for path in &sources{assert_eq!(fs::metadata(path).unwrap().nlink(),1);}assert_eq!(fs::read(&sources[0]).unwrap(),secret_environment::A);store.cleanup_identity(&identity).await.unwrap();assert!(sources.iter().all(|p|!p.exists()));assert!(volumes.list().unwrap().is_empty());
                secret_environment::no_material_in_metadata(&root);secret_environment::no_material_in_metadata(&home);
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","generations":generations,"cliChecked":true,"cliRealPtyCases":2,"initExecChildStreamRealPty":true,"hostDenialsPerWave":16,"rawDisksOnDisk":true,"secretSourceAndProjectionOnRam":true,"rawSeedOwnerLinksAndPersistentData":true,"rawIdentityAcrossGenerations":true,"fileOnlyNprocEnvelopeControl":true,"protectedSourceAfterVisibleReplacement":true,"retargetAndLiteralRefusedBeforeEntry":true,"guestUnitCounts":[8,0,0],"oldAgentV11Refused":true,"replayOneEffect":true,"staleGenerationRefused":true,"stoppedRecovery":true,"sourcePreservedUntilOwnerCleanup":true,"rawDisksRemovedByOwner":true,"plaintextAbsentFromMetadata":true})).unwrap()).unwrap();
            }).catch_unwind().await;if host.child.is_none(){host=Host::start(&root,99);}cleanup_with_home(&root,&home).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result{std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
