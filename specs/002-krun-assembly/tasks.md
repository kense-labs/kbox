# Tasks: KRun environment assembly

- [x] A01: Directory mount reference/kind, optional authorized Box port, storage
  and capability wiring; refusal, lifecycle and native live-file tests.
- [x] A02: Trusted guest installer reuses KRun validator, bounded streamed ZIP,
  mode/link preservation and private atomic publish; shared base build.
- [ ] A03: MateHarness orchestration through existing exec authority, exact
  provider/generation receipt and repeat/interrupt/stale-result tests.
- [ ] A04: Compatible environment prewarm and dirty-VM retirement.
- [ ] A05: Memory-only credential delivery, native auth refresh owner and cleanup.
- [ ] A06: Formal CLI full command surface and original-request recovery.
- [ ] A07: Three actual ZIP/model closures, external files, no ordinary OCI
  regression, isolated-prefix evidence and truthful implementation status.

Each item includes callers, exports, fixtures and owning checks. No whole-phase
completion is inferred from a subset. Full KRun M0–M5 requirements remain intact.

A01 evidence: `docs/evidence/engine/20261111-assembly/OUTCOME.md` (native macOS/HVF).
A02 evidence: the guest installer, shared pinned OCI base and actual production
host-to-guest package streams cover all three ZIPs. See
`docs/evidence/engine/20261111-mate-assembly/OUTCOME.md`.
A03 has immutable provider/generation-bound receipts, primary gating and
ready-state engine SIGKILL recovery. In-flight interruption/replacement and the
complete fault matrix remain open. A06 has the core public CLI and original
request recovery; doctor and complete batch/signal/fault behavior remain open.
Prewarm, memory credentials, generic runtime dependency readiness, real-model
closures and all M3–M5 gates are not inferred from initialize/EOF success.

A04 uses canonical reservations and engine scheduling with the accepted boot
policy, directory identities, capability bindings, provider assets and engine
authority. On HVF, the engine restarted during spare boot and Dsh claimed the
same VM at the next generation. The test covers public access before claim,
live Unicode and space directories, dirty-VM retirement, replenishment and
an independent OCI workload in the same KBox home. Down confirmation waits
for the instance's unclaimed reservations after the Runtime unit is removed.
The remaining crash gate covers loss of the boot worker before native identity
persistence.
[Engine prewarm evidence](../../docs/evidence/engine/20261111-prewarm-schedule/OUTCOME.md)

A05 now delivers an opaque launch bootstrap through the generation-bound
primary control connection. The native probe covers environment and tmpfs
file projection, legacy guest refusal, raw ACP and cleanup. All three actual ZIPs
passed initialization and cleanup with synthetic credentials on the same KBox
and libkrun artifacts as the independent OCI run. A05 remains open for ownership
and persistence of native credential refresh across generations.
[Credential bootstrap and OCI evidence](../../docs/evidence/engine/20261111-credential-bootstrap/OUTCOME.md)
