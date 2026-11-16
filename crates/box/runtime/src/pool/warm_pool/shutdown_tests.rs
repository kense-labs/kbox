use super::*;

#[tokio::test]
async fn drain_idle_signals_and_joins_in_progress_maintenance() {
    let pool = WarmPool::start(
        PoolConfig {
            min_idle: 0,
            max_size: 1,
            ..PoolConfig::default()
        },
        BoxConfig::default(),
        EventEmitter::new(10),
    )
    .await
    .expect("start an empty pool without a hypervisor");

    pool.signal_shutdown();
    pool.replenish_handle
        .lock()
        .await
        .take()
        .unwrap()
        .await
        .unwrap();
    pool.shutdown_tx.send(false).unwrap();

    let mut shutdown = pool.shutdown_rx.clone();
    let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
    let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
    *pool.replenish_handle.lock().await = Some(tokio::spawn(async move {
        shutdown.wait_for(|stop| *stop).await.unwrap();
        observed_tx.send(()).unwrap();
        // Model the cleanup of a VM whose boot finished during shutdown.
        finish_rx.await.unwrap();
    }));

    let mut draining = Box::pin(pool.drain_idle());
    tokio::select! {
        result = &mut draining => {
            panic!("drain returned before maintenance cleanup finished: {result:?}");
        }
        observed = tokio::time::timeout(Duration::from_secs(5), observed_rx) => {
            observed.expect("drain must signal shutdown").unwrap();
        }
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut draining)
            .await
            .is_err(),
        "drain must retain maintenance ownership while its cleanup is pending"
    );
    finish_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), draining)
        .await
        .expect("drain completes after maintenance cleanup")
        .unwrap();
    assert!(pool.replenish_handle.lock().await.is_none());
    assert_eq!(pool.idle_count().await, 0);
    assert_eq!(pool.stats().await.idle_count, 0);
}

#[test]
fn stopping_an_unbuilt_pool_preserves_a_preexisting_template_directory() {
    const CHILD: &str = "K_BOX_TEST_UNOWNED_POOL_TEMPLATE_CHILD";
    const CASE: &str = "pool::warm_pool::shutdown_tests::stopping_an_unbuilt_pool_preserves_a_preexisting_template_directory";
    if std::env::var_os(CHILD).is_some() {
        let home = std::path::PathBuf::from(std::env::var_os("K_HOME").unwrap());
        assert_eq!(
            std::fs::read_to_string(home.join(".owned-template-test")).unwrap(),
            "isolated-pool-template-test\n"
        );
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let image = "pool-template:unowned-preservation";
                let directory = home
                    .join("pool")
                    .join(format!("tpl-{:016x}", crate::vm::fnv1a_hash(image)));
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(
                    directory.join("caller-canary"),
                    b"caller-template-not-owned-by-this-pool",
                )
                .unwrap();
                let pool = WarmPool::start(
                    PoolConfig {
                        min_idle: 0,
                        max_size: 1,
                        ..PoolConfig::default()
                    },
                    BoxConfig {
                        image: image.into(),
                        ..BoxConfig::default()
                    },
                    EventEmitter::new(16),
                )
                .await
                .unwrap();
                assert!(matches!(
                    *pool.template.lock().await,
                    TemplateState::Unbuilt
                ));
                pool.drain_idle().await.unwrap();
                assert!(
                    directory.is_dir(),
                    "stopping an unbuilt pool must not delete a caller template: {}",
                    directory.display()
                );
                assert_eq!(
                    std::fs::read(directory.join("caller-canary")).unwrap(),
                    b"caller-template-not-owned-by-this-pool"
                );
                assert_eq!(pool.idle_count().await, 0);
            });
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home 空格");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join(".owned-template-test"),
        b"isolated-pool-template-test\n",
    )
    .unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CASE, "--nocapture"])
        .env(CHILD, "1")
        .env("K_HOME", &home)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "isolated child failed:\n{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}
