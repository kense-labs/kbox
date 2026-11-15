# Root read_only intent on explicit Mac OCI execution

Status: bounded acceptance complete; source/archives/native/cleanup closed. W4 common control gap. Existing
qualification rejects config.read_only; portable base hardcodes readonly=false
and Mac completion leaves it unchanged. Existing OCI Guest InitPlan/rootfs
finalize already enforces/readbacks root readonly before workload release.

Scope: connect Box intent/preflight/standard OCI root.readonly through existing
Mac profile/executor, no new mounter/dependency/config/state/native changes.
Actual native calibration found rootfs-image v1 explicitly writable-only. Add
required rootfs-image v2 for readonly Guest views, retaining strict v1 semantics
and exact version/OCI-root binding; old Host must refuse before preparation.
Rebuild affected SDK/Host/Agent/system image as well as Box; native library and
firmware selection remain unchanged. This versioned data contract is necessary
to close the proven admission gap, not a second configuration or mount system.
Preserve BoxVm guard013/pathguard014/tmpfs015/file/named/anonymous semantics,
resource authority, exact persisted route/identity/generation. Other profiles
remain unqualified. Guest root RO is separate from Host inodeRO/physical diskRO,
and does not revoke explicit Guest SYS_ADMIN. Unsupported combos refuse, no
silent fallback.

Native: root creation/modification/truncation/delete/rename/link/child writes
refused with actual EROFS before init/exec; stdin/process stream/actualPTY/File
and filesystem management paths inspected/tested under their contracts. Writable
tmpfs and caller/named/anonymous mounts retain explicit access. Repeated restart
and stopped owner recovery retain ID/new generation, inner tmpfs resets, caller
data retained, stale writes refused. Negative and positive writable controls,
installed CLI/real loader/assets, before proof, actual counts, verify-commit,
owned resources and final evidence required. Rebuild affected archive after
source changes; no default/support/readiness/GA promotion. Continue W4 after
bounded closure.
