# T5 — Sandbox non-TTY streaming (`kbox exec -i`)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Managed Sandbox boxes returned Unavailable for `start_process` (non-TTY
streaming). Spec050 now implements
`SandboxLocalExecutionBackend::start_process` via Docker exec create →
upgrade hijack with multiplexed stdout/stderr frames
(`DockerHttpClient::container_exec_attach` + `ExecHijack::next_output`).
Session dispatch already uses `uses_backend_session`, so Sandbox never falls
through to guest exec sockets.

CLI: managed `kbox exec -i` without `-t` opens a streaming process session
(not captured one-shot). Piped stdin works without allocating a PTY.

Captured `execute` still refuses stdin buffers and points callers at
streaming (`-i`) or PTY (`-t`).

## Hermetic

```bash
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-stream-ut-XXXXXX)
cargo test --locked -p kbox-sandbox-engine docker_http
cargo test --locked -p kbox-runtime --lib local_execution::sandbox_backend
```

Observed: docker_http 11 passed (includes
`fake_unix_http_container_exec_attach_mux`); sandbox_backend 10 passed
(includes `start_process_against_fake_unix_http_daemon` and
`start_process_fail_closes_without_daemon`).

## Native smoke

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-stream-XXXXXX)
cargo build --locked -p kbox-cli -p kbox-sandbox-engine
./target/debug/kbox run -d --name streambox alpine:3.20 -- sleep 3600
printf 'echo STREAM_OK\nexit\n' | ./target/debug/kbox exec -i streambox -- sh
./target/debug/kbox rm -f streambox
```

Observed:

```text
STREAM_OK
it_exit=0
```

K_HOME for this run: `/tmp/kbox-sandbox-stream-UQYOU8`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Captured one-shot `execute` stdin piping (still Unavailable; use `-i`/`-t`)
- Guest rootfs chroot on sandbox streaming
- Product Pass / notarized daily install
- Multi-host matrix
- Graceful exec terminate / Docker exec-kill
