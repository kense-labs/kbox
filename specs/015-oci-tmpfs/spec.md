# Box tmpfs intent on explicit macOS OCI execution

Status: accepted with final installed archive and zero-resource audit. W4 priority2 common control gap.
Current qualification rejects all config.tmpfs; HVF portable completion ignores
that field. Existing OCI Guest executor already supports tmpfs and complete mount
options. Existing Box Guest parser owns path/access handling but is private.

Scope: shared existing tmpfs parse semantics and necessary platform adaptation;
Mac preflight, bundle caller, image VOLUME override planning, standard OCI mount
output, actual Guest effects and cleanup. Preserve BoxVm behavior/route, named/
anonymous/caller storage identity, resource authority and generation fencing.
No new workspace/dependency/config/state machine/native code or new Guest mounter.
Use the existing OCI executor. Other platform qualification/support remains as-is.

Accept bounded ordinary tmpfs targets/options with explicit RO/RW, size/mode/UID/GID
and common safety flags where both consumers enforce them. Unknown/unsafe or
unqualified combinations refuse before preparation; never silently ignore intent.
Root/runtime/duplicate/conflicting destinations, data/flag confusion and image
VOLUME overrides require tests. Original kernel data options on BoxVm must not be
silently reinterpreted or removed. tmpcopyup/Secret/custom mount families stay out.

Native acceptance: filesystem actually tmpfs; data/ownership/mode, size exhaustion,
RO/child and safety-flag effects; per-execution isolation, new-generation reset,
normal restart/authenticated stopped recovery, live caller/named data retention,
CLI and exact persisted route/generation, incompatible inputs/no fallback/no
extra allocations. Verify actual test counts/results. Freeze source/assets, build
and isolate-install affected archives, repeat impacted A–F/volume/HostRO controls,
verify-commit and exact run-owned resources audit before bounded closure.

Evidence docs/evidence/runtime/20261115-oci-tmpfs/OUTCOME.md; preserve014andall
older outputs. No ~/.kbox, staging/commit/push/publication or platform promotion.
Continuous W4 task stays active after this phase; next local item comes from
remaining ledger, with confirmed correctness issues first.
