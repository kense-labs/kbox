# Spec009: exact caller file grants

Status: qualified for the bounded explicit macOS arm64 profile; see
[acceptance](../../docs/evidence/runtime/20261114-oci-files/OUTCOME.md).
Owners: Box Runtime, OCI SDK/protocol/runtime/shim/Agent.

Implement ordinary caller file RO/RW on the explicit macOS independent HVF path.
Grant a pinned regular-file incarnation, preserve live bytes and caller ownership,
mode and source pathname, and expose no caller parent/siblings. RW permits data
write/truncate, not link/unlink/rename/chmod/chown/parent changes. RO is enforced
in the Host even after privileged remount/fresh mount/child and preopened FDs.

Bind source identity to the existing Box reservation/resource plan, SDK storage
identity, immutable config and target generation. Retain source identity across
restart/reconstructed clients and stopped recovery; a replacement pathname cannot
be reauthorized. Old manifests/rootfs-image v1/Exec v1–v6/v7 keep semantics.

Use real hermetic and native tests, retained failures, final isolated Box/OCI
archives and exact cleanup. Source files and other executions survive revocation.
Cross-filesystem projections, preexisting hardlink/symlink aliases and unsafe
overlapping directory grants may refuse before VM effects; never copy as live data.
Native changes stay in the existing native repository’s separate linked worktree,
with base/patch/full source and exact binary provenance; no parent submodule
source edit or gitlink/commit change. No new crates/state manager/config, default
or readiness changes, real ~/.kbox, commit/push/publication.

This is the first stage of continuous W4 local work, not the session end.
After acceptance, proceed to the next existing local gap in W4-LOCAL.md.
