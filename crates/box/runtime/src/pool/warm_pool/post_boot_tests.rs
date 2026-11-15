use super::*;
use crate::vm::BoxState;
use crate::vmm::{VmHandler, VmMetrics};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

struct Handler {
    stopped: Arc<AtomicBool>,
}

impl VmHandler for Handler {
    fn stop(&mut self, _signal: i32, _timeout_ms: u64) -> Result<()> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn metrics(&self) -> VmMetrics {
        VmMetrics::default()
    }

    fn is_running(&self) -> bool {
        !self.stopped.load(Ordering::SeqCst)
    }

    fn has_exited(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    fn pid(&self) -> u32 {
        42
    }
}

async fn booted() -> (tempfile::TempDir, VmManager, Arc<AtomicBool>) {
    #[cfg(unix)]
    let root = tempfile::tempdir_in("/tmp").unwrap();
    #[cfg(not(unix))]
    let root = tempfile::tempdir().unwrap();
    let mut vm = VmManager::with_box_id(
        BoxConfig::default(),
        EventEmitter::new(16),
        uuid::Uuid::new_v4().to_string(),
    );
    vm.home_dir = root.path().join("home");
    std::fs::create_dir_all(vm.home_dir.join("boxes").join(vm.box_id()).join("logs")).unwrap();
    std::fs::write(root.path().join("caller"), b"caller-owned").unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    *vm.handler.write().await = Some(Box::new(Handler {
        stopped: stopped.clone(),
    }));
    *vm.state.write().await = BoxState::Ready;
    (root, vm, stopped)
}

async fn assert_clean(root: &Path, vm: &VmManager, stopped: &AtomicBool) {
    assert!(
        stopped.load(Ordering::SeqCst),
        "post-boot refusal must stop its owned handler"
    );
    assert!(vm.handler.read().await.is_none());
    assert_eq!(vm.state().await, BoxState::Stopped);
    assert!(!vm.home_dir.join("boxes").join(vm.box_id()).exists());
    assert_eq!(std::fs::read(root.join("caller")).unwrap(), b"caller-owned");
}

#[tokio::test]
async fn missing_post_boot_exec_path_retires_the_owned_vm() {
    let (root, mut vm, stopped) = booted().await;
    let error = WarmPool::complete_cold_boot(&mut vm).await.unwrap_err();
    assert!(
        matches!(error, BoxError::ExecError(ref reason) if reason == "Exec socket path is unavailable")
    );
    assert_clean(root.path(), &vm, &stopped).await;
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn lost_post_boot_exec_channel_retires_the_owned_vm_after_refusal() {
    let (root, mut vm, stopped) = booted().await;
    vm.exec_socket_path = Some(root.path().join("vanished.sock"));
    let started = tokio::time::Instant::now();
    let error = WarmPool::complete_cold_boot(&mut vm).await.unwrap_err();
    assert!(matches!(error, BoxError::ExecError(_)));
    assert!(started.elapsed() >= Duration::from_secs(120));
    assert_clean(root.path(), &vm, &stopped).await;
}

#[cfg(unix)]
async fn authenticate_once(vm: &mut VmManager, root: &Path) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let path = root.join("exec.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    vm.exec_socket_path = Some(path);
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut frame = [0; 5];
            // connect() probes transport before heartbeat opens its own stream.
            if stream.read_exact(&mut frame).await.is_err() {
                continue;
            }
            assert_eq!(frame, ktransport::Frame::heartbeat().encode().unwrap()[..]);
            stream.write_all(&frame).await.unwrap();
            return;
        }
    })
}

#[cfg(unix)]
#[tokio::test]
async fn authenticated_post_boot_vm_remains_owned_and_ready() {
    let (root, mut vm, stopped) = booted().await;
    let server = authenticate_once(&mut vm, root.path()).await;
    WarmPool::complete_cold_boot(&mut vm).await.unwrap();
    server.await.unwrap();
    assert_eq!(vm.state().await, BoxState::Ready);
    assert!(vm.exec_client().is_some());
    assert!(vm.handler.read().await.is_some());
    assert!(!stopped.load(Ordering::SeqCst));
    vm.destroy_with_timeout(0).await.unwrap();
    assert_clean(root.path(), &vm, &stopped).await;
}

#[cfg(unix)]
#[tokio::test]
async fn process_prewarm_refusal_still_retires_the_owned_vm() {
    let (root, mut vm, stopped) = booted().await;
    vm.config.pool.prewarm = Some(kbox_core::config::PrewarmConfig {
        user: None,
        workdir: None,
        env: vec![],
    });
    let server = authenticate_once(&mut vm, root.path()).await;
    let error = WarmPool::complete_cold_boot(&mut vm).await.unwrap_err();
    server.await.unwrap();
    assert!(
        matches!(error, BoxError::PoolError(ref reason) if reason == "process prewarm requires the resolved OCI image config")
    );
    assert_clean(root.path(), &vm, &stopped).await;
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "requires sentinel-owned home, cached image and installed Box companions"]
fn native_lost_post_boot_channel_stops_real_vm_and_late_writes() {
    use futures::FutureExt;
    use std::path::PathBuf;

    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let home=PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(home.canonicalize().unwrap(),home);
        assert_eq!(std::fs::read_to_string(home.join(".kbox-pool-cleanup-native")).unwrap(),"owned-pool-cleanup-native-test\n");
        let root=home.parent().unwrap();
        let caller=root.join("caller");std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"),b"caller-owned-retained").unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let config=BoxConfig {
                image:std::env::var("K_BOX_TEST_STREAM_IMAGE").unwrap(),
                network:kbox_core::NetworkMode::None,
                cmd:vec!["/bin/sh".into(),"-c".into(),"while :; do printf x >> /owned/writes; /bin/sleep 0.1; done".into()],
                volumes:vec![format!("{}:/owned:rw",caller.display())],
                ..BoxConfig::default()
            };
            let mut vm=VmManager::new(config,EventEmitter::new(16));
            assert_eq!(vm.home_dir,home);
            let id=vm.box_id().to_owned();
            let result=std::panic::AssertUnwindSafe(async {
                vm.boot().await.unwrap();assert_eq!(vm.state().await,BoxState::Ready);
                assert!(vm.exec_client().is_some());
                let pid=vm.handler.read().await.as_ref().unwrap().pid();
                let deadline=tokio::time::Instant::now()+Duration::from_secs(5);
                while !caller.join("writes").is_file() {
                    assert!(tokio::time::Instant::now()<deadline,"actual workload must write before fault");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                let endpoint=vm.exec_socket_path.clone().unwrap();
                assert_eq!(endpoint,vm.socket_dir().join("exec.sock"));
                use std::os::unix::fs::FileTypeExt;
                assert!(std::fs::symlink_metadata(&endpoint).unwrap().file_type().is_socket());
                let before=serde_json::json!({"execution":id,"pid":pid,"endpoint":endpoint,"state":"ready","authenticated":true,"callerWriteBytes":std::fs::metadata(caller.join("writes")).unwrap().len()});
                std::fs::write(root.join("before-fault.json"),serde_json::to_vec_pretty(&before).unwrap()).unwrap();
                std::fs::remove_file(&endpoint).unwrap();
                let started=tokio::time::Instant::now();
                let error=WarmPool::complete_cold_boot(&mut vm).await.unwrap_err();
                let elapsed=started.elapsed();
                assert!(matches!(error,BoxError::ExecError(_)),"{error}");
                assert!(elapsed>=Duration::from_secs(120));
                assert!(elapsed<Duration::from_secs(135));
                assert_eq!(vm.state().await,BoxState::Stopped);
                assert!(vm.handler.read().await.is_none());
                assert!(!home.join("boxes").join(&id).exists());
                assert!(!vm.socket_dir().exists());
                let writes=std::fs::read(caller.join("writes")).unwrap();assert!(!writes.is_empty());
                tokio::time::sleep(Duration::from_millis(500)).await;
                assert_eq!(std::fs::read(caller.join("writes")).unwrap(),writes,"no writes after cleanup returned");
                assert_eq!(std::fs::read(caller.join("canary")).unwrap(),b"caller-owned-retained");
                let proof=serde_json::json!({"status":"PASS","execution":id,"pid":pid,"error":error.to_string(),"postBootCheckSeconds":elapsed.as_secs_f64(),"defaultReadinessSeconds":120,"vmStopped":true,"handlerReleased":true,"boxDirectoryRemoved":true,"ownedSocketsRemoved":true,"noLateWrites":true,"callerBytesRetained":writes.len(),"callerCanaryRetained":true,"runtimeRssMeasured":false});
                std::fs::write(root.join("result.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
            }).catch_unwind().await;
            let cleanup=vm.destroy_with_timeout(0).await;
            std::fs::write(root.join("native-cleanup.json"),serde_json::to_vec_pretty(&serde_json::json!({"state":format!("{:?}",vm.state().await),"destroyOk":cleanup.is_ok(),"boxDirectoryAbsent":!home.join("boxes").join(&id).exists(),"socketDirectoryAbsent":!vm.socket_dir().exists()})).unwrap()).unwrap();
            if let Err(error)=result {std::panic::resume_unwind(error);}
            cleanup.unwrap();
        });
    }).unwrap().join().unwrap();
}
