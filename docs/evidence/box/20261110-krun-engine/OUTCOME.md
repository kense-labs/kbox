# KRun engine control and general OCI regression

Status: partial M1 evidence, 2026-11-10. This does not complete the KRun implementation plan.

The new `kbox engine serve` endpoint composes the existing ManagedRuntimeClient
and BoxRuntimeDriver. Engine storage contains accepted intent and immutable
admission references; Runtime keeps its operation receipts and Box
ExecutionManager keeps actual execution resources. Acceptance takes the existing
per-unit cross-process operation lease. Pending apply also checks the latest
intent and exact bound specification under that lease. A completed receipt can
still replay without dispatching another provider effect.

The journal has a lifetime single-writer lock, validates its instance index,
retains corrupt input, and refuses further delivery after uncertain persistence
until reopened. The same-user Unix endpoint uses a private parent, socket mode
0600, peer credentials, bounded control frames and connection budgets. One peer
disconnect does not terminate the listener.

## Native evidence

The final control smoke used the freshly built release binary with SHA-256
`597fe5f70df4f9a9df4d7094501081024f88e9576e6b2d519f45c3f3f54f1d64`.
The script verifies that the binary digest is unchanged throughout the run.
Host: macOS/arm64; guest: Linux/arm64 MicroVM; shared fixture base:
`krun-coding-fixture:0.1.0`. All image builds and execution use kbox.

Command, from the kbox repository:

```sh
python3 scripts/e2e-engine/native_smoke.py --kbox target/release/kbox --image-store docs/evidence/box/20261110-dsh-acp-long-tasks/k-home/images --output docs/evidence/engine/20261110-control-native
```

The final [result](../../engine/20261110-control-native/51057dee281e4dad9dd5141c8f3d49c3/result.json)
and [command log](../../engine/20261110-control-native/51057dee281e4dad9dd5141c8f3d49c3/commands.json)
record a PASS with:

- Registry pull; native image build including RUN and a file oracle.
- Ordinary OCI run and exec without ACP, a Harness manifest or KRun.
- Real child process execution; explicit Unicode/space host mount; 1 MiB binary
  write and rename, with independently verified SHA-256.
- Named-volume persistence, real loopback egress to a host nonce server,
  stopped-rootfs snapshot, restore and file verification.
- Accepted engine service surviving engine SIGKILL; engine identity, operation
  reference and provider execution identity retained after restart.
- Real down/removal followed by old up replay and a first-delivery stale up
  rejection; inventory remains empty.
- Explicit native guest policy compared with direct `kbox run --privileged` on
  the same image and CPU/memory: identical CapEff/CapBnd, NoNewPrivs=0,
  Seccomp=0, and a real bubblewrap user namespace succeeding with exit 0.
- Zero private execution inventory after cleanup. Test volumes and snapshots
  are removed. Cached images, test data and receipts remain as evidence.

Earlier attempts remain under the same evidence directory. They include test
driver argument/schema mistakes, one unlocalized guest assertion failure, and
an earlier complete control smoke. The unlocalized assertion was not reproduced
by the retained diagnostic; the final test adds assertion context and preserves
the original conditions. These earlier attempts are not counted as passes.

## Source checks

Actual logs and command exit codes are indexed in `checks.json`. Focused engine,
profile and unary/fence checks use the real persistence/client/provider mapping;
the hermetic backend does not establish native support. Workspace check and
Clippy complete. Existing warnings are retained in the logs.

The full `kbox-contract` suite has an existing failure in
`rmcp_fix_001_uses_only_the_generic_runtime_service_contract`: HEAD fixture bytes
hash to `9f93e95b92f0cdb8860407d7dc7676e78ae0f82e464bba736604e1c9c0472eb2`,
while its locked assertion expects
`f7e7b84867c5c646ad5a02d9e7be65a23ce7f6eb813ae1e1bf4786d4db602f4a`.
Neither the fixture nor the assertion was changed. Full workspace formatting
also reports existing unformatted files; formatting-only changes outside this
task were reverted. Scoped source formatting and diff checks are recorded
separately, without claiming the full formatting gate passed.

An additional provider-side process bridge reuses the same generation and
ownership checks as unary exec, then calls `ExecutionSessionManager::start_process`.
Its five focused checks (including the existing unary checks) preserve arbitrary
bytes, separate stderr after stdin EOF, one exit event and exact execution
identity; keyed unary replay IDs are refused on this non-replayable path. See
`stream-bridge-checks.json` and `process-bridge.log`. This source change follows
the native binary above; the provider stream bridge has hermetic coverage only.
Guest default-timeout, unbounded queues and public engine flow control remain
the next integration work, not accepted stream behavior.

## Explicitly not claimed

No process-stream capability is advertised yet. KRun engine-client integration,
raw ACP streaming, MateHarness assembly, prewarm integration, credential delivery,
five capability classes, complete in-flight create/replace fault injection,
multi-instance ownership, three formal Harness task matrices and performance
budgets remain unaccepted. No Linux KVM, Windows or other native target is
certified by this macOS run. The guest-policy probe does not establish complete
Harness capability equivalence. The existing MCP fixture gate must be resolved
with its owning cross-repository contract before full acceptance.
