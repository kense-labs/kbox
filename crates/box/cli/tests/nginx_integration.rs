//! Integration test: Run containers in kbox MicroVM.
//!
//! This test demonstrates the full lifecycle of running containers
//! inside kbox MicroVMs:
//!
//! 1. Pull an OCI image from Docker Hub
//! 2. Run a container in detached mode
//! 3. Verify the box is running via `ps`
//! 4. Execute commands inside the running box
//! 5. Stop and remove the box
//!
//! ## Prerequisites
//!
//! - `kbox` binary built (`cargo build -p kbox-cli`)
//! - macOS with Apple HVF or Linux with KVM
//! - Internet access (to pull images on first run)
//! - `DYLD_LIBRARY_PATH` set to include libkrun/libkrunfw build dirs
//!
//! ## Running
//!
//! ```bash
//! cd crates/box/src
//!
//! # Set library paths (macOS)
//! export DYLD_LIBRARY_PATH="$(ls -td target/debug/build/libksys-*/out/libkrun/lib | head -1):$(ls -td target/debug/build/libksys-*/out/libkrunfw/lib | head -1)"
//!
//! # Run all integration tests
//! cargo test -p kbox-cli --test nginx_integration -- --ignored --nocapture
//!
//! # Run a single test
//! cargo test -p kbox-cli --test nginx_integration -- --ignored --nocapture test_alpine_full_lifecycle
//! ```
//!
//! Tests are `#[ignore]` by default because they require a built binary,
//! network access, and virtualization support (HVF/KVM).

mod support;

use std::time::Duration;

use support::{cleanup, wait_for_running, CliTest};

fn wait_for<F: Fn() -> bool>(condition: F, timeout: Duration, msg: &str) {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Timeout waiting for: {msg}");
}

// ============================================================================
// Test: Full alpine lifecycle (pull → run → ps → exec → stop → rm)
// ============================================================================

/// Demonstrates the complete kbox VM lifecycle using Alpine Linux.
///
/// This is the primary integration test that verifies:
/// - Image pulling from Docker Hub
/// - VM creation and boot via libkrun
/// - Box status tracking
/// - Command execution inside the VM
/// - Graceful shutdown and cleanup
#[test]
#[ignore] // Requires built binary, network, and virtualization support
fn test_alpine_full_lifecycle() {
    let cli = CliTest::new();
    let box_name = "integ-alpine-lifecycle";
    cleanup(&cli, box_name);

    cli.ok(&["pull", "docker.io/library/alpine:latest"]);
    assert!(cli.ok(&["images"]).contains("alpine"));

    cli.ok(&[
        "run",
        "-d",
        "--name",
        box_name,
        "docker.io/library/alpine:latest",
        "--",
        "sleep",
        "3600",
    ]);
    wait_for_running(&cli, box_name, Duration::from_secs(30));

    let inspect = cli.ok(&["inspect", box_name]);
    assert!(inspect.contains(box_name));
    assert!(inspect.contains("alpine"));

    assert!(cli
        .ok(&["exec", box_name, "--", "uname", "-a"])
        .contains("Linux"));
    assert!(cli
        .ok(&["exec", box_name, "--", "cat", "/etc/os-release"])
        .contains("Alpine"));
    let root = cli.ok(&["exec", box_name, "--", "ls", "/"]);
    assert!(root.contains("bin"));
    assert!(root.contains("etc"));
    cli.ok(&["logs", box_name]);

    cli.ok(&["stop", box_name]);
    wait_for(
        || {
            let stdout = cli.ok(&["ps", "-a"]);
            stdout.contains(box_name) && (stdout.contains("stopped") || stdout.contains("exited"))
        },
        Duration::from_secs(15),
        "box to appear as stopped",
    );

    cli.ok(&["rm", box_name]);
    assert!(!cli.ok(&["ps", "-a"]).contains(box_name));
}

// ============================================================================
// Test: Execute multiple commands inside a running box
// ============================================================================

/// Demonstrates executing various commands inside a running kbox VM.
#[test]
#[ignore]
fn test_exec_commands() {
    let cli = CliTest::new();
    let box_name = "integ-exec-cmds";
    cleanup(&cli, box_name);

    cli.ok(&[
        "run",
        "-d",
        "--name",
        box_name,
        "docker.io/library/alpine:latest",
        "--",
        "sleep",
        "3600",
    ]);
    wait_for_running(&cli, box_name, Duration::from_secs(30));

    assert!(cli
        .ok(&["exec", box_name, "--", "cat", "/etc/os-release"])
        .contains("Alpine"));
    assert!(!cli
        .ok(&["exec", box_name, "--", "ls", "/usr/bin/"])
        .is_empty());
    assert!(cli.ok(&["exec", box_name, "--", "env"]).contains("PATH="));

    cli.ok(&[
        "exec",
        box_name,
        "--",
        "sh",
        "-c",
        "echo hello-kbox > /tmp/test.txt",
    ]);
    assert_eq!(
        cli.ok(&["exec", box_name, "--", "cat", "/tmp/test.txt"])
            .trim(),
        "hello-kbox"
    );

    cleanup(&cli, box_name);
}

// ============================================================================
// Test: Run with environment variables and labels
// ============================================================================

/// Demonstrates passing environment variables and labels to a box.
#[test]
#[ignore]
fn test_env_and_labels() {
    let cli = CliTest::new();
    let box_name = "integ-env-labels";
    cleanup(&cli, box_name);

    cli.ok(&[
        "run",
        "-d",
        "--name",
        box_name,
        "-e",
        "MY_APP=ktest",
        "-e",
        "MY_VERSION=1.0",
        "-l",
        "app=test",
        "-l",
        "env=integration",
        "docker.io/library/alpine:latest",
        "--",
        "sleep",
        "3600",
    ]);
    wait_for_running(&cli, box_name, Duration::from_secs(30));

    let inspect = cli.ok(&["inspect", box_name]);
    let parsed: serde_json::Value = serde_json::from_str(&inspect).expect("valid inspect JSON");
    assert_eq!(parsed[0]["name"], box_name);
    assert_eq!(parsed[0]["labels"]["app"], "test");
    assert_eq!(parsed[0]["labels"]["env"], "integration");
    assert_eq!(
        cli.ok(&["exec", box_name, "--", "sh", "-c", "echo $MY_APP"])
            .trim(),
        "ktest"
    );
    assert_eq!(
        cli.ok(&["exec", box_name, "--", "sh", "-c", "echo $MY_VERSION"])
            .trim(),
        "1.0"
    );

    cleanup(&cli, box_name);
}

// ============================================================================
// Test: nginx with known limitation
// ============================================================================

/// Pulls nginx, boots it, and verifies the actual guest HTTP response.
#[test]
#[ignore]
fn test_nginx_image_pull_and_run() {
    let cli = CliTest::new();
    let box_name = "integ-nginx";
    cleanup(&cli, box_name);

    cli.ok(&["pull", "docker.io/library/nginx:alpine"]);
    assert!(cli.ok(&["images"]).contains("nginx"));
    cli.ok(&[
        "run",
        "-d",
        "--name",
        box_name,
        "-p",
        "8088:80",
        "docker.io/library/nginx:alpine",
    ]);
    wait_for_running(&cli, box_name, Duration::from_secs(30));

    assert!(cli.ok(&["port", box_name]).contains("8088"));
    let response = cli.ok(&[
        "exec",
        box_name,
        "--",
        "wget",
        "-q",
        "-O",
        "-",
        "http://127.0.0.1:80",
    ]);
    assert!(response.contains("Welcome to nginx"), "{response}");

    cleanup(&cli, box_name);
}
