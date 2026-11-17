//! Library surface for the Spec050 ExecutionManager sandbox adapter.
//!
//! The CLI binary (`main.rs`) remains the docker-shaped product entry and owns
//! the daemon/UI modules. This library exposes state layout, a fail-closed
//! daemon socket probe, Box-home daemon ensure/spawn, and a minimal Docker
//! Engine HTTP client for the ExecutionManager adapter. It deliberately does
//! not export the window/UI.

#[cfg(unix)]
pub mod docker_http;
pub mod ensure_daemon;

use std::path::{Path, PathBuf};

#[cfg(unix)]
pub use docker_http::{ContainerPathStat, DockerHttpClient, ExecHijack, HijackOutput};
pub use ensure_daemon::{autostart_enabled, ensure_daemon, resolve_daemon_binary};

/// Default product state directory name under `$HOME` (matches engine flavor).
pub const STATE_DIR_NAME: &str = ".kbox";

/// Durable relative file under a Box execution directory that stores the
/// Docker container id created by the sandbox adapter.
pub const CONTAINER_ID_FILE: &str = "sandbox.container_id";

/// Resolve the sandbox engine state directory for a Box home root.
pub fn state_dir(home: impl AsRef<Path>) -> PathBuf {
    home.as_ref().join(STATE_DIR_NAME)
}

/// Resolve the Docker-compatible daemon socket path under a Box home root.
pub fn docker_socket_path(home: impl AsRef<Path>) -> PathBuf {
    state_dir(home).join("run/docker.sock")
}

/// Path of the persisted container id file for one Box directory.
pub fn container_id_path(box_dir: impl AsRef<Path>) -> PathBuf {
    box_dir.as_ref().join(CONTAINER_ID_FILE)
}

/// Capability snapshot used by the runtime adapter preflight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCapabilities {
    /// Absolute path the adapter expects the daemon to listen on.
    pub socket_path: PathBuf,
    /// Whether the socket path currently exists (not a full HTTP ping).
    pub daemon_reachable: bool,
}

/// Discover whether a sandbox daemon socket appears present for `home`.
///
/// This never starts a daemon and never falls back to MicroVM/OCI.
pub fn discover_capabilities(home: impl AsRef<Path>) -> EngineCapabilities {
    let socket_path = docker_socket_path(home);
    let daemon_reachable = socket_path.exists();
    EngineCapabilities {
        socket_path,
        daemon_reachable,
    }
}

/// Probe an existing daemon socket without launching anything.
#[cfg(unix)]
pub fn probe_daemon_socket(socket: &Path) -> Result<(), String> {
    if !socket.exists() {
        return Err(format!(
            "sandbox engine socket is not present at {}",
            socket.display()
        ));
    }
    Ok(())
}

/// Non-Unix hosts have no fused sandbox daemon path yet.
#[cfg(not(unix))]
pub fn probe_daemon_socket(socket: &Path) -> Result<(), String> {
    Err(format!(
        "sandbox engine is unavailable on this host (socket {})",
        socket.display()
    ))
}

/// Persist a container id under the execution directory.
pub fn write_container_id(box_dir: impl AsRef<Path>, id: &str) -> std::io::Result<PathBuf> {
    let path = container_id_path(box_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, id.as_bytes())?;
    Ok(path)
}

/// Read a previously persisted container id, if present and non-empty.
pub fn read_container_id(box_dir: impl AsRef<Path>) -> std::io::Result<Option<String>> {
    let path = container_id_path(box_dir);
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let trimmed = contents.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CONTAINER_ID_FILE, STATE_DIR_NAME, container_id_path, docker_socket_path,
        read_container_id, state_dir, write_container_id,
    };

    #[test]
    fn layout_matches_product_state_convention() {
        let home = std::path::Path::new("/tmp/kbox-home");
        assert_eq!(state_dir(home), home.join(STATE_DIR_NAME));
        assert_eq!(
            docker_socket_path(home),
            home.join(STATE_DIR_NAME).join("run/docker.sock")
        );
        assert_eq!(
            container_id_path(home.join("boxes/id")),
            home.join("boxes/id").join(CONTAINER_ID_FILE)
        );
    }

    #[test]
    fn container_id_round_trip() {
        let dir = std::env::temp_dir().join(format!("kc{}d", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_container_id(&dir).unwrap().is_none());
        write_container_id(&dir, "abc123").unwrap();
        assert_eq!(read_container_id(&dir).unwrap().as_deref(), Some("abc123"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
