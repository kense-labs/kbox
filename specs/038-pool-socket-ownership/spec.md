# Spec038: safe pool control endpoint binding

Owning path: crates/box/cli/src/commands/pool.rs. Bound initial endpoint binding
currently unlinks any existing path before warmup. Preserve caller regular files,
symlinks, special objects and live sockets. Recover only exact runtime receipt-bound, same-UID proven stale
sockets under a lifetime admission lock; concurrent cooperative daemons cannot
unlink each other. Reuse the existing pool autostart lock/platform pattern.
Autostart/caller lock contents are never modified. Recognized listener locks carry
a fixed32-byte version1 endpoint ownership receipt (magic+dev+ino); this is
listener ownership, not another execution lifecycle store. Locks are opened
without following links, never truncated/chmodded, and
validated against exact opened/path identity. New locks/socket mode0600.
Autostart's blocking lock and the daemon lifetime lock have distinct roles/paths
so the parent waiting for readiness cannot deadlock its child. Existing644
owner-only-writable autostart lock files remain readable-compatible. Unproven
legacy endpoints and malformed listener records refuse without removal; connect
refusal alone cannot distinguish a full live accept queue on this Mac. Explicit I/O error
kinds refuse unsafe conditions before VM boot. Existing ExecutionManager,
generation, routes, schemas and defaults remain unchanged. Stale recovery now
requires proven endpoint ownership; this intentional fail-closed compatibility
restriction prevents deletion of an uncooperative live listener with a full queue.

Independent tests exercise real filesystem/socket objects, exact bytes/identity,
symlink/FIFO/directory and unsafe lock refusal, stale recovery, active endpoint
exclusion, concurrent bind and replacement preservation. Frozen archive install
will repeat CLI binding refusal and normal pool start/status/run/stop, then
source-bound affected native regression and owned cleanup. No arbitrary hostile
same-UID namespace race, other-platform native, full W4/GA or RSS claim.
