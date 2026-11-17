//! Compose adapter for Box's single transient Secret materialization boundary.
//!
//! Sandbox (default) projects Secret files onto a private provider-owned disk
//! directory under `K_HOME` and bind-mounts them into the fused engine.
//! MicroVM keeps the memory-backed root + `configure_vm` staging path.

use kbox_core::compose::{ComposeConfig, ServiceConfig};
use kbox_core::config::{BoxConfig, ExecutionBoundary};

const SECRET_SCOPE_MICROVM: &str = "compose";
const SECRET_SCOPE_SANDBOX: &str = "compose-sandbox";

fn uses_secrets(config: &ComposeConfig) -> bool {
    config
        .services
        .values()
        .any(|service| !service.secret_environment.is_empty())
}

fn material_from_process_environment(
    source: &str,
) -> Result<kbox_runtime::BoxSecretMaterial, Box<dyn std::error::Error>> {
    let value = std::env::var(source).map_err(|_| {
        format!(
            "Compose Secret source environment variable {source:?} is unset or is not valid UTF-8"
        )
    })?;
    kbox_runtime::BoxSecretMaterial::new(value.into_bytes()).map_err(|error| error.into())
}

fn sandbox_projection(boundary: ExecutionBoundary) -> bool {
    matches!(boundary, ExecutionBoundary::Sandbox)
}

async fn scoped_store(
    for_sandbox: bool,
) -> Result<kbox_runtime::BoxTransientSecretStore, Box<dyn std::error::Error>> {
    let root = kbox_core::dirs_home().join("runtime-secrets");
    if for_sandbox {
        let store = kbox_runtime::BoxTransientSecretStore::prepare_private_disk_root(root).await?;
        Ok(store.private_scope(SECRET_SCOPE_SANDBOX).await?)
    } else {
        Ok(kbox_runtime::BoxTransientSecretStore::new(root)
            .private_scope(SECRET_SCOPE_MICROVM)
            .await?)
    }
}

/// Fail before network, state, or VM mutation when Secret prerequisites are
/// unavailable. Values are immediately wrapped in zeroizing memory and dropped.
///
/// Default Sandbox compose validates private-disk readiness under `K_HOME`.
/// MicroVM isolation still requires a memory-backed Secret root.
pub(super) async fn preflight(config: &ComposeConfig) -> Result<(), Box<dyn std::error::Error>> {
    if !uses_secrets(config) {
        return Ok(());
    }

    // Compose services inherit `ExecutionBoundary::default()` (Sandbox) unless a
    // future compose isolation selector pins MicroVM. Preflight the Sandbox
    // private-disk path so darwin hosts are not blocked by tmpfs/ram://.
    let _store = scoped_store(true).await.map_err(|error| {
        format!("Compose Sandbox Secret environment projection is unavailable: {error}")
    })?;
    let sources = config
        .services
        .values()
        .flat_map(|service| service.secret_environment.values())
        .collect::<std::collections::BTreeSet<_>>();
    for source in sources {
        let _material = material_from_process_environment(source)?;
    }
    Ok(())
}

pub(super) struct ComposeSecretLease {
    store: kbox_runtime::BoxTransientSecretStore,
    identity: String,
    armed: bool,
    /// When true, MicroVM may stage mounts via `configure_vm`.
    memory_backed: bool,
}

impl ComposeSecretLease {
    pub(super) fn identity(&self) -> &str {
        &self.identity
    }

    #[allow(dead_code)] // retained for lease introspection / tests
    pub(super) fn secret_root(&self) -> &std::path::Path {
        self.store.root()
    }

    pub(super) fn configure_vm(
        &self,
        manager: &mut kbox_runtime::VmManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.memory_backed {
            return Err(
                "Compose Sandbox Secret material cannot configure MicroVM Secret staging".into(),
            );
        }
        self.store.configure_vm(manager)?;
        Ok(())
    }

    /// Transfer cleanup ownership to the persisted box record.
    pub(super) fn persist(&mut self) {
        self.armed = false;
    }
}

impl Drop for ComposeSecretLease {
    fn drop(&mut self) {
        if self.armed {
            if let Err(error) = self.store.cleanup_identity_sync(&self.identity) {
                tracing::warn!(
                    error = %error,
                    "Failed to clean unregistered Compose transient Secret material"
                );
            }
        }
    }
}

/// Resolve and project one service immediately before its box lifecycle begins.
pub(super) async fn project_service(
    service: &ServiceConfig,
    box_id: &str,
    box_config: &mut BoxConfig,
) -> Result<Option<ComposeSecretLease>, Box<dyn std::error::Error>> {
    if service.secret_environment.is_empty() {
        return Ok(None);
    }

    use sha2::{Digest, Sha256};

    let for_sandbox = sandbox_projection(box_config.execution_boundary);
    let store = scoped_store(for_sandbox).await.map_err(|error| {
        if for_sandbox {
            format!("Compose Sandbox Secret environment projection failed: {error}")
        } else {
            format!(
                "Compose MicroVM Secret environment projection requires a memory-backed Secret root: {error}"
            )
        }
    })?;
    let identity = format!("sha256:{}", hex::encode(Sha256::digest(box_id.as_bytes())));
    let mut references = service.secret_environment.iter().collect::<Vec<_>>();
    references.sort_by(|left, right| left.0.cmp(right.0));
    let mut bindings = Vec::with_capacity(references.len());
    for (target, source) in references {
        bindings.push((target.clone(), material_from_process_environment(source)?));
    }
    let projection = store.materialize_environment(&identity, bindings).await?;
    box_config.volumes.extend(projection.volumes);
    box_config.extra_env.push((
        kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST.to_string(),
        projection.manifest,
    ));
    Ok(Some(ComposeSecretLease {
        store,
        identity,
        armed: true,
        memory_backed: !for_sandbox,
    }))
}

pub(crate) fn cleanup_persisted(identity: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Sandbox and MicroVM use distinct scopes under the same home root.
    let home = kbox_core::dirs_home().join("runtime-secrets");
    let sandbox_root = home.join(SECRET_SCOPE_SANDBOX);
    if sandbox_root.exists() {
        let canonical = sandbox_root.canonicalize()?;
        kbox_runtime::BoxTransientSecretStore::for_private_disk(canonical)
            .cleanup_identity_sync(identity)?;
    }
    let microvm_root = home.join(SECRET_SCOPE_MICROVM);
    if microvm_root.exists() {
        let canonical = match microvm_root.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        match kbox_runtime::BoxTransientSecretStore::new(canonical).cleanup_identity_sync(identity)
        {
            Ok(()) => {}
            // Darwin Sandbox-only hosts leave a non-memory directory absent MicroVM use.
            Err(error)
                if error.to_string().contains("memory-backed")
                    || error.to_string().contains("tmpfs")
                    || error.to_string().contains("ram://") => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kbox_core::compose::ServiceConfig;
    use std::collections::HashMap;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn service_with_secret(source: &str) -> ServiceConfig {
        let mut secret_environment = HashMap::new();
        secret_environment.insert("API_TOKEN".into(), source.into());
        ServiceConfig {
            secret_environment,
            ..Default::default()
        }
    }

    #[test]
    fn sandbox_project_service_binds_private_disk_without_microvm() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous_home = std::env::var_os("K_HOME");
        let previous_secret = std::env::var_os("KBOX_TEST_COMPOSE_SECRET");
        let home = tempfile::tempdir().unwrap();
        // SAFETY: test-local env under ENV_LOCK; restored before unlock.
        unsafe {
            std::env::set_var("K_HOME", home.path());
            std::env::set_var("KBOX_TEST_COMPOSE_SECRET", "compose-secret-fixture");
        }

        let service = service_with_secret("KBOX_TEST_COMPOSE_SECRET");
        let mut box_config = BoxConfig {
            execution_boundary: ExecutionBoundary::Sandbox,
            ..Default::default()
        };
        // block_on keeps the std Mutex across the call without an async await point.
        let lease = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(project_service(
                &service,
                "sandbox-secret-box",
                &mut box_config,
            ))
            .expect("sandbox Secret projection")
            .expect("lease");

        assert!(
            box_config.extra_env.iter().any(|(key, value)| {
                key == kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST
                    && !value.contains("compose-secret-fixture")
            }),
            "manifest must be present and must not contain plaintext"
        );
        assert_eq!(box_config.volumes.len(), 1);
        assert!(
            box_config.volumes[0].ends_with(":ro"),
            "Secret volume must be read-only"
        );
        let home_canonical = home.path().canonicalize().unwrap();
        assert!(
            lease.secret_root().starts_with(&home_canonical),
            "Sandbox Secret root must live under K_HOME ({:?} vs {:?})",
            lease.secret_root(),
            home_canonical
        );
        assert!(
            lease
                .configure_vm(&mut kbox_runtime::VmManager::new(
                    BoxConfig::default(),
                    kbox_core::event::EventEmitter::new(8),
                ))
                .is_err(),
            "Sandbox Secret lease must refuse MicroVM configure_vm"
        );

        drop(lease);
        if let Some(value) = previous_home {
            // SAFETY: restore process env under ENV_LOCK after the test body.
            unsafe { std::env::set_var("K_HOME", value) };
        } else {
            // SAFETY: restore process env under ENV_LOCK after the test body.
            unsafe { std::env::remove_var("K_HOME") };
        }
        if let Some(value) = previous_secret {
            // SAFETY: restore process env under ENV_LOCK after the test body.
            unsafe { std::env::set_var("KBOX_TEST_COMPOSE_SECRET", value) };
        } else {
            // SAFETY: restore process env under ENV_LOCK after the test body.
            unsafe { std::env::remove_var("KBOX_TEST_COMPOSE_SECRET") };
        }
    }
}
