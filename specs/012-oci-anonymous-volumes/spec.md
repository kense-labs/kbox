# Image-declared anonymous volumes through explicit HVF

Status: accepted on the existing macOS arm64 host with final installed assets;
accepted011 archives/evidence retained. Overall W4 remains open.
Owner: Box Runtime + OCI Runtime Mac transport. Bounded W4 allocation slice.

Implement image VOLUME through existing Box deterministic ownership planning,
VolumeStore anonymous claim/remove and existing SDK/Agent raw ext4 storage.
Disk contents retain Linux uid/gid/mode/links/xattrs without Host foreign chown.
Box is the only allocation/deletion authority. Runtime uses Caller/DetachOnly;
identity/exact mount/generation and block serial bindings must remain exact.
Do not introduce another store/state machine/config/dependency. Preserve v1–v6
wire meanings; mixed raw storage/rootfs/files require explicit new v7 and a
required advertised SDK capability, with old Host/shim refusal.

Independent acceptance:
1. Persist exact anonymous plan before materialization. Duplicate/normalized
   declarations and explicit overrides retain existing planner semantics.
2. Fresh volume seeds from immutable layered guest tree with numeric metadata;
   later restart/reconstructed clients/stopped Host recovery retain writes and
   allocation/source identities. Rootfs replacement must not reset volume data.
3. Distinct executions cannot share unnamed data accidentally. Explicit named/
   caller mounts still override image VOLUME; cleanup deletes only own anonymous
   allocation when existing Box lifecycle authorizes it, preserves named/caller
   data and other executions. Failed/replayed creates preserve ownership proof.
4. Mac Host/shim use existing raw-disk attachment contract, pinned descriptor,
   exact serial/size/access and sandbox enforcement; RO disks remain unwritable
   under remount/fresh mount/child attacks at actual Host execution boundary.
   Old unsupported Host/shim refuses, not a directory or fallback allocation.
5. Wrong inode/size/path, aliases, writable descriptor/overlap, malformed intent,
   wrong owner/generation and partial failure refuse with retained data/proof.

Tests: small before regressions, owning/fmt/Clippy/workspace/verify-commit;
freeze/rebuild/install affected archives; actual anonymous/native disk/file/
rootfs/named/Host/A–F regressions, checked counts/results/loaders and scoped cleanup.
Evidence: docs/evidence/runtime/20261115-oci-anonymous-volumes/OUTCOME.md.
Then next W4 local item. Secrets/othercontrols/default/fullcutover/readiness,
fresh-host/otherplatform/WindowsARM64 and GA remain open, no support promotion.
No native submodule edit, real ~/.kbox, staging/commit/push/publish or broadcleanup.
