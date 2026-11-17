# T5 — Sandbox captured one-shot `execute` stdin

Date: 2026-11-17  
Host: darwin arm64  
Branch: `main`  
≠Pass

## Gap closed

`SandboxLocalExecutionBackend::execute` previously returned Unavailable when
`request.stdin` was non-empty (message pointed at `-i`/`-t`). Spec050 now
accepts a finite captured stdin buffer on the one-shot path:

- `DockerHttpClient::container_exec(..., stdin)` — empty stdin keeps the
  plain HTTP start/demux path; non-empty uses AttachStdin + upgrade hijack,
  writes the buffer, half-closes write, drains mux stdout/stderr, inspects
  exit. Bound: `MAX_CAPTURED_EXEC_STDIN_BYTES` (16 MiB).
- Streaming stdin (`stdin_streaming`) still Unavailable on captured execute
  (use `start_process` / `kbox exec -i`).
- Fail-closed without daemon; never MicroVM guest sockets.

## CLI mapping

- Managed `kbox exec -i` (no `-t`) → streaming `start_process` (unchanged).
- Managed `kbox exec -t` → PTY (unchanged).
- Without `-i`/`-t`: captured `execute`. If stdin is a non-TTY pipe/file,
  CLI reads it into `ExecRequest.stdin` so
  `printf … | kbox exec box -- sh` works on the Sandbox default path.
  Interactive TTY stdin is left unread (no block waiting for EOF).

## Hermetic

```bash
export K_HOME=$(mktemp -d /tmp/kbox-cap-stdin-ut-XXXXXX)
cargo test --locked -p kbox-sandbox-engine docker_http
cargo test --locked -p kbox-runtime --lib local_execution::sandbox_backend
```

Observed:

- docker_http **13** passed (includes
  `fake_unix_http_container_exec_with_captured_stdin`,
  `container_exec_rejects_oversized_stdin`)
- sandbox_backend **14** passed (includes
  `execute_with_captured_stdin_against_fake_unix_http_daemon`,
  `execute_rejects_stdin_streaming_flag`)

`cargo fmt --all -- --check` clean for touched files.  
`cargo clippy --locked -p kbox-sandbox-engine --lib --no-deps` and
`-p kbox-runtime --lib --no-deps` finished (pre-existing warnings only).

## Native smoke

```bash
codesign --force --sign - --entitlements crates/box/sandbox/engine/sandbox.entitlements \
  target/debug/kbox-sandbox-engine
export KBOX_SANDBOX_ENGINE_BIN=$PWD/target/debug/kbox-sandbox-engine
export PATH=$PWD/target/debug:$PATH
export K_HOME=$(mktemp -d /tmp/kbox-sandbox-cap-stdin-XXXXXX)
cargo build --locked -p kbox-cli -p kbox-sandbox-engine
./target/debug/kbox run -d --name capstdinbox alpine:3.20 -- sleep 3600
printf 'echo CAP_STDIN_OK\n' | ./target/debug/kbox exec capstdinbox -- sh
./target/debug/kbox rm -f capstdinbox
```

Observed:

```text
CAP_STDIN_OK
it_exit=0
```

K_HOME for this run: `/tmp/kbox-sandbox-cap-stdin-uFIHwB`

## Brand

`python3 scripts/check-sandbox-brand.py` → exit 0

## Explicitly not claimed

- Product Pass / notarized daily install
- Multi-host matrix
- Guest rootfs chroot on sandbox exec
- Graceful exec terminate / Docker exec-kill
- Compose Secrets on sandbox
