# Optional production Harness assembly and ordinary OCI

Date: 2026-11-11. Native host: macOS/arm64 HVF; guest: Linux/aarch64.
Result: PASS for the assembly, transport, core CLI and OCI scopes listed below.
The full KRun acceptance goal remains open.

## Product boundary and implementation

`harness-assembly.v1` is optional. Existing intents omit the field and retain
their wire identity. Ordinary OCI keeps its existing commands, mount targets,
security defaults and lifecycle; it does not require ACP, package manifests or
KRun. The package path reserves `/opt/krun` only when assembly is selected.

The engine hashes an immutable anonymous copy of the bounded archive before
streaming through the existing process authority to the trusted guest tool.
There is no host extraction or package hook execution. Guest publication is
atomic and validates existing trees before reuse. Large host I/O does not hold
the unit operation lease; supersession cancels pending work and fences late
publication. Control wait budgets do not impose a Harness task duration limit.
New assembly bootstrap requests use `sleep infinity`; old frozen requests keep
their accepted identity.

A durable assembly receipt binds the accepted request, Runtime specification
and generation, package digest and the actual Box execution identity and
generation. Primary startup checks that physical binding at the execution
boundary. Runtime owns its receipts; ExecutionManager owns resources; KRun
persists plans and request identities only.

## Native evidence

- [Three real package streams and ACP initialize/EOF](../../../../../krun/docs/evidence/2026-11-11-mate-assembly-native/OUTCOME.md)
  use the independent Rust client and the same pinned OCI base. Codex also
  survives ready-state engine SIGKILL with exactly the same receipt and
  execution while the archive is temporarily unavailable.
- [Public KRun CLI](../../../../../krun/docs/evidence/2026-11-11-formal-cli-native/OUTCOME.md)
  covers all three ZIPs, immutable repeated up, real status/logs, native ACP,
  moved-workspace exact-ID cleanup and idempotent down. Both suites require
  zero private executions and volumes at completion.
- [Ordinary OCI result](../20261111-mate-oci/10f7dae1ea84483f91357ed1e58d7993/result.json)
  passes pull, COPY/RUN using an owned private build pool, run/exec/child,
  Unicode/space 1 MiB external-file hash, named volume, actual HTTP,
  snapshot/restore and zero-private-execution cleanup. An engine SIGKILL
  preserves the same ordinary execution. The explicit native guest policy
  matches direct execution; ordinary defaults remain separate. Its exact
  binary hash is in the result and differs from the later indefinite-bootstrap
  build; the subsequent changes affect only opted-in assembly.

Exact native launch arguments and OCI stdout/stderr are in
[checks](checks/). Each native result records the tested executable digests;
the final assembly/CLI runs freeze their client executables before launch.

## Source checks and retained failures

[Reproduction commands and counts](checks/checks.json), with corresponding
logs in the same directory:

- Assembly validation, immutable copy and receipt persistence/fencing: 4 PASS.
- Exec bridge and physical execution binding: 2 PASS.
- `engine::` filter: 165 PASS, including OCI build-engine modules.
- Runtime driver engine tests: 10 PASS.
- Workspace/all-target check and owning-package clippy: PASS, existing warnings.
- Final release build and scoped changed-file rustfmt: PASS.
- Whole-workspace fmt: FAIL on 54 existing paths; baseline had 55 and there are
  [no new failing paths](checks/mate-workspace-fmt-comparison.json). No broad
  formatter rewrite was made.

Initial test compilation failed because reused test helpers were private;
crate-visible test-only access fixed that cause. The failed logs remain here.
Three unrelated shared-contract fixture/golden failures were previously
reproduced on unchanged HEAD; retained
[baseline log](checks/assembly-contract-baseline-tests.log) and
[fixture proof](checks/assembly-mcp-fixture-baseline-proof.json) are not a
passing full-workspace test claim. No expectations were weakened.
Local document links were checked. The runtime README has one unchanged
baseline link to absent older desktop evidence; it is recorded separately in
`checks/document-links.json` rather than silently counted as passing.

## Explicitly not claimed

No real-model task runs in these new package/CLI suites. Compatible running-VM
prewarm, RAM-only credentials/native auth refresh, generic package dependency
readiness, five capability classes, full in-flight crash/replace matrix,
doctor/batch/signal CLI behavior, long-task/resume/session cancellation and
M4–M5 performance/release gates remain open. Earlier ordinary OCI RUN builds
with apt-get failed on an incomplete response frame; this batch does not fix
that path. Other native platforms and a final distributable archive are not
qualified. No commit, push or global runtime installation was performed.
