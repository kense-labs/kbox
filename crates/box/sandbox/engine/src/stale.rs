//! An older `kbox` that terminals run instead of this one: how an update
//! or reinstall seems not to have happened. Mac and Linux.
use std::path::PathBuf;
use std::process::Command;

/// The first `kbox` on the user's PATH, when it isn't this KBox's: an older
/// copy (a manual build in ~/.local/bin, Homebrew, a file in /usr/local/bin)
/// that runs instead of this one in every terminal. Reported, never deleted.
/// The PATH is the login shell's: a window opened from Finder or the app
/// menu has only the system's, without ~/.local/bin and the like.
pub fn stale_kbox() -> Option<PathBuf> {
    let ours = std::fs::canonicalize(our_kbox()?).ok()?;
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| s.starts_with('/'))
        .unwrap_or_else(|| "/bin/sh".into());
    let out = Command::new(&shell)
        .args(["-ilc", "command -v kbox"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let found = String::from_utf8_lossy(&out.stdout)
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with('/'))?
        .to_owned();
    // The snap's own command: /snap/bin/kbox is a link to snapd's launcher,
    // which runs this snap's usr/bin/kbox.
    if found == "/snap/bin/kbox" && ours.starts_with("/snap/kbox/") {
        return None;
    }
    match std::fs::canonicalize(&found) {
        Ok(real) if real == ours => None,
        _ => Some(PathBuf::from(found)),
    }
}

/// This KBox's `kbox`: the running binary on the Mac (the window is the same
/// binary), the package's /usr/bin/kbox beside Linux's separate window.
fn our_kbox() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if exe.file_name().is_some_and(|n| n == "kbox") {
        return Some(exe);
    }
    let snap = std::env::var_os("SNAP").map(|s| PathBuf::from(s).join("usr/bin/kbox"));
    [snap, Some(PathBuf::from("/usr/bin/kbox"))]
        .into_iter()
        .flatten()
        .find(|p| p.is_file())
}
