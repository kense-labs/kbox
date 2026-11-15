# Narrow the existing resource and process boundaries

Core already recognizes nproc; the Agent already implements RLIMIT_NPROC.
Mac Box currently refuses it. First establish actual admission/completion red
tests. Reuse the existing numeric/root user parser from oci_session through a
crate-internal re-export, preserving its legacy defaults and numeric behavior.
The existing Mac rlimit helper validates configured nonroot UID and empty
capabilities; actual Process validation checks UID equality, empty capabilities
and no_new_privileges before mutating process limits. Both init completion and
all session paths already call this helper.

CLI exec currently passes its optional user directly; health and attach always
send None. Add one existing-CLI helper that derives an explicit default only
for persisted OciSdk+nproc metadata. Capture/PTY/health/attach share it; do not
change the manager's None meaning or default BoxVm behavior. Test metadata
authority/default preservation and explicit override rejection.

Static Linux probe -> all-process/CLI/replay/recovery native matrix -> focused/
owning checks/Clippy/fmt/verify -> frozen Box rebuild/fresh isolated installs ->
native effects and affected regressions/A–F -> exact resource/cache/FD/socket
audit and evidence/docs. Reuse OCI019 only with unchanged source-input proof,
original03a62c3e… provenance and a new isolated extraction/identity check.

Native1 exposed a real kernel setuid equality bypass. Shared Agent finite NPROC
now prepares one temporary fork slot before credentials, forks/reaps only trusted
code after UID/capability application, then restores and verifies exact soft/hard
limits before workload entry. Only raise the temporary hard ceiling if soft+1
needs it; infinity saturates. Failure to allocate this bounded admission child
refuses launch. Zero budget refuses and a one-process budget retains its last
slot. Workload PID/supervision and cgroup PID policy remain unchanged.
Because old Agent code lacks this effect, Box requires Host nproc-admission v1
before preparation. NPROC utility-VM manifests use explicit v10 with a bound
v1-v9 transport schema, preserving every prior transport's validation/bytes and
blocking downgrade. Both HVF and KVM manifest producers wrap complete existing
transports; no operation protocol or resource store is added. Current/old Host,
shim and Agent installed assets must prove refusal and cleanup without Start.

Native CLI health exposed existing Host PATH lookup against a Guest-native ext4 placeholder. Resolve only HVF PATH in the existing Guest executor, carrying image/default PATH and preserving escape guards/other drivers. The bare-sh health fixture is retained and passed. The initial mutable-bootstrap decoder probe was refused by the correct live-file consistency guard; no guard was relaxed. The final immutable pre-boot SDK fixture uses the exact old Agent in a clearly marked test-only stderr/wait launcher image, linked by ELF/original manifest SHA to the separate original-image API refusal. Actual exit1/unknown field/token untouched/worker reap and socket cleanup pass. All failed calibration remains in FAILURES.md.

Final acceptance: source5/installed Box5 and freshly extracted OCI3, final-review5 PASS; CPU/memory/oldlimits4 retained only with packaged payload equivalence. See OUTCOME for exact counts and cleanup. Initial OCI019 reuse was superseded by a new OCI3 build after the real shared-Agent admission fix. No completion claim relies on the old reuse plan.
