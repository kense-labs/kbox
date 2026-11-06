//! Real CLI against an in-process OCI Distribution registry.

mod support;
use support::registry::TestRegistry;
use support::{create_minimal_oci_tar, CliTest};

#[test]
fn pull_push_login_logout_preserve_oci_identity_and_auth() {
    let cli = CliTest::new();
    let source = cli.home_path().join("registry-source.tar");
    create_minimal_oci_tar(&source, "local/registry-e2e:latest");
    let registry = TestRegistry::start();
    let expected_digest = registry.seed_oci_tar("kbox/e2e", "latest", &source);
    let expected_manifest = registry.manifest("kbox/e2e", "latest").unwrap();
    let pulled = registry.reference("kbox/e2e", "latest");
    let protocol = [("K_REGISTRY_PROTOCOL", "http")];

    cli.ok_with_env(&["pull", &pulled, "--platform", "linux/amd64"], &protocol);
    let inspect: serde_json::Value = serde_json::from_str(&cli.ok(&["image-inspect", &pulled]))
        .expect("pulled image inspect JSON");
    assert_eq!(inspect[0]["Digest"], expected_digest);

    let login = cli.ok_with_stdin(
        &[
            "login",
            registry.address(),
            "--username",
            "e2e-user",
            "--password-stdin",
        ],
        b"e2e-password\n",
    );
    assert!(login.contains("Login Succeeded"));
    assert!(!login.contains("e2e-password"));
    registry.require_basic_auth("e2e-user", "e2e-password");

    let pushed = registry.reference("kbox/e2e", "pushed");
    cli.ok(&["tag", &pulled, &pushed]);
    let output = cli.ok_with_env(&["push", &pushed, "--quiet", "--plain-http"], &protocol);
    assert!(output.contains("kbox/e2e"));
    assert!(!output.contains("e2e-password"));
    assert_eq!(
        registry.manifest("kbox/e2e", "pushed").as_deref(),
        Some(expected_manifest.as_slice())
    );

    let logout = cli.ok(&["logout", registry.address()]);
    assert!(logout.contains("Removing login credentials"));
    let rejected = registry.reference("kbox/e2e", "rejected");
    cli.ok(&["tag", &pulled, &rejected]);
    cli.fails_with_env(
        &["push", &rejected, "--quiet", "--plain-http"],
        &protocol,
        "Failed to push image",
    );
    assert!(registry.manifest("kbox/e2e", "rejected").is_none());

    cli.ok(&["rmi", "--force", &pulled, &pushed, &rejected]);
}

#[test]
fn compose_pull_uses_the_local_registry_and_selected_platform() {
    let cli = CliTest::new();
    let source = cli.home_path().join("compose-registry-source.tar");
    create_minimal_oci_tar(&source, "local/compose-registry-e2e:latest");
    let registry = TestRegistry::start();
    let expected_digest = registry.seed_oci_tar("kbox/compose", "latest", &source);
    let image = registry.reference("kbox/compose", "latest");
    let project = cli.home_path().join("compose-project");
    std::fs::create_dir_all(&project).unwrap();
    let compose = project.join("compose.yaml");
    std::fs::write(
        &compose,
        format!("services:\n  worker:\n    image: {image}\n"),
    )
    .unwrap();
    let compose_arg = compose.to_string_lossy().to_string();

    cli.ok_with_env(
        &[
            "compose",
            "--file",
            &compose_arg,
            "pull",
            "--quiet",
            "--platform",
            "linux/amd64",
            "worker",
        ],
        &[("K_REGISTRY_PROTOCOL", "http")],
    );
    let inspect: serde_json::Value = serde_json::from_str(&cli.ok(&["image-inspect", &image]))
        .expect("Compose pulled image inspect JSON");
    assert_eq!(inspect[0]["Digest"], expected_digest);
    assert!(cli
        .ok(&["compose", "--file", &compose_arg, "images"])
        .contains(&image));
}
