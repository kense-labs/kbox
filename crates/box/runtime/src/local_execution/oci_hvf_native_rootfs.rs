//! Native ownership and per-generation workload-disk checks.
use super::*;
use kbox_core::{ExecutionGeneration, ExecutionId};

pub(super) async fn check(
    manager: &LocalExecutionManager,
    id: &ExecutionId,
    generation: ExecutionGeneration,
) -> serde_json::Value {
    let stat = manager.execute(id, generation, exec(&["/bin/sh", "-c", "set -eu; stat -c '%n:%u:%g:%a' / /ownership /ownership/secret /ownership/alias /ownership/link /gid-owned /gid-owned/group /root-owned; test /ownership/secret -ef /ownership/alias; test \"$(readlink /ownership/link)\" = secret; test ! -e /removed; test ! -e /opaque/old; test \"$(cat /opaque/new)\" = upper"])).await.unwrap();
    assert_eq!(
        stat.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&stat.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&stat.stdout), "/:0:0:755\n/ownership:1001:2002:700\n/ownership/secret:1001:2002:600\n/ownership/alias:1001:2002:600\n/ownership/link:1001:2002:777\n/gid-owned:0:3003:770\n/gid-owned/group:0:3003:640\n/root-owned:0:0:600\n");
    let mut allowed = exec(&["/bin/sh", "-c", "set -eu; test \"$(cat /ownership/secret)\" = seed; printf owned > /ownership/created; printf changed >> /ownership/secret; test ! -r /root-owned"]);
    allowed.user = Some("1001:2002".into());
    let allowed = manager.execute(id, generation, allowed).await.unwrap();
    assert_eq!(
        allowed.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&allowed.stderr)
    );
    let mut denied = exec(&["/bin/sh", "-c", "set -eu; if cat /ownership/secret; then exit 31; fi; if printf illegal > /ownership/denied; then exit 32; fi; test ! -e /ownership/denied"]);
    denied.user = Some("1002:2003".into());
    let denied = manager.execute(id, generation, denied).await.unwrap();
    assert_eq!(
        denied.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    let mut group = exec(&["/bin/sh", "-c", "set -eu; test \"$(cat /gid-owned/group)\" = group-secret; printf group-write > /gid-owned/created; if printf illegal >> /gid-owned/group; then exit 33; fi"]);
    group.user = Some("1002:3003".into());
    let group = manager.execute(id, generation, group).await.unwrap();
    assert_eq!(
        group.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&group.stderr)
    );
    json!({"status":"PASS","generation":generation,"stat":String::from_utf8_lossy(&stat.stdout),"ownerWriteExit":allowed.exit_code,"otherUidDeniedExit":denied.exit_code,"foreignGidPermissionsExit":group.exit_code,"negativeStderr":String::from_utf8_lossy(&denied.stderr)})
}

pub(super) async fn exercise(manager: &LocalExecutionManager, root: &Path, image: &str) {
    let first = manager
        .create(
            request(image, "foreign-owner-a"),
            &OperationId::new("hvf-foreign-owner-a").unwrap(),
        )
        .await
        .unwrap();
    manager
        .start(&first.execution_id, first.generation)
        .await
        .unwrap();
    let result = check(manager, &first.execution_id, first.generation).await;
    let second = manager
        .create(
            request(image, "foreign-owner-b"),
            &OperationId::new("hvf-foreign-owner-b").unwrap(),
        )
        .await
        .unwrap();
    manager
        .start(&second.execution_id, second.generation)
        .await
        .unwrap();
    let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
    let disks: Vec<_> = [&first, &second]
        .into_iter()
        .map(|reservation| {
            let record = store.get(&reservation.execution_id).unwrap().unwrap();
            let binding = record.managed_execution.unwrap().oci_runtime.unwrap();
            let share = root
                .join("service/runtime/shares")
                .join(binding.target.id.as_str())
                .join(binding.target.generation.unwrap().0.to_string());
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(share.join(".koci-agent-vm-attachments.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                manifest["target"],
                serde_json::to_value(&binding.target).unwrap()
            );
            let disk = share.with_extension("rootfs.ext4");
            assert!(disk.is_file());
            assert!(!disk.starts_with(&share));
            assert!(!share.join("bundle/rootfs.ext4").exists());
            (disk, manifest["rootfsImage"].clone())
        })
        .collect();
    assert_ne!(disks[0].1["sourceIdentity"], disks[1].1["sourceIdentity"]);
    assert_eq!(
        disks[0].1["image"]["imageDigest"],
        disks[1].1["image"]["imageDigest"]
    );
    let isolated = manager.execute(&second.execution_id,second.generation,exec(&["/bin/sh","-c","set -eu; test ! -e /ownership/created; test \"$(cat /ownership/secret)\" = seed; printf isolated > /ownership/secret"])).await.unwrap();
    assert_eq!(
        isolated.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&isolated.stderr)
    );
    let original = manager
        .execute(
            &first.execution_id,
            first.generation,
            exec(&["/bin/cat", "/ownership/secret"]),
        )
        .await
        .unwrap();
    assert_eq!(original.stdout, b"seed\nchanged");
    manager
        .kill_with_options(
            &first.execution_id,
            first.generation,
            kbox_core::KillExecutionOptions {
                signal: Some(libc::SIGTERM),
                timeout_secs: Some(3),
            },
        )
        .await
        .unwrap();
    let restarted = manager
        .restart(
            &first.execution_id,
            first.generation,
            &OperationId::new("hvf-foreign-owner-restart").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restarted.generation.get(), first.generation.get() + 1);
    assert!(manager
        .execute(
            &first.execution_id,
            first.generation,
            exec(&["/bin/touch", "/ownership/stale"])
        )
        .await
        .is_err());
    let after = check(manager, &first.execution_id, restarted.generation).await;
    manager
        .kill(&first.execution_id, restarted.generation)
        .await
        .unwrap();
    manager
        .remove(&first.execution_id, restarted.generation)
        .await
        .unwrap();
    manager
        .kill(&second.execution_id, second.generation)
        .await
        .unwrap();
    manager
        .remove(&second.execution_id, second.generation)
        .await
        .unwrap();
    fs::write(root.join("foreign-ownership.json"),serde_json::to_vec_pretty(&json!({"initial":result,"restart":after,"independentWritableInstances":true,"oldGenerationRejected":true,"image":image,"disks":disks})).unwrap()).unwrap();
}
