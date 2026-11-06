use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

mod support;
use support::{seed_runnable_alpine_image, CliTest};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct ReplicaCleanup<'a>(&'a CliTest);

impl Drop for ReplicaCleanup<'_> {
    fn drop(&mut self) {
        for id in replica_ids(self.0) {
            let _ = self.0.output(&["rm", "--force", &id]);
        }
    }
}

fn unused_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}

fn spawn_server(address: SocketAddr, state: &Path, home: &Path, services: Option<&Path>) -> Server {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kbox"));
    command
        .args([
            "scale-api",
            "--address",
            &address.to_string(),
            "--state",
            state.to_str().unwrap(),
            "--max-instances",
            "10",
        ])
        .env("K_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(services) = services {
        command.arg("--services").arg(services);
    } else {
        command.arg("--desired-state-only");
    }
    Server(command.spawn().unwrap())
}

fn wait_ready(address: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(address).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("scale-api did not become ready at {address}");
}

fn request_json() -> &'static str {
    r#"{"schema_version":1,"operation_id":"lost-response-v1","service":"api","expected_revision":"0","direction":"Up","current_replicas":0,"desired_replicas":2,"reason":"process recovery test"}"#
}

fn send_post_body(address: SocketAddr, body: &str) -> TcpStream {
    let request = format!(
        "POST /v1/scale/api HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream
}

fn send_post(address: SocketAddr) -> TcpStream {
    send_post_body(address, request_json())
}

fn post_json(address: SocketAddr, body: &str) -> String {
    let mut stream = send_post_body(address, body);
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn post(address: SocketAddr) -> String {
    post_json(address, request_json())
}

fn operation(id: &str, expected_revision: &str, current: u32, desired: u32) -> String {
    serde_json::json!({
        "schema_version": 1,
        "operation_id": id,
        "service": "api",
        "expected_revision": expected_revision,
        "direction": if desired > current { "Up" } else { "Down" },
        "current_replicas": current,
        "desired_replicas": desired,
        "reason": "native process end-to-end test"
    })
    .to_string()
}

fn replica_ids(cli: &CliTest) -> Vec<String> {
    cli.ok(&[
        "ps",
        "--all",
        "--quiet",
        "--filter",
        "label=com.kbox.scale.managed=true",
    ])
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(str::to_string)
    .collect()
}

fn wait_revision(state: &Path, revision: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(contents) = std::fs::read_to_string(state) {
            let journal: serde_json::Value = serde_json::from_str(&contents).unwrap();
            if journal["revisions"]["api"]
                .as_u64()
                .map(|value| value.to_string())
                == Some(revision.to_string())
            {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("revision {revision} was not durably recorded");
}

#[test]
fn lost_response_is_exactly_replayable_after_process_restart() {
    let cli = CliTest::new();
    let state = cli.home_path().join("scale-authority.json");
    let address = unused_address();
    let mut first = spawn_server(address, &state, cli.home_path(), None);
    wait_ready(address);

    let abandoned_response = send_post(address);
    wait_revision(&state, "1");
    drop(abandoned_response);
    first.0.kill().unwrap();
    first.0.wait().unwrap();

    let restarted = spawn_server(address, &state, cli.home_path(), None);
    wait_ready(address);
    let replay = post(address);
    assert!(replay.starts_with("HTTP/1.1 200 OK"), "{replay}");
    assert!(replay.contains(r#""actual_replicas":2"#), "{replay}");
    assert!(replay.contains(r#""revision":"1""#), "{replay}");
    drop(restarted);
}

#[test]
#[ignore = "requires native HVF/KVM and a runnable OCI image"]
fn reconciles_real_replicas_and_rejects_stale_revisions() {
    let cli = CliTest::new();
    let image = "docker.io/library/alpine:latest";
    seed_runnable_alpine_image(&cli, image);
    let services = cli.home_path().join("services.acl");
    std::fs::write(
        &services,
        format!(
            r#"service "api" {{
  image = "{image}"
  command = ["sleep", "3600"]
}}
"#
        ),
    )
    .unwrap();
    let state = cli.home_path().join("scale-authority.json");
    let address = unused_address();
    let cleanup = ReplicaCleanup(&cli);
    let server = spawn_server(address, &state, cli.home_path(), Some(&services));
    wait_ready(address);

    let up = post_json(address, &operation("native-up", "0", 0, 2));
    let two_replicas = replica_ids(&cli);
    let down_one = post_json(address, &operation("native-down-one", "1", 2, 1));
    let one_replica = replica_ids(&cli);
    let stale = post_json(address, &operation("native-stale", "1", 1, 0));
    let after_stale = replica_ids(&cli);
    let down = post_json(address, &operation("native-down", "2", 1, 0));
    let zero_replicas = replica_ids(&cli);
    drop(server);

    assert!(up.starts_with("HTTP/1.1 200 OK"), "{up}");
    assert!(up.contains(r#""actual_replicas":2"#), "{up}");
    assert!(up.contains(r#""revision":"1""#), "{up}");
    assert_eq!(two_replicas.len(), 2, "{two_replicas:?}");
    assert!(down_one.starts_with("HTTP/1.1 200 OK"), "{down_one}");
    assert!(down_one.contains(r#""actual_replicas":1"#), "{down_one}");
    assert_eq!(one_replica.len(), 1, "{one_replica:?}");
    assert!(stale.starts_with("HTTP/1.1 409 Conflict"), "{stale}");
    assert!(stale.contains("stale_revision"), "{stale}");
    assert_eq!(after_stale, one_replica);
    assert!(down.starts_with("HTTP/1.1 200 OK"), "{down}");
    assert!(down.contains(r#""actual_replicas":0"#), "{down}");
    assert!(zero_replicas.is_empty(), "{zero_replicas:?}");
    drop(cleanup);
}
