//! The `kbox` command.

use std::process::ExitCode;

#[cfg(target_os = "macos")]
mod bench;
#[cfg(target_os = "macos")]
mod boot;
// builds, cli_pins, scan and tools on Linux too: the daemon reads an
// image's layers and scans it for kbox-win, whose window can't reach the
// store inside WSL (daemon/api.rs).
#[cfg(unix)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod builds;
// The engine stops when its app is deleted under it (kbox-win includes it
// for Windows).
#[cfg(unix)]
mod appwatch;
// Which builder plain `docker build` uses: Docker's or KBox's.
#[cfg(unix)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod buildwith;
// Linux only, but its tests run anywhere with unix sockets.
#[cfg(any(target_os = "linux", all(test, unix)))]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod bridge;
// Linux only (WSL's end of kbox-win's proxy relay); its tests run anywhere.
#[cfg(unix)]
mod cli_pins;
#[cfg(unix)]
mod client;
#[cfg(unix)]
mod daemon;
#[cfg(unix)]
mod diag;
#[cfg(any(unix, windows))]
mod docker;
mod flavor;
#[cfg(unix)]
mod guest;
#[cfg(target_os = "linux")]
mod hostaccess;
#[cfg(unix)]
mod hostrelay;
mod images;
#[cfg(target_os = "macos")]
mod import;
#[cfg(unix)]
mod k8s;
// kubectl's own file, which `kbox k8s` puts its context in.
#[cfg(unix)]
mod kubeconfig;
#[cfg(unix)]
mod machine;
#[cfg(unix)]
mod run;
#[cfg(unix)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod scan;
#[cfg(target_os = "macos")]
mod setup;
#[cfg(any(unix, windows))]
mod share;
#[cfg(unix)]
mod stale;
#[cfg(unix)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod tools;
#[cfg(any(target_os = "macos", all(target_os = "linux", feature = "window")))]
mod ui;
#[cfg(target_os = "macos")]
mod update;
#[cfg(unix)]
mod usb;
#[cfg(windows)]
mod wsl;

const USAGE: &str = "\
usage: kbox COMMAND          docker's commands, on KBox: kbox run, ps, exec,
                             logs, stop, rm, images, pull, build, compose…
       kbox daemon
       kbox bridge [--socket PATH]   (WSL2: the end of kbox-win.exe)
       kbox ui [--start] [--view containers|images|volumes|networks]
       kbox setup [--use] [--no-agent] | kbox setup --uninstall
       kbox build-with [docker|kbox]   which builder docker build uses
       kbox update [--check]
       kbox diag logs [-f] [--ui|--engine|--cli] [--errors] [-n N]
       kbox diag report [--out PATH] [--send]   (a file to send us, tokens taken out)
       kbox PORT [NAME]           share a local port on the internet (same as kbox share PORT)
       kbox share PORT|CONTAINER [--name N] | share login | logout | status
       kbox k8s start [--use] | stop | status | delete | set [--lan on|off] [--tls-san NAME,…] [--port N]
       kbox usb [list] | attach DEVICE | detach DEVICE   USB devices for containers, beta (Mac: pass through to the VM)
       kbox machine create NAME [--distro D] | list | shell NAME | stop | delete NAME
       kbox import [orbstack | desktop-linux] [--list]
       kbox bench [--runs N] [--image IMAGE] [--network none|bridge]
       kbox boot [--console] [--pings N]
       kbox raw run [--rm] [-e KEY=VALUE]... IMAGE [COMMAND...]   (its own guest, no daemon)
       kbox raw pull <image> | raw images | raw rmi <image>...";

fn main() -> ExitCode {
    flavor::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Which process this is, for its log and crash reports.
    #[cfg(unix)]
    diag::init(match args.first().map(String::as_str) {
        Some("daemon") => diag::Part::Daemon,
        // Only a build with the window: elsewhere `kbox ui` is a plain
        // command whose message must reach the terminal, not the log's pipe.
        #[cfg(any(target_os = "macos", all(target_os = "linux", feature = "window")))]
        Some("ui") => diag::Part::Ui,
        #[cfg(target_os = "macos")]
        None if setup::launched_from_app() => diag::Part::Ui,
        _ => diag::Part::Cli,
    });
    let result: std::io::Result<()> = match args.first().map(String::as_str) {
        #[cfg(unix)]
        Some("daemon") => daemon::run(&args[1..]),
        #[cfg(target_os = "linux")]
        Some("bridge") => bridge::run(&args[1..]),
        #[cfg(target_os = "linux")]
        Some("hostrelay") => hostrelay::run(),
        // Linux hosts: what the window's onboarding needs root for (through pkexec).
        #[cfg(target_os = "linux")]
        Some("host-access") => match hostaccess::run(&args[1..]) {
            Ok(code) => return ExitCode::from(code),
            Err(e) => Err(e),
        },
        #[cfg(unix)]
        Some("raw") => match args.get(1).map(String::as_str) {
            Some("run") => run::run(&args[2..]),
            Some("pull") => images::pull(&args[2..]),
            Some("images") => images::list(&args[2..]),
            Some("rmi") => images::remove(&args[2..]),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        },
        #[cfg(target_os = "macos")]
        Some("bench") => bench::run(&args[1..]),
        #[cfg(target_os = "macos")]
        Some("boot") => boot::run(&args[1..]),
        #[cfg(target_os = "macos")]
        Some("setup") => setup::run(&args[1..]),
        #[cfg(any(unix, windows))]
        Some("share") => share::run(&args[1..]),
        // `kbox 3000`: share a port straight away.
        #[cfg(any(unix, windows))]
        Some(a) if share::is_shorthand(a) => share::run(&args),
        #[cfg(unix)]
        Some("build-with") => buildwith::cli(&args[1..]),
        #[cfg(unix)]
        Some("k8s") => k8s::run(&args[1..]),
        #[cfg(unix)]
        Some("machine") => machine::run(&args[1..]),
        #[cfg(unix)]
        Some("usb") => usb::run(&args[1..]),
        #[cfg(target_os = "macos")]
        Some("import") => import::run(&args[1..]),
        #[cfg(target_os = "macos")]
        Some("update") => update::run(&args[1..]),
        #[cfg(unix)]
        Some("diag") => diag::run(&args[1..]),
        // Opening KBox.app: set up on first launch, then the window.
        #[cfg(target_os = "macos")]
        None if setup::launched_from_app() => {
            setup::first_launch().and_then(|()| ui::run(true, None))
        }
        #[cfg(any(target_os = "macos", all(target_os = "linux", feature = "window")))]
        Some("ui") => ui::run(
            args.iter().any(|a| a == "--start"),
            args.iter()
                .position(|a| a == "--view")
                .and_then(|i| args.get(i + 1))
                .cloned(),
        ),
        // The static engine has no window: the package's own build of it.
        #[cfg(all(target_os = "linux", not(feature = "window")))]
        Some("ui") => window(&args),
        #[cfg(any(unix, windows))]
        Some(c) if docker::handles(c) => {
            engine_version_check();
            docker::run(&args)
        }
        // Windows: the engine's commands, by the `kbox` in WSL.
        #[cfg(windows)]
        Some(c) if wsl::COMMANDS.contains(&c) => wsl::run(&args),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // The subcommand only: arguments can carry values.
            #[cfg(unix)]
            if !e.to_string().starts_with("usage:") {
                // Opened from Finder or the Dock there is none: "open" (#250).
                diag::warn!(
                    "command failed",
                    command = args
                        .first()
                        .map_or(Some("open"), |a| (a.len() <= 20
                            && a.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
                        .then_some(a.as_str()))
                        .unwrap_or(""),
                    error = e
                );
            }
            eprintln!("kbox: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Where the `kbox` package puts the window: the same command built with
/// GTK and WebKitGTK (packaging/linux/build.sh).
#[cfg(all(target_os = "linux", not(feature = "window")))]
const WINDOW: &str = "/usr/lib/kbox/kbox-window";

/// `kbox ui` on Linux: runs the package's window, or says why there is none.
#[cfg(all(target_os = "linux", not(feature = "window")))]
fn window(args: &[String]) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    if !std::path::Path::new(WINDOW).exists() {
        return Err(std::io::Error::other(
            "this KBox has no window. It comes with the kbox package for x86-64 (Debian, Ubuntu, Fedora); \
             kbox-lite and the ARM packages are the engine and CLI only",
        ));
    }
    // exec returns only when it failed.
    Err(std::process::Command::new(WINDOW).args(args).exec())
}

#[cfg(not(unix))]
fn engine_version_check() {}

/// Before a docker command: when the engine runs another version than this
/// `kbox`, say so (on stderr, so output stays clean). Either an update whose
/// engine restart was put off, or another `kbox` earlier on PATH.
#[cfg(unix)]
fn engine_version_check() {
    let me = env!("CARGO_PKG_VERSION");
    if me == "0.0.0" || !client::alive() {
        return;
    }
    let Some(engine) = client::request("GET", "/version", None)
        .ok()
        .filter(|r| r.ok())
        .and_then(|r| client::kbox_version(&r.json()).map(str::to_owned))
    else {
        return;
    };
    if engine != me && engine != "0.0.0" {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "kbox".into());
        eprintln!(
            "kbox: this kbox ({exe}) is {me}, but the engine runs {engine}. Restart the engine to use one version (KBox → Restart engine), or remove the older kbox."
        );
    }
}
