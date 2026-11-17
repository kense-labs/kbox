//! Spawn-or-attach the fused sandbox engine daemon for a Box home.
//!
//! Fail closed: never falls back to MicroVM/OCI. Hermetic tests that already
//! bind a fake socket short-circuit on socket presence without spawning.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::docker_socket_path;

const BINARY_NAME: &str = "kbox-sandbox-engine";
const DEFAULT_WAIT: Duration = Duration::from_secs(8);
const POLL: Duration = Duration::from_millis(50);

/// Whether spawn-on-demand is allowed (default: yes).
///
/// Set `KBOX_SANDBOX_AUTOSTART=0|false|no` to refuse spawning (fail closed on
/// a missing socket). Used by hermetic fail-closed tests.
pub fn autostart_enabled() -> bool {
    match std::env::var("KBOX_SANDBOX_AUTOSTART") {
        Ok(value) => {
            let trimmed = value.trim();
            !(trimmed.eq_ignore_ascii_case("0")
                || trimmed.eq_ignore_ascii_case("false")
                || trimmed.eq_ignore_ascii_case("no")
                || trimmed.eq_ignore_ascii_case("off"))
        }
        Err(_) => true,
    }
}

/// Resolve the engine binary: env override, `CARGO_BIN_EXE_*` (tests), sibling
/// of current exe, then PATH.
pub fn resolve_daemon_binary() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("KBOX_SANDBOX_ENGINE_BIN") {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Some(path);
        }
    }
    // Integration/unit tests for the engine package itself.
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_kbox-sandbox-engine") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if exe
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == BINARY_NAME || name.starts_with(BINARY_NAME))
            && exe.is_file()
        {
            return Some(exe.clone());
        }
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(BINARY_NAME);
            if sibling.is_file() {
                return Some(sibling);
            }
            // `cargo test` runs from `target/<profile>/deps/`; binary lives one up.
            if let Some(parent) = dir.parent() {
                let candidate = parent.join(BINARY_NAME);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    which_on_path(BINARY_NAME)
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Ensure a daemon socket exists under `home` (spawn if allowed and missing).
///
/// When the socket file is already present, returns Ok without probing HTTP
/// (hermetic single-accept fakes must not be raced by a ping).
#[cfg(unix)]
pub fn ensure_daemon(home: impl AsRef<Path>) -> Result<PathBuf, String> {
    let home = home.as_ref();
    let socket = docker_socket_path(home);
    if socket.exists() {
        return Ok(socket);
    }
    if !autostart_enabled() {
        return Err(format!(
            "sandbox engine daemon unreachable at {} (autostart disabled)",
            socket.display()
        ));
    }
    let binary = resolve_daemon_binary().ok_or_else(|| {
        format!(
            "sandbox engine daemon unreachable at {}: {BINARY_NAME} binary not found",
            socket.display()
        )
    })?;
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "sandbox could not create daemon socket directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let mut command = Command::new(&binary);
    command
        .arg("daemon")
        .env("HOME", home)
        .env("KBOX_SANDBOX_SOCKET", &socket)
        // Keep finder/NFS off in Box-managed homes.
        .env("KBOX_SANDBOX_NO_FINDER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: pre_exec runs in the child after fork; setsid only affects
        // this new process so the daemon is not tied to the parent TTY/session.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    command.spawn().map_err(|error| {
        format!(
            "sandbox could not start engine daemon ({}): {error}",
            binary.display()
        )
    })?;

    let deadline = Instant::now() + DEFAULT_WAIT;
    while Instant::now() < deadline {
        if socket.exists() {
            return Ok(socket);
        }
        std::thread::sleep(POLL);
    }
    Err(format!(
        "sandbox engine daemon did not create socket at {} within {}s",
        socket.display(),
        DEFAULT_WAIT.as_secs()
    ))
}

#[cfg(not(unix))]
pub fn ensure_daemon(home: impl AsRef<Path>) -> Result<PathBuf, String> {
    let socket = docker_socket_path(home);
    Err(format!(
        "sandbox engine is unavailable on this host (socket {})",
        socket.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::{autostart_enabled, ensure_daemon};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn existing_socket_short_circuits_without_spawn() {
        let _guard = env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("ens{}h", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        let socket = crate::docker_socket_path(&home);
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        std::fs::write(&socket, b"").unwrap();
        let got = ensure_daemon(&home).unwrap();
        assert_eq!(got, socket);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn autostart_disabled_fails_closed_without_socket() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os("KBOX_SANDBOX_AUTOSTART");
        // SAFETY: single-threaded under env_lock for this process's tests.
        unsafe { std::env::set_var("KBOX_SANDBOX_AUTOSTART", "0") };
        assert!(!autostart_enabled());
        let home = std::env::temp_dir().join(format!("ens{}d", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let error = ensure_daemon(&home).unwrap_err();
        assert!(
            error.contains("autostart disabled"),
            "unexpected error: {error}"
        );
        match previous {
            Some(value) => unsafe { std::env::set_var("KBOX_SANDBOX_AUTOSTART", value) },
            None => unsafe { std::env::remove_var("KBOX_SANDBOX_AUTOSTART") },
        }
        let _ = std::fs::remove_dir_all(&home);
    }
}
