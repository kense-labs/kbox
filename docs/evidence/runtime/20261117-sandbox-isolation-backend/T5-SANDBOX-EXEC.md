# T5 — Sandbox `kbox exec` (Docker exec API)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Managed Sandbox boxes previously timed out on MicroVM guest exec
(`Exec response timed out after 15s`). Spec050 path now uses fused daemon
HTTP exec (create → start → inspect ExitCode) via
`DockerHttpClient::container_exec` and
`SandboxLocalExecutionBackend::execute`. Session dispatch uses
`uses_backend_session` so Sandbox never falls through to the guest exec
socket.

## Hermetic

```bash
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-exec-ut-XXXXXX)
cargo test --locked -p kbox-sandbox-engine docker_http
cargo test --locked -p kbox-runtime --lib local_execution::sandbox_backend
```

Observed: docker_http 6 passed (includes `fake_unix_http_container_exec`);
sandbox_backend 7 passed (includes `execute_against_fake_unix_http_daemon`
and fail-closed without daemon).

## Native smoke

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-exec-XXXXXX)
cargo build --locked -p kbox-cli
./target/debug/kbox run -d --name sex alpine:3.20 -- sleep 3600
./target/debug/kbox exec sex -- echo SANDBOX_EXEC_OK
./target/debug/kbox rm -f sex
```

Observed:

```text
SANDBOX_EXEC_OK
exec_exit=0
```

K_HOME for this run: `/tmp/kbox-sandbox-exec-ozuNZB`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Streaming / PTY / stdin-piped sandbox exec
- Guest rootfs chroot on sandbox exec
- Product Pass / notarized daily install
- Multi-host matrix
