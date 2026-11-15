# Plan: explicit Box/HVF Host Service integration

Status: implemented and natively accepted within spec007.

## Constitution check

I/V/VIII/X: existing typed SDK/PEP admission; no fallback or audit-only isolation.
II/VI/VII: ExecutionManager, persisted routes, public SDK and existing KCL remain
authoritative; no second owner state machine or configuration system.
III/IV/IX/XI/XIII/XV: retain failures and exact artifacts/counts; synthetic private
fixtures, adversarial generations/bindings, actual host tests; no weakened gates.
XII/XIV: native repositories/readiness unchanged; all claims name this host.

## Actual gap and reuse

The independent macos-hvf-host-service already owns SDK/HVF execution and
stopped-only recovery. Box's existing DedicatedVm provider/migration paths cover
KVM/WHPX, not macOS. Reuse that provider/preparation, OciLocalExecutionBackend,
public SDK, durable router and Host Service. Reuse current system-image builder
and native recovery/owner-replacement entry points rather than making another VMM.

Sequence: freeze baseline/assets -> implement focused explicit Mac composition
with refusal tests -> real Box/OciSdk mixed-route lifecycle -> fault/behavior
matrices -> full verify -> rebuild/install affected package and native replay ->
owned cleanup -> gate-by-gate W4 handoff. Root source fixes reach exports,
callers, fixtures and affected artifacts. External gaps are recorded while
independent local work continues.

Default cutover requires roadmap parity/recovery/soak/platform gates beyond
this host. Keep the explicit entry if those remain unmet.


## Completed decisions

Reuse case-sensitive APFS only for the existing OCI directory transport; leave
BoxVm's guest ext4 provider unchanged. Preserve exact handoff config0600 and
Caller/DetachOnly v2 storage classification. Reuse shared Mac host confinement
and the typed cgroup compiler. Only HVF exec receives its admitted workload
capability ceiling. Normalize unordered capabilities in existing process serde;
version only new durable operation fingerprints (v7), preserving legacy rules.
Use existing native runner stacks and canonical reconciliation for interrupted
cleanup. Handle readiness-to-drain projection death using typed Unavailable plus
exact worker terminal confirmation; never create drain or successful exit evidence.

Final package/source/native acceptance and remaining W4 gates are linked by tasks.

Final cap_drop review reused the portable name normalizer for SYS_ADMIN/ALL
precedence; four spelling variants pass and native kernel masks are all zero.
Affected Box archive was rebuilt and all installed native gates repeated.
