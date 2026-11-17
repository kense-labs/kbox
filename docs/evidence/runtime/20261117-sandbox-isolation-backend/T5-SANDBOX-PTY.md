# T5 — Sandbox PTY (`kbox exec -it`)

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

Managed Sandbox boxes returned the default-trait `Unavailable` for
`start_pty` / `start_process` (or would hang if routed to the MicroVM guest
PTY socket). Spec050 now implements
`SandboxLocalExecutionBackend::start_pty` via Docker exec create → upgrade
hijack on the fused daemon (`DockerHttpClient::container_exec_pty`).
Non-TTY streaming was landed separately (`T5-SANDBOX-STREAM.md`). Session
dispatch already uses `uses_backend_session`, so Sandbox never falls through
to guest sockets.

CLI: piped `kbox exec -it` skips raw-mode ioctl when stdin is not a TTY so
scripted smokes work without `script(1)`.

## Hermetic

```bash
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-pty-ut-XXXXXX)
cargo test --locked -p kbox-sandbox-engine docker_http
cargo test --locked -p kbox-runtime --lib local_execution::sandbox_backend
```

Observed: docker_http 10 passed (includes
`fake_unix_http_container_exec_pty_hijack`); sandbox_backend 9 passed
(includes `start_pty_against_fake_unix_http_daemon` and
`start_process_fail_closes_unavailable`).

## Native smoke

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-pty-XXXXXX)
cargo build --locked -p kbox-cli
./target/debug/kbox run -d --name ptybox alpine:3.20 -- sleep 3600
printf 'echo PTY_OK\nexit\n' | ./target/debug/kbox exec -it ptybox -- sh
./target/debug/kbox rm -f ptybox
```

Observed:

```text
PTY_OK
it_exit=0
```

K_HOME for this run: `/tmp/kbox-sandbox-pty-5uOPxV`

Documented interactive path: `kbox exec -it <box> -- sh` (use `-t` for
stdin/PTY). Non-TTY streaming: `kbox exec -i` (`T5-SANDBOX-STREAM.md`).

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Captured one-shot `execute` stdin piping (use `-i`/`-t`)
- Guest rootfs chroot on sandbox PTY
- Product Pass / notarized daily install
- Multi-host matrix
