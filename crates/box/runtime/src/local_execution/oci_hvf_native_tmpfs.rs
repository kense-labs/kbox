//! Actual tmpfs intent, ephemeral lifetime and retained storage through the Host.
use super::*;
use kbox_core::{volume::VolumeConfig, ExecutionGeneration, ExecutionId};
use std::os::unix::fs::MetadataExt;

async fn probe(
    manager: &LocalExecutionManager,
    root: &Path,
    id: &ExecutionId,
    generation: ExecutionGeneration,
    label: &str,
) {
    let script = r#"set -eu
for path in /scratch /sealed '/临时 scratch' /owned /ownership /gid-owned '/空 卷'; do
  test "$(/bin/busybox stat -f -c %T "$path")" = tmpfs
done
test "$(/bin/busybox stat -c '%a:%u:%g' /owned)" = 1770:123:456
test "$(/bin/busybox stat -c %a /sealed)" = 1777
printf writable > /scratch/value
/bin/sh -c 'printf child > /scratch/child'
for command in 'touch /sealed/new' 'mkdir /sealed/newdir' 'sh -c "printf child > /sealed/child"'; do
  if /bin/sh -c "$command" 2>/tmpfs-ro-error; then exit 31; fi
  grep -q 'Read-only file system' /tmpfs-ro-error
done
printf '#!/bin/sh\nexit 0\n' > /scratch/program
chmod 755 /scratch/program
if /scratch/program 2>/tmpfs-exec-error; then exit 32; fi
grep -q 'Permission denied' /tmpfs-exec-error
if dd if=/dev/zero of=/scratch/fill bs=4096 count=512 2>/tmpfs-quota-error; then exit 33; fi
grep -q 'No space left on device' /tmpfs-quota-error
rm /scratch/fill
printf unicode > '/临时 scratch/value'
cat /proc/self/mountinfo
printf 'TMPFS_EFFECTS_PASS\n'
"#;
    let output = manager
        .execute(id, generation, exec(&["/bin/sh", "-c", script]))
        .await
        .unwrap();
    fs::write(root.join(format!("tmpfs-{label}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("tmpfs-{label}.stderr")), &output.stderr).unwrap();
    assert_eq!(
        output.exit_code,
        0,
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    for (target, flags) in [
        ("/scratch", &["rw", "nosuid", "nodev", "noexec"][..]),
        ("/sealed", &["ro", "nosuid", "nodev"][..]),
    ] {
        let line = output
            .lines()
            .find(|line| line.split_whitespace().nth(4) == Some(target))
            .unwrap();
        let options: Vec<_> = line.split_whitespace().nth(5).unwrap().split(',').collect();
        for flag in flags {
            assert!(options.contains(flag), "{line}");
        }
        assert!(line.contains(" - tmpfs "), "{line}");
    }
    let mut user = exec(&[
        "/bin/sh",
        "-c",
        "test $(id -u) = 123; test $(id -g) = 456; printf owner > /owned/value",
    ]);
    user.user = Some("123:456".into());
    assert_eq!(
        manager
            .execute(id, generation, user)
            .await
            .unwrap()
            .exit_code,
        0
    );
}

fn tmpfs_request(image: &str, label: &str) -> CreateExecutionRequest {
    let mut req = request(image, label);
    req.config.user = Some("0".into());
    req.config.cap_drop = vec!["ALL".into()];
    req.config.tmpfs = vec![
        "/scratch:size=1m,mode=1777,rw,nosuid,nodev,noexec".into(),
        "/sealed:size=4096,mode=1777,ro,nosuid,nodev".into(),
        "/临时 scratch:size=64k,mode=1777,rw".into(),
        "/owned:size=64k,mode=1770,uid=123,gid=456,rw".into(),
        "/ownership:size=64k,mode=1777,rw".into(),
        "/gid-owned:size=64k,mode=1777,rw".into(),
        "/空 卷:size=64k,mode=1777,rw".into(),
    ];
    req.config.cmd = vec!["/bin/sh".into(), "-c".into(), "test $(stat -f -c %T /scratch) = tmpfs && printf init > /scratch/init && exec /bin/sleep 2147483647".into()];
    req
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
        "--tmpfs",
        "/scratch:size=1m,mode=1777,noexec,nosuid,nodev",
        "--tmpfs",
        "/ownership:size=64k",
        "--tmpfs",
        "/gid-owned:size=64k",
        "--tmpfs",
        "/空 卷:size=64k",
        image,
        "--",
        "/bin/sleep",
        "2147483647",
    ]))
    .unwrap()
    .trim()
    .to_owned();
    run(&[
        "exec",
        &id,
        "--",
        "/bin/sh",
        "-c",
        "test $(stat -f -c %T /scratch) = tmpfs && printf cli > /scratch/cli",
    ]);
    let record: serde_json::Value = serde_json::from_slice(&run(&["inspect", &id])).unwrap();
    assert_eq!(record[0]["managed_execution"]["runtime_route"], "oci_sdk");
    assert_eq!(record[0]["anonymous_volumes"], json!([]));
    run(&["rm", "-f", &id]);
    true
}

#[test]
#[ignore = "requires sentinel-owned home and fixed installed independent HVF assets"]
fn native_tmpfs_intent_enforces_flags_quota_identity_restart_and_stopped_recovery() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(), root);
        assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(), "owned-hvf-native-test\n");
        let mut host = Host::start(&root, 1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
            let volumes = crate::VolumeStore::new(root.join("home/volumes.json"), root.join("home/volumes"));
            let volume = volumes.create(VolumeConfig::new("tmpfs-retained", "")).unwrap();
            let source = PathBuf::from(&volume.mount_point);
            fs::write(source.join("canary"), b"retained-named").unwrap();
            let caller = root.join("caller"); fs::create_dir(&caller).unwrap();
            fs::write(caller.join("canary"), b"retained-caller").unwrap();
            let result = std::panic::AssertUnwindSafe(async {
                let image = std::env::var("K_BOX_TEST_HVF_VOLUME_IMAGE").unwrap();
                let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                let mut req = tmpfs_request(&image, "tmpfs-a");
                req.config.volumes = vec![format!("{}:/data", source.display()), format!("{}:/caller", caller.display())];
                req.policy.volume_names = vec![volume.name.clone()];
                let first = manager.create_and_start(req, &OperationId::new("tmpfs-a-create").unwrap()).await.unwrap();
                let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
                let record = store.get(&first.execution_id).unwrap().unwrap();
                assert!(record.anonymous_volumes.is_empty());
                assert_eq!(record.managed_execution.as_ref().unwrap().runtime_route, ManagedRuntimeRoute::OciSdk);
                assert_eq!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/cat", "/scratch/init"])).await.unwrap().stdout, b"init");
                probe(&manager, &root, &first.execution_id, first.generation, "first").await;
                let second = manager.create_and_start(tmpfs_request(&image, "tmpfs-b"), &OperationId::new("tmpfs-b-create").unwrap()).await.unwrap();
                let isolated = manager.execute(&second.execution_id, second.generation, exec(&["/bin/sh", "-c", "test ! -e /scratch/value && test $(cat /scratch/init) = init && printf second > /scratch/value"])).await.unwrap();
                assert_eq!(isolated.exit_code, 0);
                assert_eq!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/cat", "/scratch/value"])).await.unwrap().stdout, b"writable");
                manager.kill(&second.execution_id, second.generation).await.unwrap(); manager.remove(&second.execution_id, second.generation).await.unwrap();
                let owner = fs::metadata(&caller).unwrap();
                let mut write = exec(&["/bin/sh", "-c", "set -eu; printf named-live > /data/live; printf caller-live > /caller/live"]);
                write.user = Some(format!("{}:{}", owner.uid(), owner.gid()));
                let written = manager.execute(&first.execution_id, first.generation, write).await.unwrap();
                fs::write(root.join("retained-write.stderr"), &written.stderr).unwrap();
                assert_eq!(written.exit_code, 0, "{}", String::from_utf8_lossy(&written.stderr));
                let restarted = manager.restart(&first.execution_id, first.generation, &OperationId::new("tmpfs-normal-restart").unwrap()).await.unwrap();
                assert_eq!(restarted.generation.get(), first.generation.get() + 1);
                assert!(matches!(manager.execute(&first.execution_id, first.generation, exec(&["/bin/touch", "/caller/stale"])).await, Err(kbox_core::ExecutionManagerError::Conflict{..})));
                assert!(!caller.join("stale").exists());
                assert_eq!(manager.execute(&first.execution_id, restarted.generation, exec(&["/bin/sh", "-c", "test ! -e /scratch/value && test ! -e /owned/value && test ! -e '/临时 scratch/value' && test $(cat /data/live) = named-live && test $(cat /caller/live) = caller-live"])).await.unwrap().exit_code, 0);
                probe(&manager, &root, &first.execution_id, restarted.generation, "restart").await;
                let cli_checked = cli(&root, &image);
                let binding = store.get(&first.execution_id).unwrap().unwrap().managed_execution.unwrap().oci_runtime.unwrap();
                host.sigkill(); assert!(manager.inspect(&first.execution_id).await.is_err());
                let recovery = root.join("service/runtime/recovery").join(format!("{}-{}.json", binding.target.id, binding.target.generation.unwrap().0));
                let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
                while !recovery.exists() && tokio::time::Instant::now() < deadline { tokio::time::sleep(Duration::from_millis(100)).await; }
                assert!(recovery.is_file()); host = Host::start(&root, 2);
                let recovered_manager = connect(&root, OciMigrationPolicy::LegacyOnly).await;
                assert_eq!(recovered_manager.inspect(&first.execution_id).await.unwrap().state, ExecutionState::Stopped);
                let recovered = recovered_manager.restart(&first.execution_id, restarted.generation, &OperationId::new("tmpfs-owner-restart").unwrap()).await.unwrap();
                assert_eq!(recovered.generation.get(), restarted.generation.get() + 1);
                assert!(matches!(recovered_manager.execute(&first.execution_id, restarted.generation, exec(&["/bin/touch", "/caller/stale"])).await, Err(kbox_core::ExecutionManagerError::Conflict{..})));
                assert_eq!(recovered_manager.execute(&first.execution_id, recovered.generation, exec(&["/bin/sh", "-c", "test ! -e /scratch/value && test ! -e /owned/value && test ! -e '/临时 scratch/value' && test $(cat /data/live) = named-live && test $(cat /caller/live) = caller-live"])).await.unwrap().exit_code, 0);
                probe(&recovered_manager, &root, &first.execution_id, recovered.generation, "recovery").await;
                let final_record = store.get(&first.execution_id).unwrap().unwrap(); assert!(final_record.anonymous_volumes.is_empty());
                assert_eq!(final_record.id, first.execution_id.as_str());
                assert_eq!(final_record.managed_execution.unwrap().runtime_route, ManagedRuntimeRoute::OciSdk);
                recovered_manager.kill(&first.execution_id, recovered.generation).await.unwrap(); recovered_manager.remove(&first.execution_id, recovered.generation).await.unwrap();
                fs::write(root.join("result.json"), serde_json::to_vec_pretty(&json!({"status":"PASS","execution":first.execution_id,"generations":[first.generation,restarted.generation,recovered.generation],"actualTmpfs":true,"sizeEnospc":true,"roErofs":true,"noexecEacces":true,"mountFlagsReadback":["nosuid","nodev","noexec"],"ownership":[123,456],"perExecutionIsolation":true,"newGenerationEmpty":true,"stoppedRecovery":true,"noAnonymousAllocations":true,"retainedCallerAndNamed":true,"cliChecked":cli_checked,"nosuidIndependentPrivilegeProbe":false})).unwrap()).unwrap();
            }).catch_unwind().await;
            if host.child.is_none() { host = Host::start(&root, 99); }
            cleanup(&root).await;
            assert_eq!(fs::read(caller.join("canary")).unwrap(), b"retained-caller");
            assert_eq!(fs::read(source.join("canary")).unwrap(), b"retained-named");
            let retained = volumes.get(&volume.name).unwrap().unwrap(); assert!(retained.in_use_by.is_empty());
            fs::write(root.join("tmpfs-storage-retained.json"), serde_json::to_vec_pretty(&json!({"named":retained,"callerRetained":true,"namedRetained":true})).unwrap()).unwrap();
            volumes.remove(&volume.name, false).unwrap(); assert!(volumes.list().unwrap().is_empty());
            drop(host); assert!(!root.join("service/runtime.sock").exists());
            if let Err(error) = result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}
