# Read-only engine diagnostics and immutable image inspection

2026-11-11, macOS/arm64 HVF host and Linux/aarch64 guest.

`engine-diagnostics.v1` adds a parameterless `diagnose` request over the existing
private same-user engine transport. It returns engine identity, product version,
platform and the existing host virtualization probe result. The blocking probe
runs outside the async executor. It submits no intent, creates no VM, and performs
no inventory reconciliation. Unsupported or unavailable conditions stay explicit;
arbitrary host error text is not returned.

The independent KRun `doctor` combines this observation with local package and
configuration checks. Guest dependencies and task readiness are not inferred from
host virtualization. [KRun contract and evidence](../../../../../krun/docs/evidence/2026-11-11-doctor-native/OUTCOME.md)

## Verification

- `cargo test --locked --offline -p kbox-runtime engine`: 187 passed, 2 ignored.
- `cargo test --locked --offline -p kbox-cli image_inspect --lib`: 2 passed.
- Owning-package Clippy, whole-workspace all-target check and release build passed,
  with existing warnings. Scoped rustfmt passed. Whole-workspace rustfmt reports the
  same 50 pre-existing paths; [exact comparison](checks/fmt-comparison.json).
- A dispatcher test verifies unchanged journal bytes and zero backend starts or
  kills. Unknown request fields are refused.
- Real KRun host diagnosis authenticated the engine, observed the native HVF probe
  and did not create a KRun HOME. Per-Harness checks and final cleanup are recorded
  in the linked KRun outcome. Aster 0.1.2, Dsh 0.1.5 and Codex 0.1.2 passed diagnosis,
  signal recovery, ACP initialization, EOF and cleanup; Codex also passed the
  post-stdio SIGINT case. Independent observation after runner and engine exit
  confirmed zero private executions and volumes.

The first native diagnosis found that read-only `image-inspect` did not resolve
`name@sha256:...` against cached content. The fix requires a complete SHA-256 pin
and deterministically selects cached content even when multiple tags share it.
Destructive image resolution remains unchanged. Unit coverage includes changed
tags, duplicate content aliases, missing content, short hashes and invalid pins.

## Ordinary OCI regression

The final signed KBox binary, guest-init, shim and native libraries were installed
in a prefix without a host KRun binary. Real pull, build with RUN, ordinary
execution, 1 MiB Unicode external-file operations, volumes, networking, export,
commit, snapshot restore and engine restart passed. Execution identity survived
the restart. Native guest privileges and sandbox behavior matched direct native
execution. After runner and engine exit, an independent query confirmed zero
private executions and volumes.

[Final OCI result](../20261111-engine-diagnostics-oci-final/7037dcb7c3864b6fab6402d29db7ff6a/result.json)
· [Independent audit](checks/independent-audit.json)
· [Commands and elapsed time](checks/oci-run.json)
· [Artifacts](checks/final-prefix.json)
· [Checks](checks/checks-summary.json)

The OCI run took 162.532 seconds; this is not a performance baseline. An earlier
diagnostics-only binary also passed OCI in 261.544 seconds; its original evidence
is retained in `../20261111-engine-diagnostics-oci/` and does not substitute for the
final binary's run.

## Explicitly not claimed

The host probe does not prove VM boot, guest dependency integrity, authentication,
remote service health or successful Harness tasks. No model inference was run.
Linux/aarch64 KRun cross-compilation is not Linux-host native acceptance. Other
host backends, the complete fault matrix and performance gates remain separate.
