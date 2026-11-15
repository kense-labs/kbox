# Extend the existing per-UID boundary

Core guest_rlimit.rs already parses sigpending and maps Linux resource11.
Agent executor/rlimit.rs already maps RLIMIT_SIGPENDING and applies/readbacks it
before credentials. Box vm/oci_hvf_rlimits.rs currently refuses the type; its
existing NPROC UID/capability/NNP boundary is also needed to keep one signal
accounting identity across every admitted process path. CLI's existing
configured_process_limit_user already reaches capture/health/PTY/attach; extend
its typed resource match rather than add another caller or metadata source.

Callers: static vm::oci_hvf_spec::validate/completion, session capture/stream/PTY,
CLI exec/health/attach. Tests: existing spec and production-ulimit tests, CLI
session defaults, one existing-native-harness sibling and a static C syscall
probe with owned children/queued signals. No configuration/export change.

First retain admission/completion/CLI red results. Add the standard mapping and
share the existing per-UID checks; preserve nproc-only Host capability/schema
requirements. Linux6.12.76 kernel/signal.c is the primary contract: realtime
sigqueue allocates against target real-UID ucounts and target soft limit;
ordinary kill retains delivery semantics without queue info at exhaustion.
Measure actual effects, including a forked same-UID receiver and queue drain,
not only getrlimit or protocol fields. Bound every probe loop and reap children.

Run focused owning tests/fmt/Clippy and verify; freeze Box source/client/archive,
reuse unchanged OCI022 only with source equality/original provenance and a fresh
isolated extraction. All native workers/libraries must come from these installs.
Rerun affected nproc/CPU/memory/oldlimits/Host/file/Secret/A–F only as justified
by changes. Strict audit all failed/successful homes/FDs/sockets/cache/RAM and
preserve source canaries. Actual counts/results and Explicitly not claimed in
OUTCOME. After acceptance continue the next existing local W4 gap.

Constitution Check: one ExecutionManager authority and existing fencing,
fail-closed unsupported profiles, KCL only, one workspace/local APIs, no new
state/dependency/submodule edits, native effects and retained failures. No
non-negotiable exception or external operation is needed.

Final acceptance: source3/Box3 with unchanged fresh OCI022, main3/verify3 and
strict14-home audit pass. Retain actual2 native regressions with full payload
equivalence; no gratuitous rerun. Two fixture failures are retained and fixed:
use real guest absence test for ENOENT and the existing explicit OCI client for
a new zero control after LegacyOnly stopped recovery. No product routing or
quota assertion changed. See OUTCOME for counts, hashes and cleanup.
