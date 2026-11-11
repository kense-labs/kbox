use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::FutureExt;
use kbox_contract::contract::{
    RuntimeApplyRequest, RuntimeExecRequest, RuntimeMount, RuntimeMountSource, RuntimeUnitClass,
    RuntimeUnitSpec,
};
use kbox_contract::{FileRuntimeStateStore, ManagedRuntimeClient, RuntimeClient};
use kbox_core::{ExecRequest, ExecutionManager, ExecutionSessionManager};
use sha2::{Digest, Sha256};

use super::test_support::{action, runtime_spec};
use super::{
    BoxDirectoryPort, BoxDirectoryPortError, BoxRuntimeDriver, BoxRuntimeDriverConfig,
    NATIVE_GUEST_PROFILE_DIGEST,
};

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
                "ungranted prewarm test scope".into(),
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
    async fn cleanup_spec(&self, _: &str) -> Result<(), BoxDirectoryPortError> {
        Ok(())
    }
}

#[test]
#[ignore = "requires an owned native home, signed KBox assets and pinned Node OCI base"]
fn native_prewarm_handoff_preserves_vm_identity_and_retires_dirty_resources() {
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
        std::fs::read_to_string(home.join(".krun-prewarm-native")).unwrap(),
        "owned-prewarm-native-test\n"
    );
    let root = home.parent().unwrap();
    let workspace = root.join("预热 workspace with spaces");
    let external = root.join("外部 external with spaces");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&external).unwrap();
    let canary = root.join("unmounted canary.txt");
    std::fs::write(&canary, b"not granted").unwrap();
    let bytes = (0..2 * 1024 * 1024)
        .map(|i| (i % 251) as u8)
        .collect::<Vec<_>>();
    std::fs::write(workspace.join("输入 data.bin"), &bytes).unwrap();
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    let grants = Arc::new(Grants {
        units: BTreeSet::from(["prewarm-template".into(), "claimed-environment".into()]),
        directories: BTreeMap::from([
            ("workspace".into(), workspace.clone()),
            ("external".into(), external.clone()),
        ]),
    });
    let config = BoxRuntimeDriverConfig {
        home_dir: home.clone(),
        control_timeout: Duration::from_secs(90),
        ..Default::default()
    };
    let base = BoxRuntimeDriver::new(config.clone())
        .unwrap()
        .with_directory_port(grants.clone());
    let scope = format!(
        "native:{:x}",
        Sha256::digest(base.provider_build().await.unwrap().as_bytes())
    );
    let driver = Arc::new(base.with_environment_prewarm(scope.clone()).unwrap());
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    let mut spec = runtime_spec("prewarm-template", 1, RuntimeUnitClass::Service);
    let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").expect("pinned Node base");
    spec.artifact.uri = format!("oci://{image}");
    spec.artifact.digest = image.rsplit_once('@').unwrap().1.into();
    spec.semantics_profile_digest = Some(NATIVE_GUEST_PROFILE_DIGEST.into());
    spec.process.command = vec!["/bin/sleep".into()];
    spec.process.args = vec!["infinity".into()];
    spec.process.working_directory = Some("/".into());
    spec.restart = kbox_contract::contract::RestartPolicy::Never;
    spec.resources.memory_bytes = 512 * 1024 * 1024;
    spec.resources.pids = u32::MAX;
    spec.mounts = vec![
        RuntimeMount {
            name: "workspace".into(),
            source: RuntimeMountSource::Directory {
                reference: "workspace".into(),
            },
            target: workspace.to_str().unwrap().into(),
            read_only: false,
        },
        RuntimeMount {
            name: "external".into(),
            source: RuntimeMountSource::Directory {
                reference: "external".into(),
            },
            target: "/external with spaces".into(),
            read_only: false,
        },
    ];
    let result=std::panic::AssertUnwindSafe(async {
        let started=Instant::now();
        let warm=driver.prepare_environment(&spec,"native-slot").await.unwrap();
        let cold_ms=started.elapsed().as_millis();
        let original=driver.manager.managed_record(&warm.execution_id).await.unwrap().unwrap();
        assert!(original.pid.is_some() && original.pid_start_time.is_some());
        let blocked=driver.manager.execute(&warm.execution_id,warm.generation,exec(vec!["touch".into(),"/tmp/should-not-run".into()])).await;
        assert!(matches!(blocked,Err(kbox_core::ExecutionManagerError::Conflict{..})));
        let recovered=Arc::new(BoxRuntimeDriver::new(config).unwrap().with_directory_port(grants).with_environment_prewarm(scope).unwrap());
        let same=recovered.prepare_environment(&spec,"native-slot").await.unwrap();
        assert_eq!((same.execution_id.clone(),same.generation),(warm.execution_id.clone(),warm.generation));
        let client=ManagedRuntimeClient::new(Arc::new(FileRuntimeStateStore::new(home.join("prewarm-runtime"))),recovered.clone());
        let mut active=spec.clone();active.unit_id="claimed-environment".into();
        let started=Instant::now();
        let running=client.apply(&apply(&active,"claim-one")).await.unwrap();
        let claim_ms=started.elapsed().as_millis();
        assert_eq!(running.provider_resource_id.as_deref(),Some(warm.execution_id.as_str()));
        let claimed=recovered.manager.managed_record(&warm.execution_id).await.unwrap().unwrap();
        let generation=claimed.managed_execution.as_ref().unwrap().generation;
        assert_eq!(generation.get(),warm.generation.get()+1);
        assert_eq!((claimed.pid,claimed.pid_start_time,claimed.started_at),(original.pid,original.pid_start_time,original.started_at));
        assert!(recovered.manager.kill(&warm.execution_id,warm.generation).await.is_err());
        recovered.retire_unclaimed_environments("native-slot").await.unwrap();
        assert_eq!(recovered.manager.inspect(&warm.execution_id).await.unwrap().generation,generation);
        let program="const fs=require('fs'),cp=require('child_process'),crypto=require('crypto'); const w=process.argv[1]; if(fs.existsSync(process.argv[2])||fs.existsSync('/tmp/should-not-run'))process.exit(3); const data=fs.readFileSync(w+'/输入 data.bin'); fs.writeFileSync('/external with spaces/output.bin',data); fs.writeFileSync('/tmp/user-dirty','dirty'); const p=cp.spawnSync(process.execPath,['-e',\"require('fs').writeFileSync('/external with spaces/child.txt','native-child')\"]); if(p.status!==0)process.exit(4); console.log(crypto.createHash('sha256').update(data).digest('hex'));";
        let output=client.exec(&RuntimeExecRequest {schema:RuntimeExecRequest::SCHEMA.into(),request_id:"warm-native-tools".into(),unit_id:active.unit_id.clone(),generation:active.generation,command:vec!["node".into(),"-e".into(),program.into(),workspace.to_str().unwrap().into(),canary.to_str().unwrap().into()],timeout_ms:30_000,deadline_at_ms:None}).await.unwrap();
        assert_eq!(output.exit_code,0,"{}",output.stderr);assert_eq!(output.stdout.trim(),checksum);
        assert_eq!(std::fs::read(external.join("output.bin")).unwrap(),bytes);
        assert_eq!(std::fs::read_to_string(external.join("child.txt")).unwrap(),"native-child");
        let next=recovered.prepare_environment(&active,"native-slot").await.unwrap();
        assert_ne!(next.execution_id,warm.execution_id);
        client.stop(&action("retire-dirty", &active)).await.unwrap();client.remove(&action("remove-dirty", &active)).await.unwrap();
        active.generation=2;
        let running=client.apply(&apply(&active,"claim-two")).await.unwrap();
        assert_eq!(running.provider_resource_id.as_deref(),Some(next.execution_id.as_str()));
        let clean=client.exec(&RuntimeExecRequest {schema:RuntimeExecRequest::SCHEMA.into(),request_id:"warm-native-clean".into(),unit_id:active.unit_id.clone(),generation:active.generation,command:vec!["node".into(),"-e".into(),"if(require('fs').existsSync('/tmp/user-dirty'))process.exit(5);console.log('clean')".into()],timeout_ms:30_000,deadline_at_ms:None}).await.unwrap();
        assert_eq!(clean.exit_code,0);assert_eq!(clean.stdout.trim(),"clean");
        client.stop(&action("stop-clean", &active)).await.unwrap();client.remove(&action("remove-clean", &active)).await.unwrap();
        recovered.retire_unclaimed_environments("native-slot").await.unwrap();
        serde_json::json!({"status":"PASS","coldPrepareMs":cold_ms,"warmClaimMs":claim_ms,"firstExecutionId":warm.execution_id.as_str(),"claimedGeneration":generation.get(),"sameOriginalProcess":true,"sourceGenerationRefused":true,"reconstructedDriverReusedReservation":true,"differentRuntimeUnitClaimedCompatibleBase":true,"unclaimedExecRefused":true,"externalFileBytes":bytes.len(),"externalFileSha256":checksum,"childProcess":true,"replacementExecutionId":next.execution_id.as_str(),"dirtyVmNotReused":true})
    }).catch_unwind().await;
    for record in driver.manager.managed_records().await.unwrap() {
        let id = kbox_core::ExecutionId::new(record.id).unwrap();
        let generation = record.managed_execution.unwrap().generation;
        let _ = driver.manager.kill(&id, generation).await;
        driver.manager.remove(&id, generation).await.unwrap();
    }
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    match result {
        Ok(mut report) => {
            report["zeroPrivateExecutions"] = true.into();
            std::fs::write(
                root.join("result.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        Err(error) => std::panic::resume_unwind(error),
    }
}

fn exec(cmd: Vec<String>) -> ExecRequest {
    ExecRequest {
        request_id: None,
        cmd,
        timeout_ns: 30_000_000_000,
        env: vec![],
        working_dir: None,
        rootfs: None,
        stdin: None,
        stdin_streaming: false,
        user: None,
        streaming: false,
    }
}
fn apply(spec: &RuntimeUnitSpec, id: &str) -> RuntimeApplyRequest {
    RuntimeApplyRequest {
        schema: RuntimeApplyRequest::SCHEMA.into(),
        request_id: id.into(),
        deadline_at_ms: None,
        spec: spec.clone(),
        restore_from_snapshot_operation_id: None,
    }
}
