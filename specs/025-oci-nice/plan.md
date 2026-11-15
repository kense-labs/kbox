# Map the existing standard resource

Core already knows nice/Linux13; the Guest Agent already maps/applies/readbacks
RLIMIT_NICE. The Mac Box mapping refuses it. Add the standard mapping without
extending per-UID counter identity restrictions: kernel scheduling limits are
per process, and empty-capability root is not exempt. The existing Mac capability
allowlist excludes SYS_NICE/SYS_RESOURCE; privileged configurations refuse.
Validate those boundaries and actual effects rather than invent a new policy.

Callers already reach vm/oci_hvf_rlimits through static admission, init completion
and capture/stream/PTY. CLI exec/health/attach keep their existing defaults;
explicit nonroot requests must carry the same nice limit. Add focused tests to
existing spec/production siblings, including defaults/capability/profile refusal.
Native sibling reuses the current harness, static bounded C syscall probe and
actual CLI PTY/attach recipe. Kernel source and Linux man-pages are primary
references; record actual priority and errno, not just rlimit metadata.

Retain red admission/completion counts, implement one standard mapping, focused/
owning/fmt/Clippy/verify. Freeze Box source/client/archive, fresh isolated Box and
unchanged OCI022 installs with original provenance and native loader identities.
Run five paths/three generations, controls/UIDs/namespace/CLI/replay/recovery,
affected prior limits/Host/files/Secret/A–F; preserve failures and audit all owned
resources/caches/FDs/sockets/mounts/disks/RAM. Evidence and release/ledger refs
precede closure; then continue the next local W4 gap.

Constitution Check: one ExecutionManager authority/fencing, typed closed input,
KCL only/one workspace/local APIs, fail closed, retained native evidence, no new
state/dependency/schema/submodule edits or support/default promotion. No waiver.

Final execution: source2/client2/Box2 rebuilt after a fixture-only root-map
prerequisite correction. Kernel CAP_SETFCAP refusal is asserted; entered root
namespace still exercises the same Nice denial, and mapped nonroot success is
independently required. No capability grant or priority assertion relaxation.
All5 current Rust native +3 Python scenarios pass; no new Guest count. Unchanged
resource/Secret/Guest evidence retains024 identities with exact minimal source
delta proof, not relabelled025 executions. Verify2257/41ignored, CLI935, installer8;
10-home/95-socket cleanup and final source/asset/count review PASS. See OUTCOME.
