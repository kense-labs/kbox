# Engine prewarm scheduling and OCI coexistence

2026-11-11, macOS 26.6.2 arm64 with HVF and Linux/aarch64 guests. The native
KRun prewarm and OCI coexistence test passed in 453.32 s with release binaries
and the pinned shared base. The private prefix records the exact CLI, shim,
guest-init, libkrun and kernel bundle hashes. [Native result](3e040e1880c1410caf086b688c2a5578/result.json) · [Invocation](checks/prewarm-access-native-retry-command.json) · [Prefix](checks/prewarm-access-prefix.json) · [Fixture hashes](checks/native-fixtures.json)

## Native observations

- Aster 0.1.0 reached `available`. The engine was killed while its spare was
  `starting` and the original worker was alive. The restarted engine retained
  its identity and observed that same reservation become `running`. Dsh 0.1.0
  replaced Aster and claimed that VM at Box generation 2 with the same PID,
  PID start time and recorded start timestamp. The previous dirty execution
  was removed and the generation-1 boot command was refused. [Commands](3e040e1880c1410caf086b688c2a5578/commands.json) · [Stale boot refusal](3e040e1880c1410caf086b688c2a5578/cli-14.stderr)
- Inspect worked on the unclaimed spare. Exec, file copy, interactive attach,
  export, resource update and rename were refused at the claim check. After
  claim, Node verified the absence of the dirty marker and both attempted
  writes, copied 2 MiB across Unicode and space paths, and spawned a child
  that wrote to the explicitly mounted external directory. The unmounted
  canary was inaccessible and the output hash matched. [Command outcomes](3e040e1880c1410caf086b688c2a5578/commands.json) · [Exec refusal](3e040e1880c1410caf086b688c2a5578/cli-7.stderr)
- Dsh returned ACP `initialize` over `krun connect`. The engine replenished
  a distinct spare. The fixture invoked `krun down` with the client stdin
  still open, then observed the client, worker and VM processes exit.
  [ACP exchange](3e040e1880c1410caf086b688c2a5578/dsh.wire.json) · [Cleanup inventory](3e040e1880c1410caf086b688c2a5578/remaining-executions.json)
- An ordinary `kbox run` workload and its named volume were in use before
  the engine started. Across the engine restart, KRun replacement and KRun
  down, the workload retained its execution ID, PID and start time. Its
  `kbox exec` found the volume marker after KRun cleanup. The fixture then
  removed its ordinary OCI workload and volume and observed zero executions
  and volumes in the private home. [Commands](3e040e1880c1410caf086b688c2a5578/commands.json) · [Native result](3e040e1880c1410caf086b688c2a5578/result.json)
- The separate OCI smoke passed with the same KBox binary and the repaired
  TSI library. It covers pull, image build with COPY and RUN, container and
  child execution, external files, named volumes, HTTP, snapshot restore and
  ordinary execution identity across engine restart. Native guest settings
  matched the direct baseline and final private execution inventory was empty.
  [OCI result](../20261111-prewarm-schedule-oci/9ef0c73adea84c3184bdacbd7394c9ab/result.json) · [Invocation](checks/prewarm-verified-oci-command.json) · [Prefix](checks/prewarm-verified-prefix.json)
- With the repaired library and updated KRun, the formal CLI fixture passed
  for Aster 0.1.0, DeepSeek 0.1.0 and Codex 0.1.1 with default prewarm.
  It exercised assembly, initialize, EOF, idempotent up/down, workspaces
  moved before cleanup and final zero private executions and volumes.
  [Three-package evidence](../../../../../krun/docs/evidence/2026-11-11-prewarm-cli-native/OUTCOME.md) · [Invocation](checks/prewarm-verified-cli-command.json)

## Follow-up defects found in native verification

The first independent OCI smoke received HTTP 200 with an empty body. A
repeat passed. The targeted comparison reproduced the truncation on both
KBox binaries using identical libkrun bytes, with 5 and 9 failures out of 64.
TSI now drains buffered reads on hangup. The replacement library returned
all 256 exact bodies. The failed smoke's private volume and execution
inventory were cleaned. [First failure](../20261111-prewarm-schedule-oci/38adcba572754d84b9d5bc17150177ea/007.stderr) · [Cleanup](../20261111-prewarm-schedule-oci/38adcba572754d84b9d5bc17150177ea/failure-cleanup.json) · [Library evidence](../../../../libkrun/docs/evidence/20261111-tsi-tcp-eof/OUTCOME.md)

The first three-package CLI run reached an accepted Aster operation and
then hit a transport error during observation. The test fixture down
removed the private execution. KRun now retries the observation of the
accepted intent after a transport error. The regression closes one read
connection and verifies the identical next observation request and retained
operation ID. [Native failure](../../../../../krun/docs/evidence/2026-11-11-prewarm-cli-native/13d005ac803a45c4a15bccaa4e7f3d3f/cli-1.stderr) · [Hermetic reproduction](checks/prewarm-cli-retry-before.log) · [Current CLI checks](checks/prewarm-cli-retry-checks.json)

## Checks

- KBox engine tests 167 passed, ExecutionManager tests 75 passed, CLI route
  tests 71 passed. The focused prewarm run includes 11 passing tests and one
  ignored hardware test. Release build, workspace all-targets check, scoped
  rustfmt and owning-package clippy passed. Workspace fmt reported 50 existing
  paths, with no new paths compared with the baseline. [Checks and commands](checks/prewarm-access-checks.json) · [Format comparison](checks/prewarm-access-fmt-comparison.json) · [Release build](checks/prewarm-access-release-final-exit.json)
- KRun tests 60 passed and three native tests were ignored in the hermetic run.
  Schema regeneration, strict clippy, fmt and release build completed.
  [Current tests](checks/prewarm-cli-retry-krun-tests.log) · [Schema](checks/prewarm-schedule-schema.log) · [Current checks](checks/prewarm-cli-retry-checks.json)
- The first native candidate reproduced an unclaimed `kbox exec` returning
  exit 0 after the engine restart. The CLI and manager now use the same claim
  check from the canonical Box record. That attempt's cleanup left no
  executions. The next attempt stopped at fixture argument parsing and removed
  its named volume. [Reproduction](e48be403e98c47aa8c24bab160e844c0/commands.json) · [Original failure](checks/prewarm-schedule-native.stderr) · [Fixture failure](cf6480ef77114d09b55d7e7f945c8e26/cli-2.stderr)

## Explicitly not claimed

The crash test targets the engine process while the boot worker survives.
Recovery after killing that worker before the native identity is recorded
requires separate evidence. The ACP exchange uses synthetic credentials and
covers initialization. Remaining acceptance includes real-model long tasks,
native credential refresh, the five capability classes, native KVM and WHPX,
isolated release archive installation and repeated cold/hot performance
measurements. The elapsed time above is the whole fixture run.
