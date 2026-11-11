# KBox primary streams, independent client and ordinary OCI

2026-11-11, macOS arm64 HVF with Linux aarch64 guest. This batch verifies the
engine's process channel and preserves KBox's independent general OCI path.
It does not complete the wider KRun plan.

## Native results

All results below used KBox CLI SHA-256
`2edaf24532c7b55077daa43e418d789535cb193c68dec5f2d050cf4aa0e30d68`.
The isolated prefix contains the new static guest-init and signed libkrun
candidate; [artifact metadata](engine-client-native-prefix.json) lists their
hashes. No global runtime installation or parent gitlink was changed.

- [Engine native result](47dbf2f784fe425aa52388dc5bff18cb/result.json): PASS.
  `scripts/e2e-engine/native_primary.py` exercised 2 MiB stdin, paused 8 MiB
  stdout with concurrent unary exec, binary and Unicode tails, stdin EOF, a
  six-second task and exact exit 37. Duplicate Open and Attach were refused.
  Control disconnect interrupted simultaneous input/output backpressure while
  the data socket remained open. Daemon SIGKILL/restart preserved engine
  identity, retired the old environment and did not replay the process. A lost
  Open ticket caused cleanup without launching that process. Parent/child files
  remained unchanged for **58.24 s** after disconnect and **52.14 s** after
  daemon crash. Private execution and volume inventories ended empty.
- [Independent Rust client result](../../../../../krun/docs/evidence/2026-11-11-engine-client-native/ed6837e271ef4c25ba1004fddb2962fc/result.json):
  PASS. The client has no KBox path dependency. It discovered the default
  home-scoped endpoint, verified kernel peer identity, retained raw duplex data
  and exact generation/provider bindings, and also preserved delayed stderr
  and exit 23 after guest stdin closed early. The owned home contains Unicode
  and spaces. Its [OUTCOME](../../../../../krun/docs/evidence/2026-11-11-engine-client-native/OUTCOME.md)
  records the 42 passing KRun tests and failed driver attempts.
  A later [control and frozen-dispatch batch](../../../../../krun/docs/evidence/2026-11-11-engine-control-native/OUTCOME.md)
  passed 52 KRun tests and exercised the Rust control API against this same KBox
  binary; it does not change the KBox artifact or the native results above.
- [Ordinary OCI result](../20261111-client-oci-regression/d1c55acfc56a4cb2875f52fb81891f98/result.json):
  PASS. Pull, image build with RUN, ordinary run/exec and child processes,
  external Unicode/space paths with a 1 MiB hash and rename, named volumes,
  real loopback HTTP, snapshot restore and engine SIGKILL/same-VM recovery
  passed without ACP or a KRun process. Native guest permissions matched the
  same-image direct baseline. Private executions ended empty.

## Changes and ownership

The engine retains intent/admission and stream receipts. ManagedRuntimeClient
retains operation observations; ExecutionManager remains the actual execution
resource owner. No ACP parsing, model loop, tool count or turn limit entered
the engine. `process-stream.primary.v1` is an explicit capability; it does not
replace ordinary command, PTY, filesystem or OCI APIs.

The primary uses an independent control-owner socket and a one-use data
attachment. Both use same-user peer authentication. Raw frames have bounded
sizes; task idle time remains unbounded, while control requests and incomplete
frames have independent deadlines. An input-side failure does not discard the
provider's remaining stdout/stderr and exact exit.

`kbox engine endpoint` returns machine-readable endpoint data without starting
a VM or task. The default endpoint is home-scoped and short enough for Unix
socket limits, including a long Unicode home.

VM boot now retains the fact that an error occurred before calling
`VmmProvider::start`. Startup reconciliation can publish Failed without inventing
an exit code. Ordinary CLI startup cleanup then removes the reservation. A
missing PID by itself does not establish this fact. The native independent-client
driver also deliberately supplied an invalid guest-init symlink and confirmed
refusal and empty canonical inventory/execution directories.

## Checks and retained failures

- `cargo test --locked -p kbox-runtime engine::`: **158 passed**, including
  engine and OCI build-engine unit tests.
- `cargo test --locked -p kbox-runtime kbox_contract_driver::engine_tests`:
  **10 passed**, including raw binary transport, duplicate attach, lost ticket,
  backpressured control disconnect, early stdin close and graceful shutdown.
- VM backend tests: **34 passed, 2 failed**. The new pre-launch failure and
  missing-PID adversarial tests passed. The same two failures reproduced with
  baseline backend source: **32 passed, 2 failed**.
- The `vm::tests::` filter matched 42 passing VM tests and hardware-VM tests
  with **20 passed, 2 failed**. Those two hardware failures also reproduced
  with baseline source, **20 passed, 2 failed**.
- `cargo check --workspace --all-targets --locked`: PASS.
- `cargo clippy --locked -p kbox-runtime --all-targets --no-deps`: exit 0 with
  existing warnings. The equivalent KBox CLI check also exited 0 with existing
  warnings. KRun's own all-target Clippy passed with `-D warnings`.
- Scoped formatting for the engine, engine integration tests and CLI: PASS.
  Full workspace formatting failed on pre-existing formatting; a baseline
  check also failed, with no newly affected file compared with baseline.
  `git diff --check`: PASS.

Baseline HEAD is `0304123f562ce7264954a8a104dced9e19b3d483`. Its unmodified
test target first failed to compile because an unrelated networking conformance
fixture referenced removed `BoxConfig::isolation`. The recorded
[compile repair](vm-backend-baseline-compile-repair.patch) changes only that
unexecuted fixture, allowing the focused baseline tests to run. It does not
change the four failing assertions. Full logs, original compilation failure,
red/green pre-launch reproduction and [counts](checks.json) are retained here.
These results are not represented as a fully green workspace.

## Explicitly not claimed

Complete KRun CLI, durable KRun connection admission, MateHarness assembly,
prewarm, memory-only credential delivery/refresh, all five capabilities, three
Harnesses' real-model and long-task acceptance, full crash/fence matrix,
performance budgets and final-archive installation remain unfinished.

The pre-launch proof is retained by the live owner. Older abandoned startup
records without that fact or an original PID binding are not retrospectively
declared clean; ambiguous failures after provider invocation also remain a
separate recovery task. No Linux KVM or Windows WHPX native result is claimed.
Native dependency source remains uncommitted in its owning repository.
