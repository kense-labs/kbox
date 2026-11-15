# Plan: exact live file projection

Constitution: ExecutionManager's existing resource plan and durable metadata own
source identity; public SDK data contracts and existing handoff own transport and
cleanup. Typed version/capability validation refuses old Hosts. Retain all prior
source/archive/evidence. No second state machine, workspace or config language.

Reach: Box resource planning/metadata/restart validation; OCI-owned spec mount
construction (leave BoxVm staging unchanged); SDK storage/required file extension;
Agent manifest new version; Host identity-bound live projection and cleanup; shim
pinned descriptors and precise Seatbelt data grant; Agent file bind rewrite;
native adversarial fixture/harness, owning tests, fixed assets and archive replay.

Reuse existing storage Caller/DetachOnly, no-follow pins, atomic directories,
identity-bound cleanup and virtio-fs. First verify live hardlink projection plus
file-only Host PEP, without caller-parent export. /dev/fd hard-link probe on this
host returns EPERM (recorded); a pathname link may be staged privately then must
match the already pinned source before any VM publication. No copy fallback.
Require same filesystem and unaliased regular source. Record permitted runtime
projection link-count/ctime effects; source data/owner/mode are unchanged by setup.

Sequence: baseline/snapshot -> native PEP/projection -> versioned SDK/protocol ->
Host/shim/Agent -> Box durable caller -> regressions -> fixed archive native ->
cleanup/ledger -> next W4 local stage. Unsafe combinations remain explicit refusals.


Native live-data correction: the strict read-after-Host-growth test exposed
virtio-fs Auto data/5s metadata caching. A generic per-export uncached API in a
separate linked libkrun worktree selects Never/zero TTL; existing API defaults
stay unchanged. Full native tests exposed event-counter semantics and its MMIO
queue-zero caller; both repaired with retained before/after failures. Firmware
and kernel bytes remain exactly pinned. New native payload/source sidecar is
explicitly uncommitted, not a landed parent gitlink or release claim.
