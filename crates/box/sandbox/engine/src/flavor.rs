//! KBox Sandbox Dev: a second KBox beside the installed one, for trying changes
//! without touching it (`cargo xtask app-dev` builds `KBox Sandbox Dev.app`).
//!
//! It is the same binary; running from a bundle whose identifier is
//! `dev.kbox.sandbox.app.dev` makes it Dev. Everything two KBoxs would share gets
//! its own name: `~/.kbox-dev` (socket, settings, logs),
//! `~/Library/Application Support/KBox Sandbox Dev` (images, volumes, the VM's
//! disk), the `dev.kbox.sandbox.daemon.dev` LaunchAgent, the `kbox-dev` docker and
//! kubectl contexts and `~/KBox Sandbox Dev` in Finder. It leaves to the installed
//! KBox what there is one of on a Mac: `*.kbox.local`, the container-name
//! routers on ports 80 and 443, docker's default context, the shell PATH
//! and updates.
//!
//! A feature app (`cargo xtask app-dev builds` builds `KBox Sandbox Builds.app`,
//! identifier `dev.kbox.sandbox.app.builds`) is a KBox Sandbox Dev of its own, for
//! trying one unreleased feature: every Dev name with `dev` swapped for the
//! feature's (`~/.kbox-builds`, the `kbox-builds` context, and so on).

// Some names serve only the Mac (the app, its LaunchAgent).
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::fmt;
use std::sync::OnceLock;

/// KBox Sandbox Dev.app's bundle identifier.
pub const DEV_APP_ID: &str = "dev.kbox.sandbox.app.dev";

/// Whether this binary is KBox Sandbox Dev (or a feature app): it runs from a
/// bundle (directly, or through `~/.kbox-dev/bin/kbox`, a link into it)
/// whose identifier is `DEV_APP_ID`, or `dev.kbox.sandbox.app.<feature>`.
pub fn dev() -> bool {
    flavor().is_some()
}

/// `Some("")` for KBox Sandbox Dev, `Some("builds")` for KBox Sandbox Builds, `None` for KBox.
fn flavor() -> Option<&'static str> {
    static FLAVOR: OnceLock<Option<String>> = OnceLock::new();
    FLAVOR
        .get_or_init(|| {
            let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
            let plist = std::fs::read_to_string(exe.parent()?.parent()?.join("Info.plist")).ok()?;
            let id = plist
                .split("<key>CFBundleIdentifier</key>")
                .nth(1)?
                .split("<string>")
                .nth(1)?
                .split("</string>")
                .next()?
                .trim();
            if id == DEV_APP_ID {
                return Some(String::new());
            }
            let f = id.strip_prefix("dev.kbox.sandbox.app.")?;
            (!f.is_empty()
                && f.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
            .then(|| f.to_owned())
        })
        .as_deref()
}

/// A Dev name made a feature's: `KBox Sandbox Dev` → `KBox Sandbox Builds`, `kbox-dev` →
/// `kbox-builds`, `dev.kbox.sandbox.daemon.dev` → `dev.kbox.sandbox.daemon.builds`.
fn featured(dev_name: &str, feature: &str) -> String {
    let title = feature[..1].to_ascii_uppercase() + &feature[1..];
    if let Some(head) = dev_name.strip_suffix(".dev") {
        return format!("{head}.{feature}");
    }
    dev_name
        .replacen(" Dev", &format!(" {title}"), 1)
        .replacen("-dev", &format!("-{feature}"), 1)
}

/// A name with a Dev counterpart. Displays as the one this binary uses.
pub struct Name(pub &'static str, pub &'static str);

impl Name {
    pub fn get(&self) -> &'static str {
        match flavor() {
            None => self.0,
            Some("") => self.1,
            Some(f) => {
                static NAMES: std::sync::Mutex<Vec<(&str, &str)>> =
                    std::sync::Mutex::new(Vec::new());
                let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
                if let Some((_, n)) = names.iter().find(|(d, _)| *d == self.1) {
                    return n;
                }
                let n: &'static str = Box::leak(featured(self.1, f).into_boxed_str());
                names.push((self.1, n));
                n
            }
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.get())
    }
}

/// The app's name, as the window and Finder show it.
pub const APP: Name = Name("KBox", "KBox Sandbox Dev");
/// Under HOME: the daemon's socket and state, the window's settings.
pub const STATE: Name = Name(".kbox", ".kbox-dev");
/// The docker context, and the kubectl one.
pub const CONTEXT: Name = Name("kbox", "kbox-dev");
pub const AGENT: Name = Name("dev.kbox.sandbox.daemon", "dev.kbox.sandbox.daemon.dev");
pub const APP_ID: Name = Name("dev.kbox.sandbox.app", DEV_APP_ID);

/// KBox Sandbox Dev keeps its images, volumes and VM disk apart: `KBOX_SANDBOX_DATA_DIR`,
/// which `Store::default_root` reads, set before any thread starts.
pub fn init() {
    if dev() && std::env::var_os("KBOX_SANDBOX_DATA_DIR").is_none() {
        if let Some(home) = std::env::var_os("HOME") {
            // SAFETY: called first thing in main, before any other thread.
            unsafe {
                std::env::set_var(
                    "KBOX_SANDBOX_DATA_DIR",
                    std::path::Path::new(&home)
                        .join("Library/Application Support")
                        .join(APP.get()),
                )
            };
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_test_binary_is_not_dev() {
        assert!(!super::dev());
        assert_eq!(
            (super::STATE.get(), super::CONTEXT.to_string()),
            (".kbox", "kbox".to_owned())
        );
    }

    #[test]
    fn a_feature_renames_every_dev_name() {
        use super::featured;
        assert_eq!(
            featured("KBox Sandbox Dev", "builds"),
            "KBox Sandbox Builds"
        );
        assert_eq!(featured(".kbox-dev", "builds"), ".kbox-builds");
        assert_eq!(featured("kbox-dev", "builds"), "kbox-builds");
        assert_eq!(featured("kbox-dev-build", "builds"), "kbox-builds-build");
        assert_eq!(
            featured("dev.kbox.sandbox.daemon.dev", "builds"),
            "dev.kbox.sandbox.daemon.builds"
        );
        assert_eq!(
            featured("dev.kbox.sandbox.app.dev", "builds"),
            "dev.kbox.sandbox.app.builds"
        );
    }
}
