# Bounded queued-signal limits

Status: accepted on this existing Mac arm64 host;
[final installed evidence](../../docs/evidence/runtime/20261115-oci-sigpending/OUTCOME.md).
Full W4/default cutover/platform gates remain open.

Carry existing Box sigpending intent through standard OCI RLIMIT_SIGPENDING and
the existing Guest executor on explicit macOS OciSdk. Require the same explicit
numeric nonroot UID, capability-free/no-new-privileges entry boundary already
qualified for nproc. CLI capture/health/PTY/attach must supply the authoritative
configured UID when omitted for this per-UID limit; raw None remains root and
refuses. Unrelated defaults, routes, request bytes, identity/fencing and existing
nproc admission remain unchanged.

Preserve standard zero/infinity/u64 soft/hard semantics, malformed/duplicate/
inverted refusal before allocation. Prove actual realtime sigqueue EAGAIN,
delivery with queued payload/order, drain/reuse, same-real-UID aggregate queue
accounting, hard-raise EPERM, independent unlimited control and instance
isolation. Cover init/exec/child/stream/realPTY, CLI health/attach/restart, replay,
normal restart and stopped Host recovery. Explicitly test that ordinary kill
delivery remains available at a full queue: this standard limit is not a ban on
signals or a replacement for process/cgroup controls.

Reuse Core parser, Box rlimit helper, OCI transport and Agent setrlimit/getrlimit.
No schema/capability/version/dependency/native source change unless an actual
new compatibility gap is proved. Kernel already implements this resource; do
not add a signal proxy/counter or silently fall back to another backend.

Final frozen assets, isolated archive installs, affected native/A–F regressions,
actual nonzero counts, verify-commit and owned cleanup precede acceptance. Keep
all failures. Full W4/default cutover/live continuity/external gates remain open;
Windows ARM64 dependencies unsupported/no native run; RSS unmeasured.
