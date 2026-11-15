//! Owned real Box -> SDK -> independent Host Service -> HVF qualification.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures::FutureExt;
use kbox_core::{
    BoxConfig, CreateExecutionRequest, ExecRequest, ExecutionManager, ExecutionSessionManager,
    ExecutionState, NetworkMode, OperationId, ResourceConfig,
};
use serde_json::json;

use super::*;
use crate::{ManagedExecutionStore, ManagedRuntimeRoute};

#[path = "oci_hvf_native_behavior.rs"]
mod behavior;

#[path = "oci_hvf_native_faults.rs"]
mod faults;

#[path = "oci_hvf_native_cli.rs"]
mod cli;

#[path = "oci_hvf_native_rootfs.rs"]
mod ownership;

#[path = "oci_hvf_native_files.rs"]
mod files;

#[path = "oci_hvf_native_lifetime.rs"]
mod lifetime;

#[path = "oci_hvf_native_pending.rs"]
mod pending;

#[path = "oci_hvf_native_failed_create.rs"]
mod failed_create;

#[path = "oci_hvf_native_rollback.rs"]
mod rollback;

#[path = "oci_hvf_native_named.rs"]
mod named;

#[path = "oci_hvf_native_anonymous.rs"]
mod anonymous;

#[path = "oci_hvf_native_raw_ro.rs"]
mod raw_ro;

#[path = "oci_hvf_native_guest_readonly.rs"]
mod guest_readonly;

#[path = "oci_hvf_native_path_guards.rs"]
mod path_guards;

#[path = "oci_hvf_native_tmpfs.rs"]
mod tmpfs;

#[path = "oci_hvf_native_shm.rs"]
mod shm;

#[path = "oci_hvf_native_root_readonly.rs"]
mod root_readonly;

#[path = "oci_hvf_native_ulimits.rs"]
mod ulimits;

#[path = "oci_hvf_native_cpu_time.rs"]
mod cpu_time;

#[path = "oci_hvf_native_memory_limits.rs"]
mod memory_limits;

#[path = "oci_hvf_native_nproc.rs"]
mod nproc;

#[path = "oci_hvf_native_sigpending.rs"]
mod sigpending;

#[path = "oci_hvf_native_msgqueue.rs"]
mod msgqueue;

#[path = "oci_hvf_native_nice.rs"]
mod nice;

#[path = "oci_hvf_native_realtime.rs"]
mod realtime;

#[path = "oci_hvf_native_disk_capacity.rs"]
mod disk_capacity;

#[path = "oci_hvf_native_capabilities.rs"]
mod capabilities;

#[path = "oci_hvf_native_raw_secrets.rs"]
mod raw_secrets;
#[path = "oci_hvf_native_secret_environment.rs"]
mod secret_environment;
#[path = "oci_hvf_native_secrets.rs"]
mod secrets;
#[path = "oci_hvf_native_vm_sizing.rs"]
mod vm_sizing;

struct Host {
    child: Option<Child>,
}

impl Host {
    fn start(root: &Path, wave: u32) -> Self {
        let shim = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_SHIM").expect("fixed OCI shim"));
        Self::start_with_shim(root, wave, &shim)
    }

    fn start_with_shim(root: &Path, wave: u32, shim: &Path) -> Self {
        let image = PathBuf::from(
            std::env::var_os("K_BOX_TEST_HVF_MANIFEST").expect("fixed system-image manifest"),
        );
        Self::start_with_assets(root, wave, shim, &image)
    }

    fn start_with_assets(root: &Path, mut wave: u32, shim: &Path, image: &Path) -> Self {
        while root.join(format!("host-{wave}.pid")).exists() {
            wave += 1;
        }
        let binary = std::env::var_os("K_BOX_TEST_HVF_HOST").expect("fixed Host binary");
        let child = Command::new(binary)
            .args(["macos-hvf-host-service", "--root"])
            .arg(root.join("service"))
            .arg("--shim")
            .arg(shim)
            .arg("--system-image-manifest")
            .arg(image)
            .stdin(Stdio::null())
            .stdout(File::create(root.join(format!("host-{wave}.stdout"))).unwrap())
            .stderr(File::create(root.join(format!("host-{wave}.stderr"))).unwrap())
            .spawn()
            .unwrap();
        fs::write(
            root.join(format!("host-{wave}.pid")),
            child.id().to_string(),
        )
        .unwrap();
        Self { child: Some(child) }
    }
    fn sigkill(&mut self) -> u32 {
        use std::os::unix::process::ExitStatusExt;
        let mut child = self.child.take().unwrap();
        let pid = child.id();
        child.kill().unwrap();
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGKILL));
        pid
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let Some(child) = &mut self.child else { return };
        if child.try_wait().unwrap().is_none() {
            // SAFETY: this unreaped child is owned by this guard, so its PID cannot be reused.
            assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
        }
        assert!(
            child.wait().unwrap().success(),
            "Host graceful shutdown failed"
        );
    }
}

async fn connect(root: &Path, policy: OciMigrationPolicy) -> LocalExecutionManager {
    connect_with_home(root, &root.join("home"), policy).await
}

async fn connect_with_home(
    root: &Path,
    home: &Path,
    policy: OciMigrationPolicy,
) -> LocalExecutionManager {
    let config = MacosHvfOciMigrationConfig::new(
        root.join("service/runtime"),
        root.join("service/runtime.sock"),
    )
    .unwrap()
    .with_creation_policy(policy);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        match LocalExecutionManager::with_macos_hvf_oci_qualification(
            home.join("boxes.json"),
            home,
            config.clone(),
        )
        .await
        {
            Ok(manager) => return manager,
            Err(error) if tokio::time::Instant::now() < deadline => {
                eprintln!("waiting for owned Host: {error}");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(error) => panic!("Host did not become ready: {error}"),
        }
    }
}

fn request(image: &str, label: &str) -> CreateExecutionRequest {
    CreateExecutionRequest {
        external_sandbox_id: label.into(),
        config: BoxConfig {
            image: image.into(),
            network: NetworkMode::None,
            resources: ResourceConfig {
                vcpus: 1,
                memory_mb: 512,
                ..Default::default()
            },
            cmd: vec!["/bin/sleep".into(), "2147483647".into()],
            ..Default::default()
        },
        labels: BTreeMap::new(),
        policy: Default::default(),
        rootfs_snapshot_id: None,
    }
}

fn exec(command: &[&str]) -> ExecRequest {
    ExecRequest {
        cmd: command.iter().map(|s| (*s).into()).collect(),
        timeout_ns: 10_000_000_000,
        request_id: None,
        env: vec![],
        working_dir: None,
        rootfs: None,
        stdin: None,
        stdin_streaming: false,
        user: None,
        streaming: false,
    }
}

#[test]
#[ignore = "requires sentinel-owned private home and fixed signed independent HVF Host assets"]
fn native_box_hvf_host_service_preserves_routes_and_recovery() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").expect("owned test root"));
    assert_eq!(root.canonicalize().unwrap(), root);
    assert_eq!(
        fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),
        "owned-hvf-native-test\n"
    );
    let mut host = Host::start(&root, 1);
    tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8 * 1024 * 1024).enable_all().build().unwrap().block_on(async {
        let result = std::panic::AssertUnwindSafe(async {
            let image = std::env::var("K_BOX_TEST_STREAM_IMAGE").expect("pinned OCI image");
            let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
            let foreign = std::env::var("K_BOX_TEST_HVF_FOREIGN_UID_IMAGE").expect("pinned foreign ownership image");
            ownership::exercise(&manager, &root, &foreign).await;
            files::exercise(&manager, &root, &image).await;
            let operation = OperationId::new("hvf-native-oci-create").unwrap();
            let mut req = request(&image, "oci");
            behavior::configure(&root, &mut req);
            files::configure(&root, "main", &mut req);
            let reservation = manager.create(req.clone(), &operation).await.unwrap();
            let replay = manager.create(req, &operation).await.unwrap();
            assert_eq!((reservation.execution_id.clone(),reservation.generation),(replay.execution_id,replay.generation));
            let store = ManagedExecutionStore::new(root.join("home/boxes.json"));
            let created = store.get(&reservation.execution_id).unwrap().unwrap();
            assert_eq!(created.managed_execution.as_ref().unwrap().runtime_route, ManagedRuntimeRoute::OciSdk);
            manager.start(&reservation.execution_id, reservation.generation).await.unwrap();
            let running = store.get(&reservation.execution_id).unwrap().unwrap();
            let binding = running.managed_execution.as_ref().unwrap().oci_runtime.clone().unwrap();
            assert_eq!(binding.driver, koci_sdk::DriverKind::LibkHvf);
            assert_eq!(binding.isolation, koci_sdk::IsolationClass::DedicatedVm);
            binding.validate_for(&reservation.execution_id).unwrap();
            assert!(running.pid.is_none() && running.pid_start_time.is_none());
            let output = manager.execute(&reservation.execution_id,reservation.generation,exec(&["/bin/sh","-c","printf independent-host; printf native-stderr >&2; exit 23"])).await.unwrap();
            assert_eq!(output.exit_code,23);assert_eq!(output.stdout,b"independent-host");assert_eq!(output.stderr,b"native-stderr");
            behavior::exercise(&manager, &root, &reservation.execution_id, reservation.generation).await;
            files::check(&manager, &root, "main", &reservation.execution_id, reservation.generation).await;
            let mut dropped_request = request(&image,"dropped-capabilities");
            dropped_request.config.cap_add = vec!["CAP_SYS_ADMIN".into()];
            dropped_request.config.cap_drop = vec![" cap_all ".into()];
            let dropped = manager.create_and_start(dropped_request,&OperationId::new("hvf-drop-all-create").unwrap()).await.unwrap();
            let status = manager.execute(&dropped.execution_id,dropped.generation,exec(&["/bin/cat","/proc/self/status"])).await.unwrap();
            assert_eq!(status.exit_code,0);
            let status_text = String::from_utf8(status.stdout).unwrap();
            let masks: Vec<_> = status_text.lines().filter(|line| ["CapInh:","CapPrm:","CapEff:","CapBnd:","CapAmb:"].iter().any(|prefix|line.starts_with(prefix))).collect();
            assert_eq!(masks.len(),5);
            for mask in &masks {assert_eq!(u64::from_str_radix(mask.split_whitespace().nth(1).unwrap(),16).unwrap(),0,"{mask}");}
            fs::write(root.join("cap-drop-all.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","spelling":" cap_all ","kernelMasks":masks})).unwrap()).unwrap();
            manager.kill(&dropped.execution_id,dropped.generation).await.unwrap();
            manager.remove(&dropped.execution_id,dropped.generation).await.unwrap();
            drop(manager);
            let changed = connect(&root,OciMigrationPolicy::LegacyOnly).await;
            assert_eq!(changed.inspect(&reservation.execution_id).await.unwrap().state,ExecutionState::Running);
            let legacy = changed.create(request(&image,"legacy"), &OperationId::new("hvf-native-legacy-create").unwrap()).await.unwrap();
            changed.start(&legacy.execution_id,legacy.generation).await.unwrap();
            let record = store.get(&legacy.execution_id).unwrap().unwrap();
            assert_eq!(record.managed_execution.as_ref().unwrap().runtime_route,ManagedRuntimeRoute::BoxVm);
            assert!(record.pid.is_some());
            let manager = connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
            assert_eq!(manager.inspect(&legacy.execution_id).await.unwrap().state,ExecutionState::Running);
            assert_eq!(manager.inspect(&reservation.execution_id).await.unwrap().state,ExecutionState::Running);
            assert_eq!(store.get(&reservation.execution_id).unwrap().unwrap().managed_execution.unwrap().oci_runtime.unwrap(),binding);
            let cli_checked = cli::exercise(&root,&image,&reservation.execution_id,&legacy.execution_id);
            faults::exercise(&root, &mut host, &reservation.execution_id, reservation.generation, &legacy.execution_id, legacy.generation).await;
            fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","schema":"kbox.hvf-oci-qualification.v1","ociBinding":binding,"ociId":reservation.execution_id,"legacyId":legacy.execution_id,"mixedRoutes":true,"createReplay":true,"reconstructedManager":true,"policyChangePreservesRoutes":true,"execIo":true,"cliChecked":cli_checked})).unwrap()).unwrap();
        }).catch_unwind().await;
        if host.child.is_none() { host = Host::start(&root,99); }
        if host.child.as_mut().unwrap().try_wait().unwrap().is_some() { panic!("Host died unexpectedly; inspect retained logs"); }
        cleanup(&root).await;
        if let Err(error) = result { std::panic::resume_unwind(error); }
    });
    drop(host);
    assert!(!root.join("service/runtime.sock").exists());
}

async fn cleanup(root: &Path) {
    cleanup_with_home(root, &root.join("home")).await;
}

async fn cleanup_with_home(root: &Path, home: &Path) {
    let manager = connect_with_home(root, home, OciMigrationPolicy::MicrovmViaOci).await;
    let store = ManagedExecutionStore::new(home.join("boxes.json"));
    fs::write(
        root.join("cleanup-records-before.json"),
        serde_json::to_vec_pretty(&store.list().unwrap()).unwrap(),
    )
    .unwrap();
    for record in store.list().unwrap() {
        if let Some(metadata) = record.managed_execution {
            let id = kbox_core::ExecutionId::new(record.id).unwrap();
            let _ = manager.reconcile(&metadata.operation_id).await;
            let Some(current) = store.get(&id).unwrap() else {
                continue;
            };
            let generation = current.managed_execution.unwrap().generation;
            let _ = manager.kill(&id, generation).await;
            manager.remove(&id, generation).await.unwrap();
        }
    }
    assert!(store.list().unwrap().is_empty());
    fs::write(root.join("cleanup-boxes.json"), b"[]\n").unwrap();
}

#[test]
#[ignore = "maintenance only: requires the existing sentinel-owned HVF test root"]
fn native_cleanup_owned_hvf_test_root() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
            assert_eq!(root.canonicalize().unwrap(), root);
            assert_eq!(
                fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),
                "owned-hvf-native-test\n"
            );
            let host = Host::start(&root, 99);
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(cleanup(&root));
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
#[ignore = "requires sentinel-owned root and fixed signed native HVF assets"]
fn native_box_hvf_single_file_host_pep() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
            assert_eq!(root.canonicalize().unwrap(), root);
            assert_eq!(
                fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),
                "owned-hvf-native-test\n"
            );
            let host = Host::start(&root, 1);
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let result = std::panic::AssertUnwindSafe(async {
                        let manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                        let mut req = request(
                            &std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                            "file-pep",
                        );
                        files::configure(&root, "pep", &mut req);
                        let execution = manager
                            .create_and_start(req, &OperationId::new("file-pep-create").unwrap())
                            .await
                            .unwrap();
                        files::check(
                            &manager,
                            &root,
                            "pep",
                            &execution.execution_id,
                            execution.generation,
                        )
                        .await;
                        fs::write(
                            root.join("result.json"),
                            b"{\"status\":\"PASS\",\"filePEPOnly\":true}\n",
                        )
                        .unwrap();
                    })
                    .catch_unwind()
                    .await;
                    cleanup(&root).await;
                    if let Err(error) = result {
                        std::panic::resume_unwind(error);
                    }
                });
            drop(host);
            assert!(!root.join("service/runtime.sock").exists());
        })
        .unwrap()
        .join()
        .unwrap();
}
