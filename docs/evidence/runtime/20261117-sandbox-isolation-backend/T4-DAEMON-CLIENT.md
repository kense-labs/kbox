# T4 — Daemon Docker-API client + SandboxLocalExecutionBackend start path

Date: 2026-11-17  
Wave: sandbox-isolation · Spec050  
Branch: `main` (P17)

## Landed

- `kbox-sandbox-engine` lib `docker_http` module: minimal Docker Engine API
  client over UnixStream (HTTP/1.1) — create / start / inspect / kill / remove.
  Socket path is always caller-supplied (`docker_socket_path(home)`).
- Container id persistence helpers: `write_container_id` / `read_container_id`
  under `box_dir/sandbox.container_id`.
- CLI-heavy deps feature-gated behind `cli` (default on for the binary).
  `kbox-runtime` depends with `default-features = false` so the adapter path
  does not pull ureq/rustls/image/proto/vmm into its link set.
- `SandboxLocalExecutionBackend`:
  - `start`: fail closed if daemon socket absent/unreachable; else create+start
    from BoxRecord image/cmd/env; persist container id; return handle with
    `exec_socket_path` = daemon socket (no host PID claim).
  - `inspect` / `kill` / `kill_with_status` / `stop_for_restart` against daemon.
  - Never falls back to BoxVm / OciSdk.
- Hermetic tests: request encoding, absent-socket fail-closed, fake Unix HTTP
  create+start+inspect (engine lib + runtime adapter).

## Commands

```bash
export K_HOME=$(mktemp -d /tmp/kbox-t4-daemon-XXXXXX)
cargo test --locked -p kbox-sandbox-engine --lib
cargo test --locked -p kbox-sandbox-engine --lib --no-default-features
cargo test --locked -p kbox-runtime --lib sandbox_backend::
python3 scripts/check-sandbox-brand.py
cargo fmt --all -- --check \
  crates/box/sandbox/engine/src/lib.rs \
  crates/box/sandbox/engine/src/docker_http.rs \
  crates/box/runtime/src/local_execution/sandbox_backend.rs
```

## Results

| Check | Result |
| --- | --- |
| `kbox-sandbox-engine --lib` | 6 passed |
| `kbox-sandbox-engine --lib --no-default-features` | 6 passed |
| `kbox-runtime sandbox_backend::` | 5 passed (route, fail-closed no-downgrade, refuse wrong route, hermetic preflight, fake unix start+inspect) |
| `check-sandbox-brand.py` | clean |
| fmt (touched) | clean after `cargo fmt` |

`K_HOME` for the verification run: isolated under `/tmp/kbox-t4-daemon-*`.

## Blockers / honest gaps for T5

- No native smoke against a **live** fused sandbox daemon (image pull, real
  OCI workload, guest isolation). Fake Unix HTTP proves framing + adapter
  wiring only.
- Pause/resume still Unavailable.
- Product/CLI default remains MicroVM — do not flip until live start is green.
- Engine **binary** (`cli` feature) still has pre-existing UI stub compile
  errors on this host; not required for the runtime adapter lib path.
- Session/exec attachment for sandbox (Linux `require_running_record` PID
  check) is not solved; start returns `pid: None` by design for this
  isolation.

## Explicitly not claimed

- Live daemon OCI workload end-to-end
- Default isolation is Sandbox / T5 flip
- Product Pass / GA
- Windows/WSL sandbox daemon support
- Engine CLI/UI binary builds clean on all targets
- Completion of unrelated `049-pool-oneshot-discard` WIP
