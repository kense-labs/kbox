use super::*;

struct RefuseOnce(bool);

impl crate::vmm::VmHandler for RefuseOnce {
    fn stop(&mut self, _signal: i32, _timeout_ms: u64) -> Result<()> {
        if std::mem::replace(&mut self.0, false) {
            Err(BoxError::StateError("test source finalizer refused".into()))
        } else {
            Ok(())
        }
    }
    fn metrics(&self) -> crate::vmm::VmMetrics {
        crate::vmm::VmMetrics::default()
    }
    fn is_running(&self) -> bool {
        true
    }
    fn has_exited(&self) -> bool {
        false
    }
    fn pid(&self) -> u32 {
        42
    }
}

async fn empty_pool() -> WarmPool {
    WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(16),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn template_cleanup_waits_for_actual_consumer_close_and_can_retry() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let path = directory.path().to_path_buf();
    std::fs::write(path.join("template.ram"), b"live-input").unwrap();
    let consumer = directory.claim().unwrap();
    let pool = empty_pool().await;
    pool.template_resources.lock().await.directory = Some(directory);
    assert!(pool.drain_idle().await.is_err());
    assert_eq!(
        std::fs::read(path.join("template.ram")).unwrap(),
        b"live-input"
    );
    assert!(pool.template_resources.lock().await.directory.is_some());
    consumer.closed();
    pool.drain_idle().await.unwrap();
    assert!(!path.exists());
    assert!(pool.template_resources.lock().await.directory.is_none());
}

#[tokio::test]
async fn build_retry_cannot_overwrite_unfinalized_template_input() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let input = directory.path().join("template.ram");
    std::fs::write(&input, b"mapped-by-other-vm").unwrap();
    let consumer = directory.claim().unwrap();
    let resources = Arc::new(Mutex::new(TemplateResources {
        directory: Some(directory),
        source: None,
    }));
    let result =
        WarmPool::build_template(&BoxConfig::default(), &EventEmitter::new(16), &resources).await;
    assert!(
        matches!(result, Err(BoxError::IoError(ref error)) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert_eq!(std::fs::read(&input).unwrap(), b"mapped-by-other-vm");
    assert!(resources.lock().await.source.is_none());
    consumer.closed();
    let directory = resources.lock().await.directory.take().unwrap();
    directory.remove().unwrap();
}

#[tokio::test]
async fn independent_pool_cleanup_does_not_remove_other_pool_or_caller_input() {
    let root = tempfile::tempdir().unwrap();
    let caller = root.path().join("tpl-caller");
    std::fs::create_dir(&caller).unwrap();
    std::fs::write(caller.join("canary"), b"caller-owned").unwrap();
    let first = empty_pool().await;
    let second = empty_pool().await;
    let a = TemplateDirectory::create(root.path()).unwrap();
    let a_path = a.path().to_path_buf();
    let b = TemplateDirectory::create(root.path()).unwrap();
    let b_path = b.path().to_path_buf();
    std::fs::write(b_path.join("template.ram"), b"second-input").unwrap();
    first.template_resources.lock().await.directory = Some(a);
    second.template_resources.lock().await.directory = Some(b);
    *first.template.lock().await = TemplateState::Unavailable;
    first.drain_idle().await.unwrap();
    assert!(!a_path.exists());
    assert_eq!(
        std::fs::read(b_path.join("template.ram")).unwrap(),
        b"second-input"
    );
    assert_eq!(
        std::fs::read(caller.join("canary")).unwrap(),
        b"caller-owned"
    );
    second.drain_idle().await.unwrap();
    assert!(!b_path.exists());
}

#[tokio::test]
async fn source_stop_refusal_retains_exact_manager_and_input_until_retry() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let path = directory.path().to_path_buf();
    std::fs::write(path.join("template.ram"), b"source-input").unwrap();
    let mut vm = VmManager::new(BoxConfig::default(), EventEmitter::new(16));
    vm.home_dir = root.path().join("home");
    vm.template_use = Some(directory.claim().unwrap());
    let handler = Arc::clone(&vm.handler);
    *handler.write().await = Some(Box::new(RefuseOnce(true)));
    let pool = empty_pool().await;
    *pool.template_resources.lock().await = TemplateResources {
        source: Some(vm),
        directory: Some(directory),
    };
    assert!(pool.drain_idle().await.is_err());
    {
        let resources = pool.template_resources.lock().await;
        assert!(Arc::ptr_eq(
            &resources.source.as_ref().unwrap().handler,
            &handler
        ));
        assert!(handler.read().await.is_some());
        assert_eq!(
            std::fs::read(path.join("template.ram")).unwrap(),
            b"source-input"
        );
    }
    pool.drain_idle().await.unwrap();
    assert!(handler.read().await.is_none());
    assert!(pool.template_resources.lock().await.source.is_none());
    assert!(!path.exists());
}

#[tokio::test]
async fn completed_removal_remains_recorded_when_its_waiter_is_dropped() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let capability = directory.exclusive_directory().unwrap().unwrap();
    let worker = Arc::clone(&directory);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (go_tx, go_rx) = tokio::sync::oneshot::channel();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let task = tokio::task::spawn_blocking(move || {
        started_tx.send(()).unwrap();
        go_rx.blocking_recv().unwrap();
        let result = worker.finish_removal(capability);
        drop(worker);
        done_tx.send(result).unwrap();
    });
    started_rx.await.unwrap();
    drop(task);
    go_tx.send(()).unwrap();
    done_rx.await.unwrap().unwrap();
    std::fs::create_dir(directory.path()).unwrap();
    std::fs::write(
        directory.path().join("caller"),
        b"replacement-after-completion",
    )
    .unwrap();
    directory.remove().unwrap();
    assert!(directory.claim().is_err());
    assert_eq!(
        std::fs::read(directory.path().join("caller")).unwrap(),
        b"replacement-after-completion"
    );
}
