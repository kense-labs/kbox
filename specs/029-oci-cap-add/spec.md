# Existing portable guest capability additions on macOS

Status: accepted bounded installed-archive qualification; continue the next W4 gap.

The Mac adapter currently rejects even the shared portable cap_add allowlist.
Reuse that existing closed compiler for its13 names; retain the separately
qualified explicit SYS_ADMIN addition. Do not broaden the shared allowlist,
change defaults, grant capabilities automatically or reinterpret cap_drop ALL.

Prove exact bounding/effective/permitted sets; root/default inheritable/ambient
remain empty, while resolved nonroot users carry only explicit undropped
additions across exec through the existing inheritable/ambient fields. Prove
normalization/drop precedence and unknown-cap refusal before allocation. Per-UID
nproc/sigpending/msgqueue must still require explicit nonroot, empty capabilities
and no_new_privileges. Other profiles retain their current qualification limits.

Installed native acceptance must prove actual Guest MKNOD and SETFCAP syscall
effects with independent denial/drop/ALL controls, exact kernel capability
readback, init/exec/child/stream/realPTY, CLI, replay/fences/restart/stopped recovery
and Host RO preservation. Audit/write permission is not a claim of a complete
audit subsystem. Preserve source metadata captured before grants and exact owned
cleanup. No new Host devices, Guest default privileges or policy authority.

Freeze/rebuild/freshly install affected Box, fresh-extract unchanged OCI with
honest provenance, run owning/default/affected/A–F gates and record actual counts,
failures/source/dependencies/cleanup. Full W4/default cutover/live continuity/
platform promotion remain open; Windows ARM64 dependencies unsupported/no native
run, RSS unmeasured. Continue next local W4 item after this bounded acceptance.
