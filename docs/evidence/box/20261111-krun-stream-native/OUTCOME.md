# Native process stream and ordinary OCI regression

2026-11-11, macOS arm64 HVF with Linux aarch64 guest. This verifies the shared
KBox process path while KRun's engine stream endpoint is still being developed.

## Result

[Native stream result](513bc850ac9049a0aae91a6fa8f21c71/result.json): PASS.
The ignored native test was explicitly run with an owned K_HOME, the pinned
Python OCI fixture, newly built static Linux guest-init, signed shim and an
isolated copy of libkrun built with KBox's existing `blk,net` features.

The test paused consumption of 8 MiB stdout and successfully executed a separate
command before draining it. Backpressure reached the guest producer. It then
sent 2 MiB stdin, half-closed stdin, retained the exact binary stdout tail and
Unicode stderr tail, observed exactly one successful exit after a six-second
delay, and stopped / removed the execution through the existing Runtime client.

[Ordinary OCI result](../../engine/20261111-oci-stream-regression/4bf26bebcf304dd686e2ca54724fe0f9/result.json):
PASS. `scripts/e2e-engine/native_smoke.py` used an isolated runtime prefix with
the same candidate library. Pull, build with RUN, run / exec, child processes,
external Unicode / space paths with a 1 MiB file hash and rename, named volumes,
real loopback HTTP, rootfs snapshot restore, engine SIGKILL / same-execution
recovery and zero private executions passed. The native guest profile matched
the same-image direct privileged baseline. Ordinary OCI required no ACP package.

The libkrun fix and focused checks are recorded in its owning repository at
`libkrun/docs/evidence/20261111-unix-stream/OUTCOME.md`. Four focused tests passed;
the full device suite had 56 passes and two i8042 failures, both reproduced on
its unchanged baseline. Full libkrun formatting passed. Guest-init previously
passed all 155 tests and core all 35 tests, with logs retained in the companion
KRun `.build/guest-stream-*` evidence.

## Failed attempts retained

The earlier native stream run in `../20261110-krun-stream-guest/` failed because
blocking transport stalled control health. A MSG_DONTWAIT-only library test
also blocked and was sampled before termination. The first rebuilt library
omitted `blk,net` and failed guest startup; that owned home's metadata could not
be removed through normal force removal because the failed start had no
durable original PID binding. Its logs and record are retained under
`../20261110-krun-stream-guest/172f399133e64fc688ea588684e4c17f/candidate-without-block-feature/`.
That failed-start cleanup defect remains open; the successful tests' zero
inventory does not claim cleanup of that separate failed attempt.

## Explicitly not claimed

The engine data endpoint, full KRun CLI, capability assembly, three Harnesses'
long-running acceptance, final archive and performance gates are not complete.
No Linux KVM / Windows WHPX result is claimed. The native dependency changes
are uncommitted in their owning repository; no parent gitlink was updated.
