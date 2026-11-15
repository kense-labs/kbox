use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kbox_contract::contract::{
    RuntimeApplyRequest, RuntimeExecRequest, RuntimeInspection, RuntimeMount, RuntimeMountSource,
    RuntimeUnitClass,
};
use kbox_contract::{FileRuntimeStateStore, ManagedRuntimeClient, RuntimeClient};
use sha2::{Digest, Sha256};

use super::*;
use crate::kbox_contract_driver::test_support::{action, apply_request, runtime_spec};
use crate::{BoxRuntimeDriver, BoxRuntimeDriverConfig};

struct Grants {
    units: BTreeSet<String>,
    directories: BTreeMap<String, PathBuf>,
}

#[async_trait]
impl BoxDirectoryPort for Grants {
    async fn mount_path(
        &self,
        spec: &RuntimeUnitSpec,
        mount: &RuntimeMount,
    ) -> Result<PathBuf, BoxDirectoryPortError> {
        if !self.units.contains(&spec.unit_id) || ![1, 2].contains(&spec.generation) {
            return Err(BoxDirectoryPortError::Rejected(
                "ungranted execution".into(),
            ));
        }
        let RuntimeMountSource::Directory { reference } = &mount.source else {
            return Err(BoxDirectoryPortError::Rejected("wrong mount kind".into()));
        };
        self.directories
            .get(reference)
            .cloned()
            .ok_or_else(|| BoxDirectoryPortError::Rejected("ungranted directory".into()))
    }
    async fn cleanup_spec(&self, _digest: &str) -> Result<(), BoxDirectoryPortError> {
        Ok(())
    }
}

#[test]
#[ignore = "requires owned K_HOME, signed native assets and pinned Python OCI image; no host Harness"]
fn native_directory_grants_preserve_live_files_fences_and_cleanup() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(run());
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn run() {
    use std::os::unix::fs::PermissionsExt;
    let home = PathBuf::from(std::env::var_os("K_HOME").expect("owned home"));
    assert_eq!(home.canonicalize().unwrap(), home);
    assert_eq!(home.metadata().unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(
        std::fs::read_to_string(home.join(".kbox-directory-native")).unwrap(),
        "owned-directory-native-test\n"
    );
    let root = home.parent().unwrap();
    let workspace = root.join("原生 workspace with spaces");
    let external = root.join("外部 external with spaces");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&external).unwrap();
    let canary = root.join("unmounted canary.txt");
    std::fs::write(&canary, b"private-canary").unwrap();
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    std::fs::write(workspace.join("输入 file.txt"), nonce.as_bytes()).unwrap();
    let id = format!("directory-{nonce}");
    let readonly_id = format!("readonly-{nonce}");
    let grants = Arc::new(Grants {
        units: BTreeSet::from([id.clone(), readonly_id.clone()]),
        directories: BTreeMap::from([
            ("grant://workspace".into(), workspace.clone()),
            ("grant://external".into(), external.clone()),
        ]),
    });
    let driver = Arc::new(
        BoxRuntimeDriver::new(BoxRuntimeDriverConfig {
            home_dir: home.clone(),
            control_timeout: Duration::from_secs(60),
            ..Default::default()
        })
        .unwrap()
        .with_directory_port(grants),
    );
    let client = ManagedRuntimeClient::new(
        Arc::new(FileRuntimeStateStore::new(home.join("directory-runtime"))),
        driver.clone(),
    );
    let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").expect("pinned OCI fixture");
    let mut spec = runtime_spec(&id, 1, RuntimeUnitClass::Service);
    spec.artifact.uri = format!("oci://{image}");
    spec.artifact.digest = image.rsplit_once('@').unwrap().1.into();
    spec.semantics_profile_digest =
        Some(crate::kbox_contract_driver::NATIVE_GUEST_PROFILE_DIGEST.into());
    spec.process.command = vec!["/bin/sleep".into()];
    spec.process.args = vec!["2147483647".into()];
    spec.resources.memory_bytes = 512 * 1024 * 1024;
    spec.resources.pids = u32::MAX;
    spec.mounts = vec![
        RuntimeMount {
            name: "workspace".into(),
            source: RuntimeMountSource::Directory {
                reference: "grant://workspace".into(),
            },
            target: workspace.to_str().unwrap().into(),
            read_only: false,
        },
        RuntimeMount {
            name: "external".into(),
            source: RuntimeMountSource::Directory {
                reference: "grant://external".into(),
            },
            target: "/external with spaces".into(),
            read_only: false,
        },
    ];
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let mut ro = spec.clone();
        ro.unit_id = readonly_id;
        ro.mounts[0].read_only = true;
        client.apply(&apply_request(&ro)).await.unwrap();
        let probe = r#"import ctypes,errno,os,pathlib,subprocess,sys
p=pathlib.Path(sys.argv[1]); original=(p/'输入 file.txt').read_text()
libc=ctypes.CDLL(None,use_errno=True)
assert libc.mount(None,os.fsencode(p),None,32,None)==0,ctypes.get_errno()
try: (p/'输入 file.txt').write_text('forbidden')
except OSError as e: assert e.errno in (errno.EPERM,errno.EACCES),e
else: raise AssertionError('privileged guest bypassed host readonly')
assert (p/'输入 file.txt').read_text()==original
print('host-readonly-after-remount')"#;
        let result = client
            .exec(&RuntimeExecRequest {
                schema: RuntimeExecRequest::SCHEMA.into(),
                request_id: "directory-readonly-remount".into(),
                unit_id: ro.unit_id.clone(),
                generation: 1,
                command: vec![
                    "python3".into(),
                    "-c".into(),
                    probe.into(),
                    workspace.to_str().unwrap().into(),
                ],
                timeout_ms: 10_000,
                deadline_at_ms: None,
            })
            .await
            .unwrap();
        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        assert_eq!(result.stdout.trim(), "host-readonly-after-remount");
        client
            .stop(&action("directory-readonly-stop", &ro))
            .await
            .unwrap();
        client
            .remove(&action("directory-readonly-remove", &ro))
            .await
            .unwrap();
        assert!(driver.manager.managed_records().await.unwrap().is_empty());
    }
    let running = client
        .apply(&RuntimeApplyRequest {
            schema: RuntimeApplyRequest::SCHEMA.into(),
            request_id: "directory-up-1".into(),
            deadline_at_ms: None,
            spec: spec.clone(),
            restore_from_snapshot_operation_id: None,
        })
        .await
        .unwrap();
    let first_id = running.provider_resource_id.unwrap();
    let program="import hashlib,json,pathlib,subprocess,sys; w=pathlib.Path(sys.argv[1]); assert pathlib.Path.cwd()==w; assert (w/'输入 file.txt').read_text()==sys.argv[2]; assert not pathlib.Path(sys.argv[3]).exists(); data=bytes(range(256))*65536; p=pathlib.Path('/external with spaces/大文件.bin'); p.write_bytes(data); p.rename('/external with spaces/renamed file.bin'); subprocess.run([sys.executable,'-c',\"from pathlib import Path; Path('child output.txt').write_text('native-child')\"],check=True); print(json.dumps({'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}))";
    let result = client
        .exec(&RuntimeExecRequest {
            schema: RuntimeExecRequest::SCHEMA.into(),
            request_id: "directory-exec-1".into(),
            unit_id: spec.unit_id.clone(),
            generation: 1,
            command: vec![
                "python3".into(),
                "-c".into(),
                format!(
                    "import os; os.chdir({:?}); {program}",
                    workspace.to_str().unwrap()
                ),
                workspace.to_str().unwrap().into(),
                nonce.clone(),
                canary.to_str().unwrap().into(),
            ],
            timeout_ms: 30_000,
            deadline_at_ms: None,
        })
        .await
        .unwrap();
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert!(!result.truncated);
    let proof: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    let bytes = std::fs::read(external.join("renamed file.bin")).unwrap();
    assert_eq!(proof["bytes"], 16 * 1024 * 1024);
    assert_eq!(proof["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(
        std::fs::read(workspace.join("child output.txt")).unwrap(),
        b"native-child"
    );
    std::fs::write(workspace.join("host change.txt"), b"live-host-change").unwrap();
    let live = client
        .exec(&RuntimeExecRequest {
            schema: RuntimeExecRequest::SCHEMA.into(),
            request_id: "directory-live-change".into(),
            unit_id: spec.unit_id.clone(),
            generation: 1,
            command: vec![
                "cat".into(),
                workspace.join("host change.txt").to_str().unwrap().into(),
            ],
            timeout_ms: 5_000,
            deadline_at_ms: None,
        })
        .await
        .unwrap();
    assert_eq!(live.exit_code, 0);
    assert_eq!(live.stdout, "live-host-change");
    client
        .stop(&action("directory-stop-1", &spec))
        .await
        .unwrap();
    client
        .remove(&action("directory-remove-1", &spec))
        .await
        .unwrap();
    let mut second = spec.clone();
    second.generation = 2;
    let next = client
        .apply(&RuntimeApplyRequest {
            schema: RuntimeApplyRequest::SCHEMA.into(),
            request_id: "directory-up-2".into(),
            deadline_at_ms: None,
            spec: second.clone(),
            restore_from_snapshot_operation_id: None,
        })
        .await
        .unwrap();
    assert_ne!(
        next.provider_resource_id.as_deref(),
        Some(first_id.as_str())
    );
    let stale = client
        .exec(&RuntimeExecRequest {
            schema: RuntimeExecRequest::SCHEMA.into(),
            request_id: "directory-late-old-exec".into(),
            unit_id: spec.unit_id.clone(),
            generation: 1,
            command: vec![
                "touch".into(),
                workspace
                    .join("forbidden late write")
                    .to_str()
                    .unwrap()
                    .into(),
            ],
            timeout_ms: 5_000,
            deadline_at_ms: None,
        })
        .await;
    assert!(matches!(
        stale,
        Err(kbox_contract::RuntimeError::StaleGeneration { .. })
    ));
    assert!(!workspace.join("forbidden late write").exists());
    let moved = workspace.with_file_name("moved workspace");
    std::fs::rename(&workspace, &moved).unwrap();
    client
        .stop(&action("directory-stop-2", &second))
        .await
        .unwrap();
    client
        .remove(&action("directory-remove-2", &second))
        .await
        .unwrap();
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    assert!(matches!(
        client.inspect(&spec.unit_id).await.unwrap(),
        RuntimeInspection::NotFound { .. }
    ));
    assert_eq!(std::fs::read(&canary).unwrap(), b"private-canary");
    assert_eq!(
        std::fs::read(moved.join("输入 file.txt")).unwrap(),
        nonce.as_bytes()
    );
    assert!(external.join("renamed file.bin").is_file());
    std::fs::write(root.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","platform":std::env::consts::OS,"image":image,
        "firstExecutionId":first_id,"secondExecutionId":next.provider_resource_id,"fileProbe":proof,"nativeWorkspacePath":true,"liveHostChanges":true,
        "unmountedCanaryPreserved":true,"staleGenerationRefused":true,"movedSourceCleanup":true,"zeroPrivateExecutions":true})).unwrap()).unwrap();
}
