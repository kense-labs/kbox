use super::*;

pub(super) fn exercise(
    root: &Path,
    image: &str,
    oci: &kbox_core::ExecutionId,
    legacy: &kbox_core::ExecutionId,
) -> bool {
    let Some(binary) = std::env::var_os("K_BOX_TEST_HVF_CLI") else {
        return false;
    };
    let run = |policy: &str, args: &[&str]| {
        let output = Command::new(&binary)
            .current_dir(root)
            .env("K_HOME", root.join("home"))
            .env("K_BOX_OCI_HVF_ENDPOINT", root.join("service/runtime.sock"))
            .env("K_BOX_OCI_MIGRATION", policy)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "CLI {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    for (id, route) in [(oci, "oci_sdk"), (legacy, "box_vm")] {
        let inspected: serde_json::Value =
            serde_json::from_slice(&run("off", &["inspect", id.as_str()])).unwrap();
        assert_eq!(inspected[0]["managed_execution"]["runtime_route"], route);
    }
    assert_eq!(
        run(
            "off",
            &["exec", oci.as_str(), "--", "/bin/echo", "cli-retained-oci"]
        ),
        b"cli-retained-oci\n"
    );
    let id = String::from_utf8(run(
        "microvm",
        &[
            "run",
            "-d",
            "--name",
            "cli-hvf",
            "--cpus",
            "1",
            "--memory",
            "512",
            "--network",
            "none",
            image,
            "--",
            "/bin/sleep",
            "2147483647",
        ],
    ))
    .unwrap()
    .trim()
    .to_string();
    let inspected: serde_json::Value =
        serde_json::from_slice(&run("microvm", &["inspect", &id])).unwrap();
    assert_eq!(
        inspected[0]["managed_execution"]["runtime_route"],
        "oci_sdk"
    );
    assert_eq!(
        run(
            "microvm",
            &["exec", &id, "--", "/bin/echo", "cli-created-oci"]
        ),
        b"cli-created-oci\n"
    );
    run("off", &["rm", "-f", &id]);
    fs::write(root.join("cli.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","binary":binary,"created":id,"persistedRoutes":true,"envComposition":true})).unwrap()).unwrap();
    true
}
