//! End-to-end coverage for local artifacts and observation commands.

use std::time::Duration;

mod support;
use support::{read_file_from_saved_oci_tar, CliTest};

fn inspect_image(cli: &CliTest, reference: &str) -> serde_json::Value {
    let output = cli.ok(&["image-inspect", reference]);
    let value: serde_json::Value = serde_json::from_str(&output).expect("image inspect JSON");
    match value {
        serde_json::Value::Array(mut records) if !records.is_empty() => records.remove(0),
        other => other,
    }
}

#[test]
fn import_creates_a_runnable_oci_artifact_with_changes() {
    let cli = CliTest::new();
    let rootfs = cli.home_path().join("rootfs.tar");
    let file = std::fs::File::create(&rootfs).expect("create rootfs archive");
    let mut archive = tar::Builder::new(file);
    let content = b"import-e2e-ok\n";
    let mut header = tar::Header::new_gnu();
    header.set_path("message.txt").unwrap();
    header.set_size(content.len() as u64);
    header.set_mode(0o640);
    header.set_cksum();
    archive
        .append(&header, &content[..])
        .expect("append rootfs file");
    archive.finish().expect("finish rootfs archive");

    let reference = "local/import-e2e:latest";
    let rootfs_arg = rootfs.to_string_lossy().to_string();
    let output = cli.ok(&[
        "import",
        &rootfs_arg,
        reference,
        "--change",
        "CMD [\"cat\",\"/message.txt\"]",
        "--change",
        "ENV IMPORT_E2E=ok",
        "--message",
        "local import e2e",
    ]);
    assert!(output.contains("sha256:"));

    let inspect = inspect_image(&cli, reference);
    assert_eq!(inspect["Reference"], reference);
    assert_eq!(inspect["LayerCount"], 1);
    assert_eq!(
        inspect["Config"]["Cmd"],
        serde_json::json!(["cat", "/message.txt"])
    );
    assert_eq!(inspect["Config"]["Env"]["IMPORT_E2E"], "ok");
    assert!(cli.ok(&["history", reference]).contains("local import e2e"));

    let saved = cli.home_path().join("imported.tar");
    let saved_arg = saved.to_string_lossy().to_string();
    cli.ok(&["save", reference, "--output", &saved_arg]);
    assert_eq!(
        read_file_from_saved_oci_tar(&saved, "/message.txt").as_deref(),
        Some("import-e2e-ok\n")
    );

    cli.ok(&[
        "load",
        "--input",
        &saved_arg,
        "--tag",
        "local/import-reloaded:latest",
    ]);
    assert_eq!(
        inspect_image(&cli, "local/import-reloaded:latest")["Digest"],
        inspect["Digest"]
    );
    cli.ok(&["rmi", "--force", reference, "local/import-reloaded:latest"]);
}

#[test]
fn import_rejects_invalid_change_without_publishing_a_reference() {
    let cli = CliTest::new();
    let rootfs = cli.home_path().join("invalid-rootfs.tar");
    let mut archive = tar::Builder::new(std::fs::File::create(&rootfs).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_path("message.txt").unwrap();
    header.set_size(1);
    header.set_mode(0o644);
    header.set_cksum();
    archive.append(&header, &b"x"[..]).unwrap();
    archive.finish().unwrap();

    let rootfs_arg = rootfs.to_string_lossy().to_string();
    cli.fails(
        &[
            "import",
            &rootfs_arg,
            "local/import-invalid:latest",
            "--change",
            "NOT_AN_INSTRUCTION value",
        ],
        "Unsupported --change instruction",
    );
    assert!(!cli
        .ok(&["images", "--quiet"])
        .contains("local/import-invalid:latest"));
}

#[test]
fn audit_returns_real_create_records_with_filters_and_redaction() {
    let cli = CliTest::new();
    let id = cli
        .ok(&[
            "create",
            "--name",
            "audit-e2e",
            "docker.io/library/alpine:latest",
        ])
        .trim()
        .to_string();

    let output = cli.ok(&[
        "audit",
        "--json",
        "--action",
        "box_create",
        "--box-id",
        &id,
        "--outcome",
        "success",
        "--limit",
        "1",
    ]);
    let event: serde_json::Value = serde_json::from_str(output.trim()).expect("audit JSON line");
    assert_eq!(event["action"], "box_create");
    assert_eq!(event["outcome"], "success");
    assert_eq!(event["box_id"], id);
    assert!(!output.contains("password"));
    cli.fails(&["audit", "--action", "not_an_action"], "Unknown action");
    cli.ok(&["rm", "audit-e2e"]);
}

#[test]
fn events_streams_real_create_transition_as_json() {
    let cli = CliTest::new();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(300));
            cli.ok(&[
                "create",
                "--name",
                "events-e2e",
                "docker.io/library/alpine:latest",
            ]);
        });
        let (stdout, _stderr) = cli.interrupts_after_output_any_status(
            &["events", "--json"],
            "\"action\":\"create\"",
            Duration::from_secs(10),
        );
        let event = stdout
            .lines()
            .find(|line| line.contains("\"action\":\"create\""))
            .expect("create event");
        let event: serde_json::Value = serde_json::from_str(event).expect("event JSON");
        assert_eq!(event["type"], "container");
        assert_eq!(event["actor"]["name"], "events-e2e");
    });
    cli.ok(&["rm", "events-e2e"]);
}

#[test]
fn network_prune_removes_only_unused_networks() {
    let cli = CliTest::new();
    cli.ok(&[
        "network",
        "create",
        "prune-unused",
        "--subnet",
        "10.181.0.0/24",
    ]);
    cli.ok(&[
        "network",
        "create",
        "prune-connected",
        "--subnet",
        "10.182.0.0/24",
    ]);

    let store = kbox_runtime::NetworkStore::new(cli.home_path().join("networks.json"));
    let mut connected = store
        .get("prune-connected")
        .expect("load connected network")
        .expect("connected network exists");
    connected
        .connect("box-e2e", "endpoint-e2e")
        .expect("attach test endpoint");
    store.update(&connected).expect("save connected network");

    let output = cli.ok(&["network", "prune", "--force"]);
    assert!(output.contains("prune-unused"), "{output}");
    assert!(!output.contains("prune-connected\n"), "{output}");
    assert!(store.get("prune-unused").expect("load pruned").is_none());
    assert!(store
        .get("prune-connected")
        .expect("load connected")
        .is_some());
}

#[test]
fn snapshot_prune_enforces_count_and_byte_limits() {
    let cli = CliTest::new();
    cli.fails(&["snapshot", "prune"], "specify --keep");

    let store = kbox_runtime::SnapshotStore::new(&cli.home_path().join("snapshots"))
        .expect("create snapshot store");
    let rootfs = cli.home_path().join("snapshot-rootfs");
    std::fs::create_dir_all(&rootfs).expect("create snapshot rootfs");
    std::fs::write(rootfs.join("payload"), b"snapshot-e2e").expect("write snapshot payload");

    for index in 0..3 {
        let mut metadata = kbox_core::snapshot::SnapshotMetadata::new(
            format!("snap-e2e-{index}"),
            format!("snapshot-{index}"),
            "box-e2e".to_string(),
            "alpine:latest".to_string(),
        );
        metadata.created_at = chrono::DateTime::from_timestamp(1_700_000_000 + index, 0)
            .expect("valid snapshot timestamp");
        store.save(metadata, &rootfs).expect("save snapshot");
    }

    let output = cli.ok(&["snapshot", "prune", "--keep", "2"]);
    assert!(output.contains("snap-e2e-0"), "{output}");
    assert_eq!(
        store
            .list()
            .expect("list after count prune")
            .iter()
            .map(|snapshot| snapshot.id.as_str())
            .collect::<Vec<_>>(),
        ["snap-e2e-2", "snap-e2e-1"]
    );

    let newest_size = store
        .get("snap-e2e-2")
        .expect("load newest snapshot")
        .expect("newest snapshot exists")
        .size_bytes;
    let max_bytes = newest_size.to_string();
    let output = cli.ok(&["snapshot", "prune", "--max-bytes", &max_bytes]);
    assert!(output.contains("snap-e2e-1"), "{output}");
    assert_eq!(
        store
            .list()
            .expect("list after size prune")
            .iter()
            .map(|snapshot| snapshot.id.as_str())
            .collect::<Vec<_>>(),
        ["snap-e2e-2"]
    );
}
