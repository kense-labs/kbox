//! WarmPool — Pre-warmed pool of ready-to-use MicroVMs.
//!
//! Maintains a set of pre-booted VMs in `Ready` state so that
//! `acquire()` can return a VM instantly without waiting for boot.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use kbox_core::config::{BoxConfig, PoolConfig};
use kbox_core::error::{BoxError, Result};
use kbox_core::event::{BoxEvent, EventEmitter};
use tokio::sync::{watch, Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::task::{JoinHandle, JoinSet};

use crate::pool::scaler::PoolScaler;
use crate::vm::{TemplateDirectory, VmManager};

/// A pre-warmed VM waiting in the pool.
struct WarmVm {
    /// The ready VM manager instance.
    vm: VmManager,
    /// When this VM was added to the pool.
    created_at: Instant,
}

type RetainedVm = Arc<Mutex<VmManager>>;
type RetainedVms = Arc<parking_lot::Mutex<Vec<RetainedVm>>>;

enum PoolCloseScope {
    Selected,
    AllRetained,
}

struct ForegroundHandoff {
    retained: RetainedVms,
    shutdown: watch::Sender<bool>,
    owner: std::sync::Weak<Mutex<VmManager>>,
}

impl Drop for ForegroundHandoff {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let retained = self
                .retained
                .lock()
                .iter()
                .any(|vm| Arc::ptr_eq(vm, &owner));
            if retained {
                // A cancelled transfer must not keep launching beside its unclosed VM.
                let _ = self.shutdown.send(true);
            }
        }
    }
}

type CompletedBoot = (RetainedVm, ForegroundHandoff);

type BootVmFuture<'a> = Pin<Box<dyn Future<Output = Result<CompletedBoot>> + Send + 'a>>;

/// Keeps the in-flight warm-pool boot gauge balanced even when a boot task is
/// cancelled or panics while its JoinSet is being drained.
struct BootMetricGuard {
    metrics: Option<crate::prom::RuntimeMetrics>,
}

impl BootMetricGuard {
    fn new(metrics: Option<crate::prom::RuntimeMetrics>) -> Self {
        if let Some(metrics) = &metrics {
            metrics.warm_pool_boots_inflight.inc();
        }
        Self { metrics }
    }
}

impl Drop for BootMetricGuard {
    fn drop(&mut self) {
        if let Some(metrics) = &self.metrics {
            metrics.warm_pool_boots_inflight.dec();
        }
    }
}

/// Acquire the per-pool and optional daemon-wide permits in one consistent
/// order. Keeping the order identical for eager and on-demand boots avoids a
/// cross-pool semaphore cycle while a daemon is filling multiple images.
async fn acquire_boot_permits(
    boot_limiter: Arc<Semaphore>,
    global_boot_limiter: Option<Arc<Semaphore>>,
) -> Result<(OwnedSemaphorePermit, Option<OwnedSemaphorePermit>)> {
    let pool_permit = boot_limiter
        .acquire_owned()
        .await
        .map_err(|_| BoxError::PoolError("Warm-pool boot limiter closed".to_string()))?;
    let global_permit = match global_boot_limiter {
        Some(limiter) => Some(limiter.acquire_owned().await.map_err(|_| {
            BoxError::PoolError("Warm-pool global boot limiter closed".to_string())
        })?),
        None => None,
    };
    Ok((pool_permit, global_permit))
}

/// Statistics about the warm pool.
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of idle VMs ready for acquisition.
    pub idle_count: usize,
    /// Total number of VMs created by this pool (including acquired ones).
    pub total_created: u64,
    /// Total number of VMs acquired from the pool.
    pub total_acquired: u64,
    /// Total number of VMs released back to the pool.
    pub total_released: u64,
    /// Total number of VMs evicted due to idle TTL.
    pub total_evicted: u64,
}

/// A pre-warmed pool of ready-to-use MicroVMs.
///
/// The pool maintains `min_idle` VMs in `Ready` state. When a VM is
/// acquired, the pool spawns a replacement in the background. Idle VMs
/// that exceed `idle_ttl_secs` are automatically evicted.
///
/// # Usage
///
/// ```ignore
/// let pool = WarmPool::start(pool_config, box_config, emitter).await?;
/// let vm = pool.acquire().await?;  // Instant if pool has capacity
/// // ... use vm ...
/// pool.release(vm).await?;         // Return to pool or destroy
/// pool.drain().await?;             // Graceful shutdown
/// ```
pub struct WarmPool {
    /// Pool configuration.
    config: PoolConfig,
    /// Base BoxConfig template for creating new VMs.
    box_config: BoxConfig,
    /// Idle VMs ready for acquisition.
    idle: Arc<Mutex<Vec<WarmVm>>>,
    /// Pool statistics.
    stats: Arc<Mutex<PoolStats>>,
    /// Event emitter for pool lifecycle events.
    event_emitter: EventEmitter,
    /// Background replenishment task handle.
    ///
    /// The handle is behind a mutex because daemon shutdown owns pools through
    /// `Arc<WarmPool>`. Taking and awaiting it during `drain_idle` is required:
    /// dropping a maintenance task while it is booting can leave its newly
    /// created VM (and shim) detached from the pool's idle list.
    replenish_handle: Mutex<Option<JoinHandle<()>>>,
    /// Shutdown signal sender.
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver (cloned for background task).
    shutdown_rx: watch::Receiver<bool>,
    /// Autoscaler for dynamic min_idle adjustment (None if scaling disabled).
    scaler: Option<Arc<Mutex<PoolScaler>>>,
    /// Prometheus metrics (optional).
    metrics: Option<crate::prom::RuntimeMetrics>,
    /// Per-pool boot limiter shared by eager fill and on-demand misses.
    boot_limiter: Arc<Semaphore>,
    /// Optional daemon-wide boot limiter shared by all image pools.
    global_boot_limiter: Option<Arc<Semaphore>>,
    /// Snapshot-fork template state (built lazily on first fill when
    /// `config.snapshot_fork`): the file-backed RAM image + state file every other
    /// pool VM restores from. Caches an `Unavailable` verdict so a build failure
    /// (native VM snapshot unsupported on this build) is not re-attempted on every
    /// fill — the pool cold-boots instead.
    template: Arc<Mutex<TemplateState>>,
    template_resources: Arc<Mutex<TemplateResources>>,
    retiring: RetainedVms,
    /// Consecutive idle acquire re-auth failures while a snapshot-fork template
    /// is `Ready`. Hollow restores can pass publish then fail `health_check`;
    /// after a small bound the template is invalidated so cold-boot serves
    /// leases (#563).
    fork_reauth_failures: AtomicU32,
}

/// A built snapshot-fork template: the shared RAM image + state file that pool VMs
/// restore from (MAP_PRIVATE CoW of the RAM file).
#[derive(Clone)]
struct PoolTemplate {
    mem_file: String,
    state_file: String,
    rootfs_cache_key: Option<String>,
    directory: Arc<TemplateDirectory>,
}

#[derive(Default)]
struct TemplateResources {
    directory: Option<Arc<TemplateDirectory>>,
    source: Option<VmManager>,
}

/// How many consecutive template-build failures are tolerated before the
/// verdict becomes permanently `Unavailable`. A transient failure (host
/// resource pressure, a source VM slow to bind its snapshot socket) presents
/// identically to "snapshot unsupported by this libkrun build" ("snapshot
/// socket never appeared"), so a bounded retry avoids permanently downgrading
/// the whole pool to cold-boot on a one-off hiccup, while still giving up on a
/// genuinely-unsupported host after a few attempts.
const MAX_TEMPLATE_BUILD_FAILURES: u32 = 3;

/// Consecutive idle acquire re-auth failures before a Ready snapshot-fork
/// template is treated as hollow and permanently invalidated (#563).
const MAX_FORK_REAUTH_FAILURES: u32 = 3;

/// Settle after dropping host exec proof before snapshot so the vsock muxer
/// can drain (libkrun reaper timeout is 5s on the snapshot line).
const SNAPSHOT_VSOCK_QUIESCE: Duration = Duration::from_secs(6);

/// Bound concurrent VM teardown so a large pool does not turn shutdown into a
/// host-resource spike. Teardown is I/O-heavy and independent per VM, so a
/// small fixed fan-out shortens the critical path without overwhelming the
/// hypervisor or filesystem.
const MAX_DRAIN_CONCURRENCY: usize = 4;

/// Warm-pool VMs are ephemeral (`persistent = false` in the pool daemon). Give
/// the guest a short graceful window during daemon shutdown, then let the VM
/// lifecycle's force-stop fallback finish host cleanup. This keeps shutdown
/// bounded when a keepalive process ignores SIGTERM.
const EPHEMERAL_DRAIN_TIMEOUT_MS: u64 = 2_000;

/// Cached state of the snapshot-fork template.
enum TemplateState {
    /// Not built yet — the first snapshot-fork fill attempts the build.
    Unbuilt,
    /// Built and ready; pool VMs restore from it.
    Ready(PoolTemplate),
    /// The last build failed but is still retryable; carries the consecutive
    /// failure count. A later fill retries until it reaches
    /// `MAX_TEMPLATE_BUILD_FAILURES`, then it becomes `Unavailable`.
    Failing(u32),
    /// The build failed permanently (native VM snapshot unavailable on this
    /// build/platform, or too many consecutive failures). Cached so it is not
    /// retried — `boot_or_restore` cold-boots instead.
    Unavailable,
}

#[derive(Clone, Copy)]
enum InitialFill {
    /// Boot the configured `min_idle` count before returning from `start`.
    Eager,
    /// Boot one ready VM before returning and let maintenance fill the rest.
    FirstReady,
}

/// Inputs for one bounded warm-pool boot batch.
///
/// Keeping the batch request together makes the concurrency boundary explicit
/// and avoids a long parameter list that is easy to get out of sync when new
/// pool-wide controls are added.
struct BootBatch<'a> {
    snapshot_fork: bool,
    box_config: &'a BoxConfig,
    event_emitter: &'a EventEmitter,
    template: &'a Arc<Mutex<TemplateState>>,
    template_resources: &'a Arc<Mutex<TemplateResources>>,
    needed: usize,
    max_concurrent_boots: usize,
    metrics: Option<crate::prom::RuntimeMetrics>,
    boot_limiter: Arc<Semaphore>,
    global_boot_limiter: Option<Arc<Semaphore>>,
    retained: &'a RetainedVms,
    shutdown: &'a watch::Sender<bool>,
}

impl WarmPool {
    /// Create and start the warm pool.
    ///
    /// Spawns `min_idle` VMs in the background and starts the
    /// replenishment/eviction loop.
    pub async fn start(
        config: PoolConfig,
        box_config: BoxConfig,
        event_emitter: EventEmitter,
    ) -> Result<Self> {
        Self::start_with_metrics(config, box_config, event_emitter, None).await
    }

    /// Create and start the warm pool with an optional shared metrics sink.
    ///
    /// The sink is installed before the initial fill so the first pre-warmed
    /// VMs contribute boot, cache, and pool metrics. [`Self::start`] remains
    /// the compatibility entry point for callers that do not need metrics.
    pub async fn start_with_metrics(
        config: PoolConfig,
        box_config: BoxConfig,
        event_emitter: EventEmitter,
        metrics: Option<crate::prom::RuntimeMetrics>,
    ) -> Result<Self> {
        Self::start_with_metrics_and_boot_limiter(config, box_config, event_emitter, metrics, None)
            .await
    }

    /// Create and start the warm pool with optional metrics and a shared
    /// daemon-wide boot limiter.
    ///
    /// `max_concurrent_boots` still limits each pool independently. When a
    /// shared limiter is supplied, it additionally caps aggregate boots across
    /// all pools that use it (for example, a multi-image pool daemon).
    pub async fn start_with_metrics_and_boot_limiter(
        config: PoolConfig,
        box_config: BoxConfig,
        event_emitter: EventEmitter,
        metrics: Option<crate::prom::RuntimeMetrics>,
        global_boot_limiter: Option<Arc<Semaphore>>,
    ) -> Result<Self> {
        Self::start_with_metrics_and_boot_limiter_with_fill(
            config,
            box_config,
            event_emitter,
            metrics,
            global_boot_limiter,
            InitialFill::Eager,
        )
        .await
    }

    /// Create a warm pool with one ready VM on the critical path.
    ///
    /// This is intended for lazy, multi-image daemons: the first request for a
    /// new pool can run as soon as one VM is ready while the maintenance loop
    /// fills the remaining `min_idle` capacity in the background. The regular
    /// [`Self::start_with_metrics_and_boot_limiter`] API keeps its eager-fill
    /// behavior for explicit pool startup.
    pub async fn start_with_metrics_and_boot_limiter_first_ready(
        config: PoolConfig,
        box_config: BoxConfig,
        event_emitter: EventEmitter,
        metrics: Option<crate::prom::RuntimeMetrics>,
        global_boot_limiter: Option<Arc<Semaphore>>,
    ) -> Result<Self> {
        Self::start_with_metrics_and_boot_limiter_with_fill(
            config,
            box_config,
            event_emitter,
            metrics,
            global_boot_limiter,
            InitialFill::FirstReady,
        )
        .await
    }

    async fn start_with_metrics_and_boot_limiter_with_fill(
        config: PoolConfig,
        box_config: BoxConfig,
        event_emitter: EventEmitter,
        metrics: Option<crate::prom::RuntimeMetrics>,
        global_boot_limiter: Option<Arc<Semaphore>>,
        initial_fill: InitialFill,
    ) -> Result<Self> {
        if config.max_size == 0 {
            return Err(BoxError::PoolError(
                "Pool max_size must be greater than 0".to_string(),
            ));
        }
        if config.min_idle > config.max_size {
            return Err(BoxError::PoolError(format!(
                "Pool min_idle ({}) cannot exceed max_size ({})",
                config.min_idle, config.max_size
            )));
        }
        if config.max_concurrent_boots == 0 {
            return Err(BoxError::PoolError(
                "Pool max_concurrent_boots must be greater than 0".to_string(),
            ));
        }

        let idle = Arc::new(Mutex::new(Vec::with_capacity(config.max_size)));
        let stats = Arc::new(Mutex::new(PoolStats {
            idle_count: 0,
            total_created: 0,
            total_acquired: 0,
            total_released: 0,
            total_evicted: 0,
        }));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let scaler = if config.scaling.enabled {
            Some(Arc::new(Mutex::new(PoolScaler::new(
                config.scaling.clone(),
                config.min_idle,
                config.max_size,
            ))))
        } else {
            None
        };

        let boot_limiter = Arc::new(Semaphore::new(config.max_concurrent_boots));
        let mut box_config = box_config;
        // Snapshot-fork requires an IDLE deferred-main template (empty vsock
        // muxer). Auto-enable when the daemon omitted `--deferred` (#563).
        if config.snapshot_fork && !box_config.deferred_main {
            tracing::info!(
                "snapshot-fork requires deferred-main idle template; enabling deferred_main"
            );
            box_config.deferred_main = true;
        }
        let pool = Self {
            config,
            box_config,
            idle,
            stats,
            event_emitter,
            replenish_handle: Mutex::new(None),
            shutdown_tx,
            shutdown_rx,
            scaler,
            metrics,
            boot_limiter,
            global_boot_limiter,
            template: Arc::new(Mutex::new(TemplateState::Unbuilt)),
            template_resources: Arc::new(Mutex::new(TemplateResources::default())),
            retiring: Arc::new(parking_lot::Mutex::new(Vec::new())),
            fork_reauth_failures: AtomicU32::new(0),
        };

        if let Some(metrics) = &pool.metrics {
            metrics.warm_pool_capacity.set(pool.config.max_size as i64);
        }

        // Initial fill. Lazy pools only wait for the first ready VM; the
        // maintenance loop immediately schedules the remaining capacity.
        let initial_target = match initial_fill {
            InitialFill::Eager => pool.config.min_idle,
            InitialFill::FirstReady => pool.config.min_idle.min(1),
        };
        let initial_fill_started = Instant::now();
        pool.fill_to_target(initial_target).await;
        if let Some(metrics) = &pool.metrics {
            metrics
                .warm_pool_initial_fill_duration
                .observe(initial_fill_started.elapsed().as_secs_f64());
        }

        // Start background maintenance loop
        let handle = pool.spawn_maintenance_loop();
        *pool.replenish_handle.lock().await = Some(handle);

        tracing::info!(
            min_idle = pool.config.min_idle,
            max_size = pool.config.max_size,
            idle_ttl_secs = pool.config.idle_ttl_secs,
            "Warm pool started"
        );

        Ok(pool)
    }

    /// Attach Prometheus metrics to this pool.
    pub fn set_metrics(&mut self, metrics: crate::prom::RuntimeMetrics) {
        metrics.warm_pool_capacity.set(self.config.max_size as i64);
        metrics.warm_pool_size.set(
            self.idle
                .try_lock()
                .map(|idle| idle.len() as i64)
                .unwrap_or_default(),
        );
        self.metrics = Some(metrics);
    }

    fn sync_idle_metric(metrics: Option<&crate::prom::RuntimeMetrics>, idle_count: usize) {
        if let Some(metrics) = metrics {
            metrics.warm_pool_size.set(idle_count as i64);
        }
    }

    /// Acquire a ready VM from the pool.
    ///
    /// Idle membership alone must not invent operable Ready — re-prove exec
    /// health before handoff (#423 / #421). Stale idle VMs are destroyed and
    /// the next candidate (or an on-demand boot) is tried.
    pub async fn acquire(&self) -> Result<VmManager> {
        self.ensure_admission_open()?;
        loop {
            let warm_vm = {
                let mut idle = self.idle.lock().await;
                self.ensure_admission_open()?;
                let mut stats = self.stats.lock().await;
                self.ensure_admission_open()?;
                let warm_vm = idle.pop().map(|warm| self.retain_foreground_vm(warm.vm));
                if warm_vm.is_some() {
                    Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
                    stats.idle_count = idle.len();
                }
                warm_vm
            };

            let Some((warm_vm, _handoff)) = warm_vm else {
                break;
            };

            // Publish authenticated once; idle TTL only ages by wall clock.
            // Guest exec can die while shim PID / in-memory Ready remain —
            // refuse inventing a lease from pool membership alone (#423).
            let (health, box_id) = {
                let vm = warm_vm.lock().await;
                (vm.health_check().await, vm.box_id().to_owned())
            };
            match health {
                Ok(true) => {
                    self.fork_reauth_failures.store(0, Ordering::Relaxed);
                    let mut scaler = match &self.scaler {
                        Some(scaler) => Some(scaler.lock().await),
                        None => None,
                    };
                    let mut stats = self.stats.lock().await;
                    self.ensure_admission_open()?;
                    let vm = self.take_foreground_vm(warm_vm)?;
                    stats.total_acquired += 1;
                    if let Some(scaler) = scaler.as_mut() {
                        scaler.record_acquire(true);
                    }

                    if let Some(ref m) = self.metrics {
                        m.warm_pool_hits.inc();
                    }

                    self.event_emitter.emit(BoxEvent::with_string(
                        "pool.vm.acquired",
                        format!("Acquired VM {box_id} from pool"),
                    ));

                    tracing::debug!(
                        %box_id,
                        "Acquired authenticated VM from warm pool"
                    );

                    return Ok(vm);
                }
                Ok(false) | Err(_) => {
                    tracing::warn!(
                        %box_id,
                        "Idle pool VM failed exec re-auth; destroying instead of inventing Ready"
                    );
                    self.retire_foreground_vm(warm_vm, Some(2000)).await?;
                    let mut stats = self.stats.lock().await;
                    stats.total_evicted += 1;
                    drop(stats);
                    if self.config.snapshot_fork {
                        let failures =
                            self.fork_reauth_failures.fetch_add(1, Ordering::Relaxed) + 1;
                        if failures >= MAX_FORK_REAUTH_FAILURES {
                            self.invalidate_snapshot_fork_template(
                                "idle restores failed exec re-auth repeatedly",
                            )
                            .await;
                            self.fork_reauth_failures.store(0, Ordering::Relaxed);
                        }
                    }
                    continue;
                }
            }
        }

        // No authenticated idle VM — boot one on demand (miss)
        tracing::info!("No idle VM in pool, booting on demand");

        if let Some(ref scaler) = self.scaler {
            scaler.lock().await.record_acquire(false);
        }

        if let Some(ref m) = self.metrics {
            m.warm_pool_misses.inc();
        }

        let vm = self.boot_new_vm().await?;
        let (owner, _handoff) = self.retain_foreground_vm(vm);

        let mut stats = self.stats.lock().await;
        self.ensure_admission_open()?;
        let vm = self.take_foreground_vm(owner)?;
        stats.total_acquired += 1;

        Ok(vm)
    }

    /// Release a VM back to the pool.
    ///
    /// Idle membership must not invent operable Ready later — re-prove exec
    /// health before push (#429 / #423). Unauthenticated VMs are destroyed
    /// instead of polluting the idle set.
    ///
    /// If the pool is at capacity, the VM is destroyed instead.
    pub async fn release(&self, vm: VmManager) -> Result<()> {
        let (owner, _handoff) = self.retain_foreground_vm(vm);
        // Fail closed before taking the idle lock: health_check is async and
        // must not hold idle while dialing exec.
        let (health, box_id) = {
            let vm = owner.lock().await;
            (vm.health_check().await, vm.box_id().to_owned())
        };
        match health {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                tracing::warn!(
                    %box_id,
                    "Released VM failed exec re-auth; destroying instead of inventing idle Ready"
                );
                self.retire_foreground_vm(owner, Some(2000)).await?;
                let mut stats = self.stats.lock().await;
                stats.total_evicted += 1;
                return Ok(());
            }
        }

        let mut idle = self.idle.lock().await;
        let mut stats = self.stats.lock().await;

        // Don't return a VM to a pool that is shutting down: drain_idle has (or
        // soon will have) cleared `idle` and won't run again, so a push here leaks
        // the VM (no Drop reaper). Checked under the idle lock so it is atomic with
        // a concurrent drain_idle. Destroy the VM instead.
        if *self.shutdown_rx.borrow() {
            drop(stats);
            drop(idle);
            self.retire_foreground_vm(owner, None).await?;
            return Ok(());
        }

        if idle.len() >= self.config.max_size {
            // Pool is full — destroy the VM
            drop(stats);
            drop(idle); // Release lock before async destroy
            self.retire_foreground_vm(owner, None).await?;

            tracing::debug!(
                box_id = %box_id,
                "Pool full, destroyed released VM"
            );
            return Ok(());
        }

        let vm = self.take_foreground_vm(owner)?;
        idle.push(WarmVm {
            vm,
            created_at: Instant::now(),
        });

        stats.total_released += 1;
        stats.idle_count = idle.len();

        if let Some(ref m) = self.metrics {
            m.warm_pool_size.set(idle.len() as i64);
        }

        self.event_emitter.emit(BoxEvent::with_string(
            "pool.vm.released",
            format!("Released VM {} back to pool", box_id),
        ));

        tracing::debug!(
            box_id = %box_id,
            idle_count = idle.len(),
            "Released VM back to warm pool"
        );

        Ok(())
    }

    /// Get current pool statistics.
    pub async fn stats(&self) -> PoolStats {
        self.stats.lock().await.clone()
    }

    /// Get the number of idle VMs currently in the pool.
    pub async fn idle_count(&self) -> usize {
        self.idle.lock().await.len()
    }

    /// Signal the pool to shutdown. This signals the background task to stop
    /// replenishing and sets the shutdown flag. VMs will continue to exist
    /// until the pool is drained or dropped.
    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        tracing::info!("Warm pool shutdown signaled");
    }

    async fn join_maintenance(&self) {
        let mut handle = self.replenish_handle.lock().await;
        // Keep the join owner in its slot so cancelling a drain can retry the wait.
        if let Some(task) = handle.as_mut() {
            let _ = task.await;
        }
        *handle = None;
    }

    /// Gracefully drain all VMs and stop the pool.
    pub async fn drain(&mut self) -> Result<()> {
        // Signal shutdown to background task
        let _ = self.shutdown_tx.send(true);

        // Wait for background task to finish
        self.join_maintenance().await;

        // Detach idle VMs before destroying them. VM teardown is asynchronous
        // and must not hold the pool lock, otherwise acquire/release and the
        // maintenance loop can be blocked for the entire drain duration.
        let idle_vms = {
            let mut idle = self.idle.lock().await;
            // Wait before detaching: cancellation must leave actual managers owned.
            let mut stats = self.stats.lock().await;
            let idle_vms = idle.drain(..).collect::<Vec<_>>();
            stats.idle_count = idle.len();
            Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
            idle_vms
        };
        let count = idle_vms.len();

        Self::destroy_vms(
            idle_vms,
            &self.retiring,
            None,
            "drain",
            PoolCloseScope::AllRetained,
        )
        .await?;

        self.clear_snapshot_template().await?;

        let mut stats = self.stats.lock().await;
        stats.idle_count = 0;

        self.event_emitter.emit(BoxEvent::empty("pool.drained"));

        tracing::info!(destroyed = count, "Warm pool drained");

        Ok(())
    }

    /// Destroy all idle VMs without consuming the pool (`&self`), so it can be
    /// shut down from behind an `Arc` (e.g. a daemon serving concurrent requests).
    /// This method signals and joins the background replenisher before taking
    /// the idle snapshot, then tears down VMs with bounded concurrency.
    pub async fn drain_idle(&self) -> Result<()> {
        // Stop and join the maintenance loop before taking the idle snapshot.
        // Otherwise a replenishment batch can finish after this method drains
        // the vector and publish a VM that no owner remains to destroy.
        self.signal_shutdown();
        self.join_maintenance().await;

        // Detach idle VMs before destroying them. Keeping `idle` locked while
        // awaiting VM teardown blocks concurrent acquire/release operations and
        // widens the shutdown race window for a replenishment batch.
        let idle_vms = {
            let mut idle = self.idle.lock().await;
            // Wait before detaching: cancellation must leave actual managers owned.
            let mut stats = self.stats.lock().await;
            let idle_vms = idle.drain(..).collect::<Vec<_>>();
            stats.idle_count = idle.len();
            Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
            idle_vms
        };
        let count = idle_vms.len();
        let timeout_ms = (!self.box_config.persistent).then_some(EPHEMERAL_DRAIN_TIMEOUT_MS);
        Self::destroy_vms(
            idle_vms,
            &self.retiring,
            timeout_ms,
            "drain_idle",
            PoolCloseScope::AllRetained,
        )
        .await?;
        self.clear_snapshot_template().await?;
        tracing::info!(destroyed = count, "Warm pool idle VMs drained");
        Ok(())
    }

    /// Release only remembered owned artifacts after every consumer closes.
    async fn clear_snapshot_template(&self) -> Result<()> {
        let prior = {
            let mut state = self.template.lock().await;
            std::mem::replace(&mut *state, TemplateState::Unbuilt)
        };
        drop(prior);
        let mut resources = self.template_resources.lock().await;
        if let Some(source) = resources.source.as_mut() {
            source
                .destroy_with_timeout(EPHEMERAL_DRAIN_TIMEOUT_MS)
                .await?;
            resources.source = None;
        }
        if let Some(directory) = &resources.directory {
            if let Some(capability) = directory.exclusive_directory()? {
                let directory = Arc::clone(directory);
                tokio::task::spawn_blocking(move || directory.finish_removal(capability))
                    .await
                    .map_err(|error| {
                        BoxError::PoolError(format!("Template cleanup task failed: {error}"))
                    })??;
            }
            resources.directory = None;
        }
        Ok(())
    }

    async fn destroy_vms(
        vms: Vec<WarmVm>,
        retained: &RetainedVms,
        timeout_ms: Option<u64>,
        operation: &'static str,
        scope: PoolCloseScope,
    ) -> Result<()> {
        // Register every actual manager before any teardown can yield or be cancelled.
        let mut pending = {
            let mut owners = retained.lock();
            let selected = vms
                .into_iter()
                .map(|warm| Arc::new(Mutex::new(warm.vm)))
                .collect::<Vec<_>>();
            owners.extend(selected.iter().cloned());
            match scope {
                PoolCloseScope::Selected => selected,
                PoolCloseScope::AllRetained => owners.clone(),
            }
            .into_iter()
        };
        let mut tasks = JoinSet::new();
        for _ in 0..MAX_DRAIN_CONCURRENCY {
            if let Some(vm) = pending.next() {
                tasks.spawn(Self::destroy_one(vm, Arc::clone(retained), timeout_ms));
            }
        }
        let mut failure = None;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((box_id, Ok(()))) => tracing::debug!(%box_id, operation, "Destroyed pooled VM"),
                Ok((box_id, Err(error))) => {
                    tracing::warn!(%box_id, %error, operation, "Pooled VM retained for finalization retry");
                    failure.get_or_insert(error);
                }
                Err(error) => {
                    failure.get_or_insert_with(|| {
                        BoxError::PoolError(format!("Pooled VM teardown task failed: {error}"))
                    });
                }
            }
            if let Some(vm) = pending.next() {
                tasks.spawn(Self::destroy_one(vm, Arc::clone(retained), timeout_ms));
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    async fn destroy_one(
        owner: RetainedVm,
        retained: RetainedVms,
        timeout_ms: Option<u64>,
    ) -> (String, Result<()>) {
        let mut vm = owner.lock().await;
        let box_id = vm.box_id().to_owned();
        let result = match timeout_ms {
            Some(timeout) => vm.destroy_with_timeout(timeout).await,
            None => vm.destroy().await,
        };
        if result.is_ok() {
            retained
                .lock()
                .retain(|candidate| !Arc::ptr_eq(candidate, &owner));
        }
        (box_id, result)
    }

    /// Destroy a pool-owned manager; on failure, best-effort reap leftovers.
    ///
    /// `VmManager` has no Drop reaper, so ignored destroy errors permanently
    /// leak shim/mount/box-dir state across daemon shutdown and failed fills.
    async fn destroy_manager_or_reap(vm: &mut VmManager, timeout_ms: Option<u64>) -> Result<()> {
        let box_id = vm.box_id().to_string();
        let result = match timeout_ms {
            Some(timeout_ms) => vm.destroy_with_timeout(timeout_ms).await,
            None => vm.destroy().await,
        };
        if let Err(error) = &result {
            tracing::warn!(
                %box_id,
                %error,
                "Failed to destroy pooled VM; attempting orphan reap"
            );
            #[cfg(target_os = "linux")]
            crate::vm::reap::reap_orphaned_box(&box_id);
        }
        result
    }

    /// Remove and destroy specific idle VMs by their box IDs.
    ///
    /// Used when a fill partially fails and needs to roll back
    /// successfully added VMs.
    async fn remove_idle_vms(&self, box_ids: &[String]) {
        let to_destroy = {
            let mut idle = self.idle.lock().await;
            let mut stats = self.stats.lock().await;
            let mut selected = Vec::new();
            // Indices cannot survive an idle-list unlock; select identity at removal.
            for index in (0..idle.len()).rev() {
                if box_ids.iter().any(|id| id == idle[index].vm.box_id()) {
                    selected.push(idle.remove(index));
                }
            }
            stats.idle_count = idle.len();
            Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
            selected
        };
        if to_destroy.is_empty() {
            return;
        }

        // Destroy collected VMs (outside of pool lock).
        if let Err(error) = Self::destroy_vms(
            to_destroy,
            &self.retiring,
            None,
            "fill rollback",
            PoolCloseScope::Selected,
        )
        .await
        {
            self.signal_shutdown();
            tracing::warn!(%error, "Pool fill rollback retained unfinished VMs");
        }
    }

    /// Boot a new VM using the pool's template config.
    async fn boot_new_vm(&self) -> Result<VmManager> {
        let _boot_permits =
            acquire_boot_permits(self.boot_limiter.clone(), self.global_boot_limiter.clone())
                .await?;
        self.ensure_admission_open()?;
        let _boot_guard = BootMetricGuard::new(self.metrics.clone());
        let result = Self::boot_or_restore(
            self.config.snapshot_fork,
            &self.box_config,
            &self.event_emitter,
            &self.template,
            &self.template_resources,
            &self.retiring,
            &self.shutdown_tx,
        )
        .await;
        if result.is_err() {
            if let Some(metrics) = &self.metrics {
                metrics.warm_pool_boot_failures_total.inc();
            }
        }
        let (owner, _handoff) = result?;
        let mut stats = self.stats.lock().await;
        self.ensure_admission_open()?;
        let vm = self.take_foreground_vm(owner)?;
        let box_id = vm.box_id().to_owned();
        stats.total_created += 1;

        self.event_emitter.emit(BoxEvent::with_string(
            "pool.vm.created",
            format!("Booted new VM {box_id}"),
        ));

        Ok(vm)
    }

    fn ensure_admission_open(&self) -> Result<()> {
        if *self.shutdown_rx.borrow() {
            return Err(BoxError::PoolError("Warm pool is shutting down".into()));
        }
        Ok(())
    }

    fn retain_foreground_vm(&self, vm: VmManager) -> CompletedBoot {
        Self::register_vm(vm, &self.retiring, &self.shutdown_tx)
    }

    fn register_vm(
        vm: VmManager,
        retained: &RetainedVms,
        shutdown: &watch::Sender<bool>,
    ) -> CompletedBoot {
        let owner = Arc::new(Mutex::new(vm));
        retained.lock().push(owner.clone());
        let handoff = ForegroundHandoff {
            retained: retained.clone(),
            shutdown: shutdown.clone(),
            owner: Arc::downgrade(&owner),
        };
        (owner, handoff)
    }

    fn take_foreground_vm(&self, owner: RetainedVm) -> Result<VmManager> {
        Self::take_registered_vm(owner, &self.retiring, &self.shutdown_tx)
    }

    fn take_registered_vm(
        owner: RetainedVm,
        retained: &RetainedVms,
        shutdown: &watch::Sender<bool>,
    ) -> Result<VmManager> {
        let mut owners = retained.lock();
        if let Some(index) = owners
            .iter()
            .position(|candidate| Arc::ptr_eq(candidate, &owner))
        {
            drop(owners.swap_remove(index));
            match Arc::try_unwrap(owner) {
                Ok(vm) => return Ok(vm.into_inner()),
                Err(owner) => owners.push(owner),
            }
        } else {
            owners.push(owner);
        }
        drop(owners);
        let _ = shutdown.send(true);
        Err(BoxError::PoolError(
            "Pooled VM ownership is closing or unavailable; retained for cleanup".into(),
        ))
    }

    async fn retire_foreground_vm(&self, owner: RetainedVm, timeout_ms: Option<u64>) -> Result<()> {
        let (_, result) = Self::destroy_one(owner, self.retiring.clone(), timeout_ms).await;
        if result.is_err() {
            self.signal_shutdown();
        }
        result
    }

    /// Fill one slot: restore from the snapshot-fork template when enabled, else cold
    /// boot. Static so both `boot_new_vm` and the background replenish task use it.
    fn boot_or_restore<'a>(
        snapshot_fork: bool,
        box_config: &'a BoxConfig,
        event_emitter: &'a EventEmitter,
        template: &'a Arc<Mutex<TemplateState>>,
        template_resources: &'a Arc<Mutex<TemplateResources>>,
        retained: &'a RetainedVms,
        shutdown: &'a watch::Sender<bool>,
    ) -> BootVmFuture<'a> {
        Box::pin(async move {
            if snapshot_fork && crate::vm::native_snapshot_fork_supported() {
                // Try the snapshot-fork template. If it can't be built (native VM
                // snapshot unavailable — the verdict is cached so this is attempted at
                // most once), fall back to a normal cold boot so the warm pool still
                // fills rather than failing outright.
                match Self::ensure_template(box_config, event_emitter, template, template_resources)
                    .await
                {
                    Ok(tpl) => {
                        if *shutdown.borrow() {
                            return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                        }
                        let mut cfg = box_config.clone();
                        cfg.snapshot_mem_file = Some(tpl.mem_file.clone());
                        cfg.restore_from = Some(tpl.state_file.clone());
                        cfg.snapshot_sock = None;
                        let mut vm = VmManager::new(cfg, event_emitter.clone());
                        vm.restore_rootfs_cache_key = tpl.rootfs_cache_key.clone();
                        vm.template_use = Some(tpl.directory.claim()?);
                        let (owner, handoff) = Self::register_vm(vm, retained, shutdown);
                        let restored = {
                            let mut vm = owner.lock().await;
                            async {
                                if *shutdown.borrow() {
                                    // This newly prepared manager has never entered boot.
                                    if let Some(template_use) = vm.template_use.take() {
                                        template_use.closed();
                                    }
                                    return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                                }
                                vm.boot().await?;
                                vm.wait_for_exec_available(std::time::Duration::from_secs(120))
                                    .await?;
                                // Drop the boot handshake and re-prove so a single
                                // lucky connect cannot publish a hollow restore (#563).
                                vm.exec_client = None;
                                vm.wait_for_exec_available(std::time::Duration::from_secs(30))
                                    .await?;
                                // Align BoxState::Ready after authenticated heartbeat —
                                // #414 may leave Created after restore soft-probe (#416).
                                if !vm.set_boot_completion_state().await {
                                    return Err(BoxError::BoxBootError {
                                        message: "restored pool VM exec authenticated but Ready was not published"
                                            .to_string(),
                                        hint: None,
                                    });
                                }
                                vm.start_prewarmed_exec().await?;
                                Ok(())
                            }.await
                        };
                        match restored {
                            Ok(()) => return Ok((owner, handoff)),
                            Err(error) => {
                                Self::destroy_one(owner.clone(), retained.clone(), Some(2000))
                                    .await
                                    .1?;
                                if *shutdown.borrow() || matches!(error, BoxError::ConfigError(_)) {
                                    return Err(error);
                                }
                                tracing::warn!(
                                    %error,
                                    "snapshot-fork restore failed; cold-booting this pool VM"
                                );
                            }
                        }
                    }
                    Err(error @ BoxError::ConfigError(_)) => return Err(error),
                    Err(error) => {
                        tracing::debug!(%error, "snapshot-fork unavailable; cold-booting this pool VM");
                    }
                }
            } else if snapshot_fork {
                tracing::debug!(
                    "snapshot-fork is unavailable on this build; cold-booting without snapshot side effects"
                );
            }
            let vm = VmManager::new(box_config.clone(), event_emitter.clone());
            Self::boot_cold_vm(vm, template_resources, retained, shutdown).await
        })
    }

    async fn boot_cold_vm(
        vm: VmManager,
        template_resources: &Arc<Mutex<TemplateResources>>,
        retained: &RetainedVms,
        shutdown: &watch::Sender<bool>,
    ) -> Result<CompletedBoot> {
        let (owner, handoff) = Self::register_vm(vm, retained, shutdown);
        let (result, finalized) = {
            let mut vm = owner.lock().await;
            let mut refused_before_boot = false;
            let result = async {
                if *shutdown.borrow() {
                    refused_before_boot = true;
                    return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                }
                {
                    let resources = template_resources.lock().await;
                    if *shutdown.borrow() {
                        refused_before_boot = true;
                        return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                    }
                    if let Some(directory) = &resources.directory {
                        // Failed snapshot attempts still own private inputs during cold fallback.
                        vm.template_use = Some(directory.claim()?);
                    }
                }
                vm.boot().await?;
                Self::complete_cold_boot(&mut vm).await
            }
            .await;
            let finalized = refused_before_boot
                || vm.boot_failed_before_provider_start()
                || vm.cleanup_is_complete();
            (result, finalized)
        };
        if let Err(error) = result {
            if finalized {
                Self::destroy_one(owner.clone(), retained.clone(), Some(2000))
                    .await
                    .1?;
            }
            return Err(error);
        }
        Ok((owner, handoff))
    }

    async fn complete_cold_boot(vm: &mut VmManager) -> Result<()> {
        let result = async {
            vm.wait_for_exec_available(std::time::Duration::from_secs(120))
                .await?;
            // Re-assert Ready only after the pool's independent authenticated probe.
            if !vm.set_boot_completion_state().await {
                return Err(BoxError::BoxBootError {
                    message: "pool VM exec authenticated but Ready was not published".to_string(),
                    hint: None,
                });
            }
            vm.start_prewarmed_exec().await
        }
        .await;
        if result.is_err() {
            // VmManager has no Drop reaper after a successful boot.
            let _ = Self::destroy_manager_or_reap(vm, Some(2000)).await;
        }
        result
    }

    /// Boot a bounded batch of VMs and return results in completion order.
    ///
    /// VM boot is expensive in both host CPU and memory. A JoinSet without a
    /// concurrency limit turns a large min_idle or autoscaler step into a
    /// resource burst, so only `max_concurrent_boots` tasks are in flight at
    /// once. Each task owns cloned inputs, allowing the set to remain
    /// `'static` while the caller retains its pool state.
    async fn boot_batch(batch: BootBatch<'_>) -> Vec<Result<CompletedBoot>> {
        if batch.needed == 0 {
            return Vec::new();
        }

        let limit = bounded_boot_limit(batch.needed, batch.max_concurrent_boots);
        let mut set = tokio::task::JoinSet::new();
        let mut launched = 0usize;
        let mut results = Vec::with_capacity(batch.needed);

        while launched < batch.needed || !set.is_empty() {
            while launched < batch.needed && set.len() < limit {
                let config = batch.box_config.clone();
                let emitter = batch.event_emitter.clone();
                let shared_template = Arc::clone(batch.template);
                let shared_resources = Arc::clone(batch.template_resources);
                let boot_metrics = batch.metrics.clone();
                let pool_boot_limiter = batch.boot_limiter.clone();
                let daemon_boot_limiter = batch.global_boot_limiter.clone();
                let snapshot_fork = batch.snapshot_fork;
                let retained = batch.retained.clone();
                let shutdown = batch.shutdown.clone();
                set.spawn(async move {
                    if *shutdown.borrow() {
                        return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                    }
                    let _boot_permits =
                        acquire_boot_permits(pool_boot_limiter, daemon_boot_limiter).await?;
                    if *shutdown.borrow() {
                        return Err(BoxError::PoolError("Warm pool is shutting down".into()));
                    }
                    let _boot_guard = BootMetricGuard::new(boot_metrics);
                    WarmPool::boot_or_restore(
                        snapshot_fork,
                        &config,
                        &emitter,
                        &shared_template,
                        &shared_resources,
                        &retained,
                        &shutdown,
                    )
                    .await
                });
                launched += 1;
            }

            if let Some(result) = set.join_next().await {
                let result = match result {
                    Ok(result) => result,
                    Err(error) => Err(BoxError::PoolError(format!(
                        "Warm-pool boot task failed: {error}"
                    ))),
                };
                if result.is_err() {
                    if let Some(metrics) = &batch.metrics {
                        metrics.warm_pool_boot_failures_total.inc();
                    }
                }
                results.push(result);
            }
        }

        results
    }

    /// Get the snapshot-fork template, building it once lazily. Concurrent callers
    /// wait on the lock and reuse the first result — a built template OR a cached
    /// `Unavailable` verdict, so a failed build (native VM snapshot unsupported on
    /// this build) is attempted at most once rather than re-tried (and re-timed-out)
    /// on every pool fill. Returns `Err` when unavailable so `boot_or_restore` cold
    /// boots instead.
    async fn ensure_template(
        box_config: &BoxConfig,
        event_emitter: &EventEmitter,
        template: &Arc<Mutex<TemplateState>>,
        template_resources: &Arc<Mutex<TemplateResources>>,
    ) -> Result<PoolTemplate> {
        if !crate::vm::native_snapshot_fork_supported() {
            return Err(BoxError::PoolError(
                "snapshot-fork requires the Linux x86_64 KVM build".to_string(),
            ));
        }
        let mut guard = template.lock().await;
        let prior_failures = match &*guard {
            TemplateState::Ready(t) => return Ok(t.clone()),
            TemplateState::Unavailable => {
                return Err(BoxError::PoolError(
                    "snapshot-fork template unavailable (native VM snapshot unsupported)"
                        .to_string(),
                ));
            }
            // Unbuilt or a still-retryable prior failure: (re)attempt the build.
            TemplateState::Failing(n) => *n,
            TemplateState::Unbuilt => 0,
        };

        match Self::build_template(box_config, event_emitter, template_resources).await {
            Ok(tpl) => {
                *guard = TemplateState::Ready(tpl.clone());
                event_emitter.emit(BoxEvent::with_string(
                    "pool.template.built",
                    format!(
                        "Snapshot-fork template built for image {}",
                        box_config.image
                    ),
                ));
                Ok(tpl)
            }
            Err(error @ BoxError::ConfigError(_)) => Err(error),
            Err(error) => {
                // Feature-missing failures are permanent on this host/build —
                // do not burn MAX_TEMPLATE_BUILD_FAILURES × socket poll (#563).
                if template_error_is_feature_missing(&error) {
                    tracing::warn!(
                        %error,
                        "snapshot-fork unavailable on this host; marking template \
                         Unavailable — the warm pool will cold-boot"
                    );
                    *guard = TemplateState::Unavailable;
                    return Err(error);
                }
                // Bounded retry: a transient failure presents identically to
                // "snapshot unsupported", so only give up permanently after a few
                // consecutive failures rather than downgrading the pool to
                // cold-boot forever on a one-off hiccup.
                let failures = prior_failures + 1;
                if failures >= MAX_TEMPLATE_BUILD_FAILURES {
                    tracing::warn!(
                        %error, failures,
                        "snapshot-fork template build failed repeatedly; marking \
                         unavailable — the warm pool will cold-boot"
                    );
                    *guard = TemplateState::Unavailable;
                } else {
                    tracing::warn!(
                        %error, failures,
                        "snapshot-fork template build failed; will retry on a later fill"
                    );
                    *guard = TemplateState::Failing(failures);
                }
                Err(error)
            }
        }
    }

    /// Cold-boot one source VM with file-backed RAM + a trigger socket, snapshot it,
    /// and tear it down — leaving the RAM image + state file as the template.
    async fn build_template(
        box_config: &BoxConfig,
        event_emitter: &EventEmitter,
        template_resources: &Arc<Mutex<TemplateResources>>,
    ) -> Result<PoolTemplate> {
        let mut resources = template_resources.lock().await;
        if let Some(source) = resources.source.as_mut() {
            source
                .destroy_with_timeout(EPHEMERAL_DRAIN_TIMEOUT_MS)
                .await?;
            resources.source = None;
        }
        if let Some(directory) = &resources.directory {
            // A failed drain may leave consumers alive; never overwrite their RAM.
            if directory.exclusive_directory()?.is_none() {
                resources.directory = None;
            }
        }
        if resources.directory.is_none() {
            let parent = kbox_core::dirs_home().join("pool");
            resources.directory = Some(
                tokio::task::spawn_blocking(move || TemplateDirectory::create(&parent))
                    .await
                    .map_err(|error| {
                        BoxError::PoolError(format!("Template allocation task failed: {error}"))
                    })??,
            );
        }
        let directory =
            Arc::clone(resources.directory.as_ref().ok_or_else(|| {
                BoxError::PoolError("Template directory was not allocated".into())
            })?);
        let native_path = |name: &str| {
            directory
                .path()
                .join(name)
                .into_os_string()
                .into_string()
                .map_err(|_| {
                    BoxError::ConfigError("Snapshot paths must be UTF-8 for the native API".into())
                })
        };
        let mem_file = native_path("template.ram")?;
        let state_file = native_path("template.state")?;
        let socket = directory.path().join("template.sock");
        match tokio::fs::remove_file(&socket).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let mut config = box_config.clone();
        config.snapshot_mem_file = Some(mem_file.clone());
        config.snapshot_sock = Some(native_path("template.sock")?);
        config.restore_from = None;
        let mut source = VmManager::new(config, event_emitter.clone());
        source.template_use = Some(directory.claim()?);
        let source = resources.source.insert(source);
        source.boot().await?;
        let rootfs_cache_key = match source.current_rootfs_cache_key() {
            Ok(key) => key,
            Err(error) => {
                if source
                    .destroy_with_timeout(EPHEMERAL_DRAIN_TIMEOUT_MS)
                    .await
                    .is_ok()
                {
                    resources.source = None;
                }
                return Err(error);
            }
        };
        source.exec_client = None;
        source.exec_socket_path = None;
        tokio::time::sleep(SNAPSHOT_VSOCK_QUIESCE).await;
        let snapshot = Self::trigger_snapshot(&socket, std::path::Path::new(&state_file)).await;
        let finalized = source
            .destroy_with_timeout(EPHEMERAL_DRAIN_TIMEOUT_MS)
            .await;
        if finalized.is_ok() {
            resources.source = None;
        }
        snapshot?;
        finalized?;
        Ok(PoolTemplate {
            mem_file,
            state_file,
            rootfs_cache_key,
            directory,
        })
    }

    /// Permanently abandon a Ready snapshot-fork template and drain idle VMs so
    /// hollow restores cannot keep failing acquire while looking `idle>0` (#563).
    async fn invalidate_snapshot_fork_template(&self, reason: &str) {
        {
            let mut guard = self.template.lock().await;
            if !matches!(*guard, TemplateState::Ready(_)) {
                return;
            }
            *guard = TemplateState::Unavailable;
            tracing::warn!(
                %reason,
                "snapshot-fork template invalidated; warm pool will cold-boot for this daemon"
            );
            self.event_emitter.emit(BoxEvent::with_string(
                "pool.template.invalidated",
                format!("Snapshot-fork template invalidated: {reason}"),
            ));
        }
        let drained: Vec<WarmVm> = {
            let mut idle = self.idle.lock().await;
            let mut stats = self.stats.lock().await;
            let drained = idle.drain(..).collect();
            stats.idle_count = idle.len();
            Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
            drained
        };
        if !drained.is_empty() {
            if let Err(error) = Self::destroy_vms(
                drained,
                &self.retiring,
                Some(2000),
                "invalidate-snapshot-template",
                PoolCloseScope::Selected,
            )
            .await
            {
                self.signal_shutdown();
                tracing::warn!(%error, "Invalidated pool retained unfinished VMs");
            }
        }
    }

    /// Send a `snapshot <state>` request to libkrun's per-template trigger socket and
    /// wait for the `ok` reply (the socket appears once the template's vCPUs run).
    ///
    /// Snapshot-fork is a Linux/KVM (Unix) feature; on non-Unix hosts the trigger
    /// socket does not exist, so this is unavailable (see the `not(unix)` stub).
    #[cfg(unix)]
    async fn trigger_snapshot(sock: &std::path::Path, state_file: &std::path::Path) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        // The socket is bound by libkrun after the guest starts; poll briefly.
        // Cap well under the historical 5s busy-wait so a missing template
        // socket fails fast instead of taxing snapshot-fork fill.
        const SNAPSHOT_SOCKET_POLL_ATTEMPTS: u32 = 60;
        const SNAPSHOT_SOCKET_POLL_INTERVAL: std::time::Duration =
            std::time::Duration::from_millis(25);
        let mut stream = None;
        for _ in 0..SNAPSHOT_SOCKET_POLL_ATTEMPTS {
            match tokio::net::UnixStream::connect(sock).await {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_) => tokio::time::sleep(SNAPSHOT_SOCKET_POLL_INTERVAL).await,
            }
        }
        let mut stream = stream.ok_or_else(|| {
            BoxError::PoolError(format!("snapshot socket {} never appeared", sock.display()))
        })?;
        let cmd = format!("snapshot {}\n", state_file.display());
        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(BoxError::IoError)?;
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).await.map_err(BoxError::IoError)?;
        let reply = String::from_utf8_lossy(&buf[..n]);
        if reply.trim() == "ok" {
            Ok(())
        } else {
            Err(BoxError::PoolError(format!(
                "snapshot trigger failed: {}",
                reply.trim()
            )))
        }
    }

    /// Non-Unix stub: snapshot-fork relies on libkrun's Unix trigger socket and KVM
    /// state save/restore, neither of which exist on Windows. `--snapshot-fork` is
    /// Linux/KVM-only, so this path is never reached there in practice.
    #[cfg(not(unix))]
    async fn trigger_snapshot(
        _sock: &std::path::Path,
        _state_file: &std::path::Path,
    ) -> Result<()> {
        Err(BoxError::PoolError(
            "snapshot-fork is only supported on Linux/KVM hosts".to_string(),
        ))
    }

    /// Fill the pool to a specific idle target.
    async fn fill_to_target(&self, target: usize) {
        let current = self.idle.lock().await.len();
        let needed = target.saturating_sub(current);

        if needed == 0 {
            return;
        }

        tracing::debug!(current, needed, target, "Replenishing warm pool");

        // Track VMs added in this fill attempt so we can clean up on failure.
        let mut added_ids: Vec<String> = Vec::new();
        let mut failed = false;
        let results = Self::boot_batch(BootBatch {
            snapshot_fork: self.config.snapshot_fork,
            box_config: &self.box_config,
            event_emitter: &self.event_emitter,
            template: &self.template,
            template_resources: &self.template_resources,
            needed,
            max_concurrent_boots: self.config.max_concurrent_boots,
            metrics: self.metrics.clone(),
            boot_limiter: self.boot_limiter.clone(),
            global_boot_limiter: self.global_boot_limiter.clone(),
            retained: &self.retiring,
            shutdown: &self.shutdown_tx,
        })
        .await;

        for result in results {
            match result {
                Ok((owner, _handoff)) => {
                    let mut idle = self.idle.lock().await;
                    let mut stats = self.stats.lock().await;
                    if *self.shutdown_rx.borrow() {
                        drop(stats);
                        drop(idle);
                        let (_, closed) =
                            Self::destroy_one(owner, self.retiring.clone(), Some(2000)).await;
                        if let Err(error) = closed {
                            tracing::warn!(%error, "Completed fill VM retained for shutdown retry");
                        }
                        failed = true;
                        continue;
                    }
                    let vm = match self.take_foreground_vm(owner) {
                        Ok(vm) => vm,
                        Err(error) => {
                            failed = true;
                            tracing::warn!(%error, "Completed fill VM remains owned during cleanup");
                            continue;
                        }
                    };
                    let box_id = vm.box_id().to_owned();
                    idle.push(WarmVm {
                        vm,
                        created_at: Instant::now(),
                    });
                    Self::sync_idle_metric(self.metrics.as_ref(), idle.len());
                    stats.total_created += 1;
                    stats.idle_count = idle.len();
                    added_ids.push(box_id.clone());

                    self.event_emitter.emit(BoxEvent::with_string(
                        "pool.vm.created",
                        format!("Booted new VM {box_id}"),
                    ));
                    tracing::debug!(box_id = %box_id, "Added VM to warm pool");
                }
                Err(error) => {
                    failed = true;
                    tracing::warn!(error = %error, "Failed to boot VM for warm pool");
                }
            }
        }

        if failed && !added_ids.is_empty() {
            tracing::info!(
                count = added_ids.len(),
                "Cleaning up VMs added before pool fill failed"
            );
            self.remove_idle_vms(&added_ids).await;
        }

        self.event_emitter.emit(BoxEvent::empty("pool.replenish"));
    }

    /// Spawn the background maintenance loop.
    ///
    /// Periodically checks for:
    /// 1. Autoscaler evaluation → adjust min_idle dynamically
    /// 2. Pool below min_idle → replenish
    /// 3. Idle VMs past TTL → evict
    fn spawn_maintenance_loop(&self) -> JoinHandle<()> {
        let idle = Arc::clone(&self.idle);
        let stats = Arc::clone(&self.stats);
        let retiring = Arc::clone(&self.retiring);
        let config = self.config.clone();
        let box_config = self.box_config.clone();
        let event_emitter = self.event_emitter.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();
        let shutdown_tx = self.shutdown_tx.clone();
        let scaler = self.scaler.clone();
        let template = Arc::clone(&self.template);
        let template_resources = Arc::clone(&self.template_resources);
        let metrics = self.metrics.clone();
        let boot_limiter = self.boot_limiter.clone();
        let global_boot_limiter = self.global_boot_limiter.clone();

        tokio::spawn(async move {
            let check_interval = std::time::Duration::from_secs(
                // Check every 1/5 of TTL, minimum 5 seconds
                if config.idle_ttl_secs > 0 {
                    (config.idle_ttl_secs / 5).max(5)
                } else {
                    30
                },
            );
            let mut maintenance = tokio::time::interval(check_interval);
            // The first tick is immediate, which lets a first-ready lazy pool
            // continue filling without waiting for the full maintenance period.
            maintenance.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            // Dynamic min_idle starts from config, adjusted by scaler
            let mut effective_min_idle = config.min_idle;
            // A provider failure should not cause the maintenance loop to
            // repeatedly launch expensive doomed boots. Back off retries while
            // keeping the normal maintenance/eviction cadence unchanged.
            let mut replenish_failures = 0u32;
            let mut next_replenish_at = Instant::now();

            loop {
                tokio::select! {
                    result = shutdown_rx.changed() => {
                        if result.is_ok() && *shutdown_rx.borrow() {
                            tracing::debug!("Pool maintenance loop shutting down");
                            break;
                        }
                    }
                    _ = maintenance.tick() => {
                        // Evict expired VMs
                        if config.idle_ttl_secs > 0 {
                            if let Err(error) = Self::evict_expired_static(
                                &idle,
                                &stats,
                                &event_emitter,
                                metrics.as_ref(),
                                config.idle_ttl_secs,
                                &retiring,
                            ).await {
                                let _ = shutdown_tx.send(true);
                                tracing::warn!(%error, "Pool admission closed after eviction finalizer failure");
                                break;
                            }
                        }

                        // Evaluate autoscaler
                        if let Some(ref scaler) = scaler {
                            let mut s = scaler.lock().await;
                            let decision = s.evaluate();
                            let new_min = s.current_min_idle();
                            if new_min != effective_min_idle {
                                tracing::info!(
                                    old_min_idle = effective_min_idle,
                                    new_min_idle = new_min,
                                    ?decision,
                                    "Autoscaler adjusted min_idle"
                                );
                                event_emitter.emit(BoxEvent::with_string(
                                    "pool.autoscale",
                                    format!(
                                        "min_idle adjusted {} → {} ({:?})",
                                        effective_min_idle, new_min, decision
                                    ),
                                ));
                                effective_min_idle = new_min;
                            }
                        }

                        // Replenish if below effective min_idle
                        let current = idle.lock().await.len();
                        if current < effective_min_idle && Instant::now() >= next_replenish_at {
                            let needed = effective_min_idle - current;
                            tracing::debug!(current, needed, min_idle = effective_min_idle, "Replenishing warm pool");

                            // Overlap readiness waits while keeping the number of
                            // expensive VM boots bounded by configuration.
                            let results = Self::boot_batch(BootBatch {
                                snapshot_fork: config.snapshot_fork,
                                box_config: &box_config,
                                event_emitter: &event_emitter,
                                template: &template,
                                template_resources: &template_resources,
                                needed,
                                max_concurrent_boots: config.max_concurrent_boots,
                                metrics: metrics.clone(),
                                boot_limiter: boot_limiter.clone(),
                                global_boot_limiter: global_boot_limiter.clone(),
                                retained: &retiring,
                                shutdown: &shutdown_tx,
                            })
                            .await;
                            let mut batch_failed = false;
                            for result in results {
                                match result {
                                    Ok((owner, _handoff)) => {
                                        // If shutdown landed while this batch was
                                        // booting, drain_idle has already cleared
                                        // `idle` and will not run again, so a VM
                                        // pushed now leaks (no Drop reaper). Destroy
                                        // it instead. Acquire the idle lock FIRST and
                                        // re-check shutdown UNDER it: drain_idle drains
                                        // while holding this same lock (always after
                                        // signal_shutdown), so the check-and-push is
                                        // atomic against it — closing the TOCTOU window
                                        // that an unlocked `borrow()` check left open.
                                        let mut pool = idle.lock().await;
                                        let mut s = stats.lock().await;
                                        if *shutdown_rx.borrow() {
                                            drop(s);
                                            drop(pool);
                                            tracing::debug!(
                                                "Pool shutting down mid-replenish; destroying freshly-booted VM"
                                            );
                                            let (_, closed) = Self::destroy_one(owner, retiring.clone(), Some(2000)).await;
                                            if let Err(error) = closed {
                                                tracing::warn!(%error, "Late pool VM retained for shutdown retry");
                                            }
                                            continue;
                                        }
                                        let vm = match Self::take_registered_vm(owner, &retiring, &shutdown_tx) {
                                            Ok(vm) => vm,
                                            Err(error) => {
                                                batch_failed = true;
                                                tracing::warn!(%error, "Completed pool VM remains owned during cleanup");
                                                continue;
                                            }
                                        };
                                        let box_id = vm.box_id().to_owned();
                                        pool.push(WarmVm {
                                            vm,
                                            created_at: Instant::now(),
                                        });
                                        Self::sync_idle_metric(metrics.as_ref(), pool.len());
                                        s.total_created += 1;
                                        s.idle_count = pool.len();
                                        drop(s);
                                        drop(pool);

                                        event_emitter.emit(BoxEvent::with_string(
                                            "pool.vm.created",
                                            format!("Replenished VM {}", box_id),
                                        ));
                                    }
                                    Err(error) => {
                                        batch_failed = true;
                                        tracing::warn!(error = %error, "Failed to replenish warm pool");
                                    }
                                }
                            }

                            if batch_failed {
                                replenish_failures = replenish_failures.saturating_add(1);
                                let delay = replenish_backoff_delay(
                                    replenish_failures,
                                    check_interval,
                                );
                                next_replenish_at = Instant::now() + delay;
                                tracing::warn!(
                                    failures = replenish_failures,
                                    retry_in_secs = delay.as_secs(),
                                    "Backing off warm-pool replenishment after boot failure"
                                );
                            } else {
                                replenish_failures = 0;
                                next_replenish_at = Instant::now();
                            }

                            event_emitter.emit(BoxEvent::empty("pool.replenish"));
                        }
                    }
                }
            }
        })
    }

    /// Static version of evict_expired for use in the spawned task.
    async fn evict_expired_static(
        idle: &Arc<Mutex<Vec<WarmVm>>>,
        stats: &Arc<Mutex<PoolStats>>,
        event_emitter: &EventEmitter,
        metrics: Option<&crate::prom::RuntimeMetrics>,
        idle_ttl_secs: u64,
        retiring: &RetainedVms,
    ) -> Result<()> {
        let ttl = std::time::Duration::from_secs(idle_ttl_secs);

        let mut pool = idle.lock().await;
        let mut kept = Vec::new();
        let mut expired = Vec::new();

        for warm_vm in pool.drain(..) {
            if warm_vm.created_at.elapsed() > ttl {
                expired.push(warm_vm);
            } else {
                kept.push(warm_vm);
            }
        }
        *pool = kept;
        let after_count = pool.len();
        drop(pool);

        let evicted_count = expired.len();
        Self::sync_idle_metric(metrics, after_count);
        let finalized = Self::destroy_vms(
            expired,
            retiring,
            None,
            "eviction",
            PoolCloseScope::Selected,
        )
        .await;

        if evicted_count > 0 {
            let mut s = stats.lock().await;
            s.total_evicted += evicted_count as u64;
            s.idle_count = after_count;

            event_emitter.emit(BoxEvent::with_string(
                "pool.vm.evicted",
                format!("Evicted {} expired VMs", evicted_count),
            ));
        }
        finalized
    }
}

/// Return the number of boot tasks that may be in flight for one batch.
///
/// Configuration validation rejects zero for normal pool construction. The
/// defensive `max(1)` keeps this scheduler total for internal callers and
/// prevents a zero limit from deadlocking the JoinSet loop.
fn bounded_boot_limit(needed: usize, max_concurrent_boots: usize) -> usize {
    needed.min(max_concurrent_boots.max(1))
}

/// Calculate the retry delay after a failed background replenishment batch.
///
/// The regular maintenance tick remains responsible for eviction and scaling,
/// while only replenishment is delayed. Capping the delay keeps a transient
/// provider outage recoverable without allowing a permanently unavailable
/// backend to churn the host indefinitely.
fn replenish_backoff_delay(failures: u32, check_interval: Duration) -> Duration {
    let exponent = failures.saturating_sub(1).min(8);
    let multiplier = 1u64 << exponent;
    let delay_secs = check_interval
        .as_secs()
        .max(1)
        .saturating_mul(multiplier)
        .min(300);
    Duration::from_secs(delay_secs)
}

/// Classify template-build errors that mean snapshot-fork cannot work on this
/// host/build. These must mark `Unavailable` immediately (#563).
fn template_error_is_feature_missing(error: &BoxError) -> bool {
    let msg = error.to_string();
    msg.contains("snapshot socket") && msg.contains("never appeared")
        || msg.contains("snapshot-fork is only supported")
        || msg.contains("snapshot-fork requires the Linux")
        || msg.contains("native VM snapshot unsupported")
        || msg.contains("snapshot-fork template unavailable")
}

#[cfg(test)]
mod shutdown_tests;

#[cfg(test)]
#[path = "warm_pool/shutdown_ownership_tests.rs"]
mod shutdown_ownership_tests;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "warm_pool/shutdown_native.rs"]
mod shutdown_native;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "warm_pool/batch_native.rs"]
mod batch_native;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "warm_pool/rollback_native.rs"]
mod rollback_native;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "warm_pool/drain_native.rs"]
mod drain_native;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "warm_pool/boot_owner_native.rs"]
mod boot_owner_native;

#[cfg(test)]
#[path = "warm_pool/template_ownership_tests.rs"]
mod template_ownership_tests;

#[cfg(test)]
mod post_boot_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use kbox_core::config::PoolConfig;

    fn test_pool_config(min_idle: usize, max_size: usize) -> PoolConfig {
        PoolConfig {
            enabled: true,
            min_idle,
            max_size,
            idle_ttl_secs: 300,
            ..Default::default()
        }
    }

    fn test_event_emitter() -> EventEmitter {
        EventEmitter::new(100)
    }

    #[test]
    fn boot_or_restore_future_stays_heap_indirected() {
        let config = BoxConfig::default();
        let emitter = test_event_emitter();
        let template = Arc::new(Mutex::new(TemplateState::Unbuilt));
        let resources = Arc::new(Mutex::new(TemplateResources::default()));
        let retained = Arc::default();
        let (shutdown, _) = watch::channel(false);
        let future = WarmPool::boot_or_restore(
            false, &config, &emitter, &template, &resources, &retained, &shutdown,
        );

        assert!(
            std::mem::size_of_val(&future) <= 2 * std::mem::size_of::<usize>(),
            "boot_or_restore future must remain pointer-sized so pool misses fit on Tokio worker stacks; got {} bytes",
            std::mem::size_of_val(&future)
        );
    }

    #[test]
    fn feature_missing_classifies_absent_snapshot_socket() {
        let error = BoxError::PoolError(
            "snapshot socket /tmp/tpl/template.sock never appeared".to_string(),
        );
        assert!(template_error_is_feature_missing(&error));
    }

    #[test]
    fn feature_missing_classifies_platform_stubs() {
        assert!(template_error_is_feature_missing(&BoxError::PoolError(
            "snapshot-fork is only supported on Linux/KVM hosts".to_string(),
        )));
        assert!(template_error_is_feature_missing(&BoxError::PoolError(
            "snapshot-fork requires the Linux x86_64 KVM build".to_string(),
        )));
    }

    #[test]
    fn feature_missing_ignores_transient_boot_errors() {
        let error = BoxError::BoxBootError {
            message: "Failed to create socket directory".to_string(),
            hint: None,
        };
        assert!(!template_error_is_feature_missing(&error));
    }

    #[tokio::test]
    async fn snapshot_fork_auto_enables_deferred_main() {
        let pool_config = PoolConfig {
            enabled: true,
            min_idle: 0,
            max_size: 1,
            snapshot_fork: true,
            ..Default::default()
        };
        let box_config = BoxConfig {
            deferred_main: false,
            pool: pool_config.clone(),
            ..Default::default()
        };
        // min_idle 0 avoids booting; we only assert config normalization.
        let pool = WarmPool::start(pool_config, box_config, test_event_emitter())
            .await
            .unwrap();
        assert!(
            pool.box_config.deferred_main,
            "snapshot-fork must auto-enable deferred_main"
        );
        pool.signal_shutdown();
        let _ = pool.drain_idle().await;
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    #[tokio::test]
    async fn unsupported_snapshot_fork_is_rejected_before_template_construction() {
        let template = Arc::new(Mutex::new(TemplateState::Unbuilt));
        let resources = Arc::new(Mutex::new(TemplateResources::default()));
        let result = WarmPool::ensure_template(
            &BoxConfig::default(),
            &test_event_emitter(),
            &template,
            &resources,
        )
        .await;
        let error = match result {
            Ok(_) => panic!("unsupported host unexpectedly built a snapshot template"),
            Err(error) => error.to_string(),
        };

        assert!(error.contains("Linux x86_64 KVM"), "{error}");
        assert!(matches!(&*template.lock().await, TemplateState::Unbuilt));
        assert!(resources.lock().await.directory.is_none());
    }

    // --- PoolConfig validation tests ---

    #[tokio::test]
    async fn test_pool_rejects_zero_max_size() {
        let config = test_pool_config(0, 0);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        match result {
            Err(e) => assert!(e.to_string().contains("max_size must be greater than 0")),
            Ok(_) => panic!("Expected error for zero max_size"),
        }
    }

    #[tokio::test]
    async fn test_pool_rejects_min_idle_exceeds_max() {
        let config = test_pool_config(10, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        match result {
            Err(e) => assert!(e.to_string().contains("cannot exceed max_size")),
            Ok(_) => panic!("Expected error for min_idle > max_size"),
        }
    }

    #[tokio::test]
    async fn test_pool_rejects_zero_max_concurrent_boots() {
        let mut config = test_pool_config(0, 1);
        config.max_concurrent_boots = 0;
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        match result {
            Err(error) => assert!(error.to_string().contains("max_concurrent_boots")),
            Ok(_) => panic!("Expected error for zero max_concurrent_boots"),
        }
    }

    #[tokio::test]
    async fn acquire_boot_permits_releases_both_scopes() {
        let pool_limiter = Arc::new(Semaphore::new(1));
        let global_limiter = Arc::new(Semaphore::new(1));
        let (pool_permit, global_permit) =
            acquire_boot_permits(pool_limiter.clone(), Some(global_limiter.clone()))
                .await
                .expect("both boot limiters should grant a permit");
        assert_eq!(pool_limiter.available_permits(), 0);
        assert_eq!(global_limiter.available_permits(), 0);
        drop((pool_permit, global_permit));
        assert_eq!(pool_limiter.available_permits(), 1);
        assert_eq!(global_limiter.available_permits(), 1);
    }

    #[tokio::test]
    async fn warm_pool_acquire_refuses_ready_without_authenticated_heartbeat() {
        use crate::vm::BoxState;
        use crate::vmm::{VmHandler, VmMetrics};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct AlwaysRunningHandler {
            stop_calls: Arc<AtomicUsize>,
        }

        impl VmHandler for AlwaysRunningHandler {
            fn stop(&mut self, _signal: i32, _timeout_ms: u64) -> kbox_core::error::Result<()> {
                self.stop_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn metrics(&self) -> VmMetrics {
                VmMetrics::default()
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

        let pool = WarmPool::start(
            test_pool_config(0, 1),
            BoxConfig::default(),
            test_event_emitter(),
        )
        .await
        .expect("empty pool starts without a hypervisor");

        let vm = VmManager::new(BoxConfig::default(), test_event_emitter());
        *vm.state.write().await = BoxState::Ready;
        *vm.handler.write().await = Some(Box::new(AlwaysRunningHandler {
            stop_calls: Arc::new(AtomicUsize::new(0)),
        }));
        // No exec socket / client — health_check must fail closed (#419/#420).
        let stale_id = vm.box_id().to_string();
        pool.idle.lock().await.push(WarmVm {
            vm,
            created_at: Instant::now(),
        });
        assert_eq!(pool.idle_count().await, 1);

        let result = pool.acquire().await;
        match &result {
            Ok(vm) => {
                assert_ne!(
                    vm.box_id(),
                    stale_id,
                    "idle membership must not invent operable Ready without exec heartbeat"
                );
            }
            Err(_) => {
                // Expected on hosts without a hypervisor: stale idle destroyed,
                // on-demand boot miss fails closed.
            }
        }
        assert_eq!(
            pool.idle_count().await,
            0,
            "stale Ready idle VM must be destroyed, not left for the next lease"
        );
        assert!(
            pool.stats().await.total_evicted >= 1,
            "failed re-auth must count as eviction, not a pool hit"
        );
    }

    #[tokio::test]
    async fn warm_pool_release_refuses_idle_without_authenticated_heartbeat() {
        use crate::vm::BoxState;
        use crate::vmm::{VmHandler, VmMetrics};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct AlwaysRunningHandler {
            stop_calls: Arc<AtomicUsize>,
        }

        impl VmHandler for AlwaysRunningHandler {
            fn stop(&mut self, _signal: i32, _timeout_ms: u64) -> kbox_core::error::Result<()> {
                self.stop_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn metrics(&self) -> VmMetrics {
                VmMetrics::default()
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

        let pool = WarmPool::start(
            test_pool_config(0, 1),
            BoxConfig::default(),
            test_event_emitter(),
        )
        .await
        .expect("empty pool starts without a hypervisor");

        let vm = VmManager::new(BoxConfig::default(), test_event_emitter());
        *vm.state.write().await = BoxState::Ready;
        *vm.handler.write().await = Some(Box::new(AlwaysRunningHandler {
            stop_calls: Arc::new(AtomicUsize::new(0)),
        }));
        // In-memory Ready + running handler without exec heartbeat must not
        // invent idle membership (#429 / #423/#419).
        pool.release(vm)
            .await
            .expect("unauthenticated release destroys and returns Ok");
        assert_eq!(
            pool.idle_count().await,
            0,
            "release must destroy, not idle, without authenticated exec heartbeat"
        );
        assert_eq!(
            pool.stats().await.total_released,
            0,
            "failed re-auth must not count as a pool release"
        );
        assert!(
            pool.stats().await.total_evicted >= 1,
            "failed release re-auth must count as eviction"
        );
    }

    #[test]
    fn boot_batch_limit_is_bounded_and_never_deadlocks() {
        assert_eq!(bounded_boot_limit(0, 2), 0);
        assert_eq!(bounded_boot_limit(8, 2), 2);
        assert_eq!(bounded_boot_limit(2, 8), 2);
        assert_eq!(bounded_boot_limit(8, 0), 1);
    }

    #[test]
    fn replenish_backoff_is_exponential_and_capped() {
        let base = Duration::from_secs(5);
        assert_eq!(replenish_backoff_delay(0, base), Duration::from_secs(5));
        assert_eq!(replenish_backoff_delay(1, base), Duration::from_secs(5));
        assert_eq!(replenish_backoff_delay(2, base), Duration::from_secs(10));
        assert_eq!(replenish_backoff_delay(7, base), Duration::from_secs(300));
        assert_eq!(replenish_backoff_delay(20, base), Duration::from_secs(300));
    }

    #[test]
    fn boot_metric_guard_balances_inflight_gauge() {
        let metrics = crate::prom::RuntimeMetrics::new();
        {
            let _guard = BootMetricGuard::new(Some(metrics.clone()));
            assert_eq!(metrics.warm_pool_boots_inflight.get(), 1);
        }
        assert_eq!(metrics.warm_pool_boots_inflight.get(), 0);
    }

    // --- PoolStats tests ---

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats {
            idle_count: 0,
            total_created: 0,
            total_acquired: 0,
            total_released: 0,
            total_evicted: 0,
        };
        assert_eq!(stats.idle_count, 0);
        assert_eq!(stats.total_created, 0);
    }

    #[test]
    fn test_pool_stats_clone() {
        let stats = PoolStats {
            idle_count: 3,
            total_created: 10,
            total_acquired: 7,
            total_released: 5,
            total_evicted: 2,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.idle_count, 3);
        assert_eq!(cloned.total_created, 10);
        assert_eq!(cloned.total_acquired, 7);
        assert_eq!(cloned.total_released, 5);
        assert_eq!(cloned.total_evicted, 2);
    }

    #[test]
    fn test_pool_stats_debug() {
        let stats = PoolStats {
            idle_count: 1,
            total_created: 2,
            total_acquired: 3,
            total_released: 4,
            total_evicted: 5,
        };
        let debug = format!("{:?}", stats);
        assert!(debug.contains("idle_count"));
        assert!(debug.contains("total_created"));
    }

    // --- PoolConfig serialization tests ---

    #[test]
    fn test_pool_config_roundtrip() {
        let config = PoolConfig {
            enabled: true,
            min_idle: 3,
            max_size: 10,
            idle_ttl_secs: 600,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PoolConfig = serde_json::from_str(&json).unwrap();

        assert!(parsed.enabled);
        assert_eq!(parsed.min_idle, 3);
        assert_eq!(parsed.max_size, 10);
        assert_eq!(parsed.idle_ttl_secs, 600);
    }

    #[test]
    fn test_pool_config_default_values() {
        let config = PoolConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.min_idle, 1);
        assert_eq!(config.max_size, 5);
        assert_eq!(config.idle_ttl_secs, 300);
    }

    #[test]
    fn test_pool_config_deserialization_with_defaults() {
        let json = r#"{"enabled": true}"#;
        let config: PoolConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.min_idle, 1);
        assert_eq!(config.max_size, 5);
        assert_eq!(config.idle_ttl_secs, 300);
    }

    // --- PoolConfig validation edge cases ---

    #[tokio::test]
    async fn test_pool_accepts_min_idle_equals_max() {
        let config = test_pool_config(3, 3);
        // This should be accepted (min_idle == max_size is valid)
        // It will fail at boot (no shim), but config validation should pass
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        // The error should be about VM boot, not config validation
        match result {
            Err(e) => assert!(!e.to_string().contains("cannot exceed max_size")),
            Ok(mut pool) => {
                let _ = pool.drain().await;
            }
        }
    }

    #[tokio::test]
    async fn test_pool_accepts_min_idle_zero() {
        let config = test_pool_config(0, 5);
        // min_idle=0 means no pre-warming, should be valid
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        match result {
            Ok(mut pool) => {
                // Pool should start with 0 idle VMs
                assert_eq!(pool.idle_count().await, 0);
                let stats = pool.stats().await;
                assert_eq!(stats.idle_count, 0);
                assert_eq!(stats.total_created, 0);
                let _ = pool.drain().await;
            }
            Err(e) => {
                // If it fails, it should NOT be a config validation error
                assert!(!e.to_string().contains("max_size"));
                assert!(!e.to_string().contains("min_idle"));
            }
        }
    }

    // --- WarmPool internal state tests (using min_idle=0 to avoid boot) ---

    #[tokio::test]
    async fn test_pool_stats_initial() {
        let config = test_pool_config(0, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        if let Ok(mut pool) = result {
            let stats = pool.stats().await;
            assert_eq!(stats.idle_count, 0);
            assert_eq!(stats.total_created, 0);
            assert_eq!(stats.total_acquired, 0);
            assert_eq!(stats.total_released, 0);
            assert_eq!(stats.total_evicted, 0);
            let _ = pool.drain().await;
        }
    }

    #[tokio::test]
    async fn test_pool_idle_count_initial() {
        let config = test_pool_config(0, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        if let Ok(mut pool) = result {
            assert_eq!(pool.idle_count().await, 0);
            let _ = pool.drain().await;
        }
    }

    #[tokio::test]
    async fn test_pool_drain_empty_pool() {
        let config = test_pool_config(0, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        if let Ok(mut pool) = result {
            // Draining an empty pool should succeed without error
            let drain_result = pool.drain().await;
            assert!(drain_result.is_ok());

            let stats = pool.stats().await;
            assert_eq!(stats.idle_count, 0);
        }
    }

    #[tokio::test]
    async fn test_pool_drain_emits_event() {
        let emitter = test_event_emitter();
        let mut receiver = emitter.subscribe();
        let config = test_pool_config(0, 5);

        let result = WarmPool::start(config, BoxConfig::default(), emitter).await;
        if let Ok(mut pool) = result {
            pool.drain().await.unwrap();

            // Check that pool.drained event was emitted
            let mut found_drain_event = false;
            // Drain all events from the receiver
            while let Ok(event) = receiver.try_recv() {
                if event.key == "pool.drained" {
                    found_drain_event = true;
                }
            }
            assert!(found_drain_event, "Expected pool.drained event");
        }
    }

    #[tokio::test]
    async fn test_pool_acquire_from_empty_pool_fails_without_shim() {
        let config = test_pool_config(0, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        if let Ok(pool) = result {
            // Acquire from empty pool should try to boot a VM, which will fail
            // because there's no shim binary available in test environment
            let acquire_result = pool.acquire().await;
            assert!(acquire_result.is_err());
        }
    }

    // --- Maintenance loop check interval calculation ---

    #[test]
    #[allow(clippy::unnecessary_min_or_max)]
    fn test_maintenance_check_interval_with_ttl() {
        // TTL = 300s → check every 60s (300/5)
        let interval = if 300_u64 > 0 {
            (300_u64 / 5).max(5)
        } else {
            30
        };
        assert_eq!(interval, 60);
    }

    #[test]
    #[allow(clippy::unnecessary_min_or_max)]
    fn test_maintenance_check_interval_short_ttl() {
        // TTL = 10s → check every 5s (min 5)
        let interval = if 10_u64 > 0 { (10_u64 / 5).max(5) } else { 30 };
        assert_eq!(interval, 5);
    }

    #[test]
    #[allow(clippy::unnecessary_min_or_max)]
    fn test_maintenance_check_interval_very_short_ttl() {
        // TTL = 1s → check every 5s (min 5)
        let interval = if 1_u64 > 0 { (1_u64 / 5).max(5) } else { 30 };
        assert_eq!(interval, 5);
    }

    #[test]
    #[allow(
        clippy::absurd_extreme_comparisons,
        clippy::erasing_op,
        clippy::unnecessary_min_or_max,
        unused_comparisons
    )]
    fn test_maintenance_check_interval_no_ttl() {
        // TTL = 0 → check every 30s
        let interval = if 0_u64 > 0 { (0_u64 / 5).max(5) } else { 30 };
        assert_eq!(interval, 30);
    }

    // --- WarmVm struct tests ---

    #[test]
    fn test_warm_vm_created_at_is_recent() {
        let before = Instant::now();
        let created_at = Instant::now();
        let after = Instant::now();

        assert!(created_at >= before);
        assert!(created_at <= after);
    }

    // --- PoolStats field coverage ---

    #[test]
    fn test_pool_stats_all_fields() {
        let stats = PoolStats {
            idle_count: 10,
            total_created: 100,
            total_acquired: 80,
            total_released: 70,
            total_evicted: 15,
        };

        assert_eq!(stats.idle_count, 10);
        assert_eq!(stats.total_created, 100);
        assert_eq!(stats.total_acquired, 80);
        assert_eq!(stats.total_released, 70);
        assert_eq!(stats.total_evicted, 15);

        // Verify debug output contains all fields
        let debug = format!("{:?}", stats);
        assert!(debug.contains("10"));
        assert!(debug.contains("100"));
        assert!(debug.contains("80"));
        assert!(debug.contains("70"));
        assert!(debug.contains("15"));
    }

    // Note: Full integration tests for acquire/release/drain with actual VMs
    // require a working VM runtime (shim binary + libkrun). These are tested
    // in integration tests with the full box environment. The unit tests here
    // validate configuration, statistics, error handling, and pool lifecycle
    // with min_idle=0 (no VM boot required).

    #[tokio::test]
    async fn test_pool_set_metrics_attaches() {
        let config = test_pool_config(0, 5);
        let result = WarmPool::start(config, BoxConfig::default(), test_event_emitter()).await;
        match result {
            Ok(mut pool) => {
                let metrics = crate::prom::RuntimeMetrics::new();
                pool.set_metrics(metrics.clone());
                assert!(pool.metrics.is_some());
                // Metrics start at zero
                assert_eq!(metrics.warm_pool_hits.get(), 0);
                assert_eq!(metrics.warm_pool_misses.get(), 0);
                assert_eq!(metrics.warm_pool_size.get(), 0);
                let _ = pool.drain().await;
            }
            Err(_) => {
                // Boot failure is acceptable in unit test environment
            }
        }
    }

    #[tokio::test]
    async fn test_pool_start_with_metrics_installs_sink_before_fill() {
        let config = test_pool_config(0, 5);
        let metrics = crate::prom::RuntimeMetrics::new();
        let result = WarmPool::start_with_metrics(
            config,
            BoxConfig::default(),
            test_event_emitter(),
            Some(metrics.clone()),
        )
        .await;

        match result {
            Ok(mut pool) => {
                assert!(pool.metrics.is_some());
                assert_eq!(metrics.warm_pool_capacity.get(), 5);
                assert_eq!(
                    metrics.warm_pool_initial_fill_duration.get_sample_count(),
                    1
                );
                let _ = pool.drain().await;
            }
            Err(_) => {
                // Boot failure is acceptable in unit test environments without
                // a usable VM provider; min_idle=0 normally avoids this path.
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trigger_snapshot_fails_fast_when_socket_never_appears() {
        use std::time::{Duration, Instant};

        let tmp = tempfile::tempdir().unwrap();
        let sock = tmp.path().join("missing-template.sock");
        let state = tmp.path().join("template.state");

        let started = Instant::now();
        let error = WarmPool::trigger_snapshot(&sock, &state)
            .await
            .expect_err("missing snapshot socket must fail");
        assert!(error.to_string().contains("never appeared"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "missing snapshot socket must not busy-wait the historical ~5s window"
        );
    }

    #[tokio::test]
    async fn clear_snapshot_template_removes_ready_template_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let directory = TemplateDirectory::create(tmp.path()).unwrap();
        let dir = directory.path().to_path_buf();
        let mem_file = dir.join("template.ram");
        let state_file = dir.join("template.state");
        std::fs::write(&mem_file, b"ram").unwrap();
        std::fs::write(&state_file, b"state").unwrap();

        let config = test_pool_config(0, 1);
        let mut pool = WarmPool::start(config, BoxConfig::default(), test_event_emitter())
            .await
            .unwrap();
        {
            let mut template = pool.template.lock().await;
            *template = TemplateState::Ready(PoolTemplate {
                mem_file: mem_file.to_string_lossy().into_owned(),
                state_file: state_file.to_string_lossy().into_owned(),
                rootfs_cache_key: None,
                directory: Arc::clone(&directory),
            });
        }
        pool.template_resources.lock().await.directory = Some(directory);

        pool.clear_snapshot_template().await.unwrap();
        assert!(
            !dir.exists(),
            "drain must remove the on-disk snapshot-fork template directory"
        );
        assert!(matches!(
            &*pool.template.lock().await,
            TemplateState::Unbuilt
        ));
        pool.drain().await.unwrap();
    }

    #[tokio::test]
    async fn clear_snapshot_template_removes_owned_dir_when_failing() {
        let config = test_pool_config(0, 1);
        let box_config = BoxConfig {
            image: "alpine:clear-tpl-failing".into(),
            ..BoxConfig::default()
        };
        let mut pool = WarmPool::start(config, box_config, test_event_emitter())
            .await
            .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let directory = TemplateDirectory::create(tmp.path()).unwrap();
        let dir = directory.path().to_path_buf();
        let caller = tmp.path().join("caller-template");
        std::fs::create_dir(&caller).unwrap();
        std::fs::write(caller.join("canary"), b"caller-owned").unwrap();
        std::fs::write(dir.join("template.ram"), b"ram").unwrap();
        pool.template_resources.lock().await.directory = Some(directory);
        {
            let mut template = pool.template.lock().await;
            *template = TemplateState::Failing(1);
        }

        pool.clear_snapshot_template().await.unwrap();
        assert!(
            !dir.exists(),
            "Failing template state must still remove its owned tpl directory"
        );
        assert_eq!(
            std::fs::read(caller.join("canary")).unwrap(),
            b"caller-owned"
        );
        assert!(matches!(
            &*pool.template.lock().await,
            TemplateState::Unbuilt
        ));
        pool.drain().await.unwrap();
    }
}
