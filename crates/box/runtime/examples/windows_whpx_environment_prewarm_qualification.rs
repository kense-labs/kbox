//! Windows WHPX environment-prewarm qualification (Phase 1 vertical slice).
//!
//! Measures cold create+start vs ensure/start_prewarmed + claim_prewarmed on the
//! same LocalExecutionManager / WHPX OCI path. Does **not** use HCS, LCOW, pool
//! UDS, or process isolation.
//!
//! Required absolute env (same family as windows-whpx-oci-qualification):
//! - `K_BOX_WHPX_PREWARM_QUALIFICATION=1`
//! - `K_HOME`, `K_BOX_OCI_HOST_ROOT`, `K_BOX_OCI_WHPX_ENDPOINT`
//! - `K_BOX_WHPX_OCI_IMAGE` — digest-pinned (`name@sha256:…`) idle-capable image
//! - `K_BOX_WHPX_PREWARM_REPORT` — absolute JSON report path
//!
//! Optional box-owned host: `K_BOX_WHPX_OCI_BOX_OWNED=1` plus
//! `K_BOX_WHPX_OCI_SERVICE_{ROOT,BIN,SHIM,VM_ROOTFS,MANIFEST}`.

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
fn main() {
    eprintln!("windows-whpx-environment-prewarm-qualification requires Windows x86_64");
    std::process::exit(2);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("prewarm qualification failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::Instant;

    use kbox_core::{
        BoxConfig, CreateExecutionRequest, ExecutionManager, NetworkMode, OperationId,
        ResourceConfig,
    };
    use kbox_runtime::{LocalExecutionManager, WindowsWhpxOciMigrationConfig};
    use serde::Serialize;

    const ENABLE: &str = "K_BOX_WHPX_PREWARM_QUALIFICATION";
    const REPORT: &str = "K_BOX_WHPX_PREWARM_REPORT";
    const SCHEMA: &str = "kbox.windows-whpx-environment-prewarm.v1";

    if std::env::var_os(ENABLE).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Err(format!("{ENABLE}=1 is required (refusing accidental runs)").into());
    }

    let home = abs_env("K_HOME")?;
    let state_path = home.join("boxes.json");
    let runtime_root = abs_env("K_BOX_OCI_HOST_ROOT")?;
    let endpoint = std::env::var("K_BOX_OCI_WHPX_ENDPOINT")
        .map_err(|_| "K_BOX_OCI_WHPX_ENDPOINT must be set")?;
    let image =
        std::env::var("K_BOX_WHPX_OCI_IMAGE").map_err(|_| "K_BOX_WHPX_OCI_IMAGE must be set")?;
    if !image.contains("@sha256:") {
        return Err("K_BOX_WHPX_OCI_IMAGE must be digest-pinned (name@sha256:…)".into());
    }
    let report_path = abs_env(REPORT)?;

    let box_owned =
        std::env::var_os("K_BOX_WHPX_OCI_BOX_OWNED").as_deref() == Some(std::ffi::OsStr::new("1"));

    let mut config = WindowsWhpxOciMigrationConfig::new(runtime_root.clone(), endpoint.clone())?;
    if box_owned {
        config = config.with_box_owned_owner(
            abs_env("K_BOX_WHPX_OCI_SERVICE_ROOT")?,
            abs_env("K_BOX_WHPX_OCI_SERVICE_BIN")?,
            abs_env("K_BOX_WHPX_OCI_SERVICE_SHIM")?,
            abs_env("K_BOX_WHPX_OCI_SERVICE_VM_ROOTFS")?,
            abs_env("K_BOX_WHPX_OCI_SERVICE_MANIFEST")?,
        )?;
    }

    let manager =
        LocalExecutionManager::with_windows_whpx_oci_qualification(&state_path, &home, config)
            .await?;

    let idle = |name: &str| -> CreateExecutionRequest {
        CreateExecutionRequest {
            external_sandbox_id: name.to_string(),
            config: BoxConfig {
                image: image.clone(),
                resources: ResourceConfig {
                    vcpus: 1,
                    memory_mb: 512,
                    ..Default::default()
                },
                entrypoint_override: Some(vec!["/bin/sleep".into()]),
                cmd: vec!["infinity".into()],
                network: NetworkMode::None,
                persistent: false,
                ..Default::default()
            },
            labels: BTreeMap::from([("purpose".into(), "windows-whpx-environment-prewarm".into())]),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        }
    };

    // --- cold path ---
    let cold_op = OperationId::new(format!(
        "prewarm-cold-{}",
        chrono::Utc::now().timestamp_millis()
    ))?;
    let cold_t0 = Instant::now();
    let cold_res = manager.create(idle("prewarm-cold"), &cold_op).await?;
    manager
        .start(&cold_res.execution_id, cold_res.generation)
        .await?;
    let cold_ms = cold_t0.elapsed().as_millis() as u64;
    manager
        .kill(&cold_res.execution_id, cold_res.generation)
        .await?;
    manager
        .remove(&cold_res.execution_id, cold_res.generation)
        .await?;

    // --- prewarm + claim ---
    let warm_op = OperationId::new(format!(
        "prewarm-warm-{}",
        chrono::Utc::now().timestamp_millis()
    ))?;
    let warm = manager
        .ensure_prewarmed_reservation(idle("prewarm-warm"), &warm_op, "phase1", "phase1-owner")
        .await?;
    // Same public start path as local_execution prewarm unit tests (start_prewarmed is crate-private).
    let boot_t0 = Instant::now();
    manager.start(&warm.execution_id, warm.generation).await?;
    let prewarm_boot_ms = boot_t0.elapsed().as_millis() as u64;

    let claim_op = OperationId::new(format!(
        "prewarm-claim-{}",
        chrono::Utc::now().timestamp_millis()
    ))?;
    let claim_t0 = Instant::now();
    let claimed = manager
        .claim_prewarmed(idle("prewarm-claim"), &claim_op, "phase1")
        .await?
        .ok_or("claim_prewarmed returned None (no compatible Running prewarm)")?;
    let claim_ms = claim_t0.elapsed().as_millis() as u64;

    let same_vm = claimed.execution_id == warm.execution_id;
    let gen_advanced = claimed.generation.get() > warm.generation.get();

    manager
        .kill(&claimed.execution_id, claimed.generation)
        .await?;
    manager
        .remove(&claimed.execution_id, claimed.generation)
        .await?;

    #[derive(Serialize)]
    struct Report {
        schema_version: &'static str,
        status: &'static str,
        home_dir: PathBuf,
        image: String,
        cold_create_start_ms: u64,
        prewarm_boot_ms: u64,
        claim_ms: u64,
        same_execution_id: bool,
        generation_advanced: bool,
        note: &'static str,
    }

    let ok = same_vm && gen_advanced;
    let report = Report {
        schema_version: SCHEMA,
        status: if ok { "passed" } else { "failed" },
        home_dir: home,
        image,
        cold_create_start_ms: cold_ms,
        prewarm_boot_ms,
        claim_ms,
        same_execution_id: same_vm,
        generation_advanced: gen_advanced,
        note: "Does not promote support-matrix readiness; baseline host WHPX only; no HCS/LCOW",
    };

    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !ok {
        return Err("prewarm claim invariants failed".into());
    }
    Ok(())
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn abs_env(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    use std::path::{Component, PathBuf};
    let path = PathBuf::from(std::env::var_os(name).ok_or_else(|| format!("{name} must be set"))?);
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(format!("{name} must be absolute and normalized").into());
    }
    Ok(path)
}
