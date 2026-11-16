# Spec042: preserve idle owners before drain detachment

Proven source gap: drain/drain_idle and snapshot invalidation detach actual idle
managers before awaiting stats and before existing close registration. Cancellation
at that await drops actual handlers/resources without physical finalization.
Select a bounded shared lock-order correction, preserving exact counts, typed
errors/retry, selected/all scope and resource authority. No new state/config/
dependency/workspace/native edits or routing/generation change.

Acceptance: original actual-manager cancellation regressions fail then pass for
both public drain variants and invalidation; full pool/CLI/Clippy/default/fmt,
frozen rebuilt/fresh installed archive, real VM PID/allocation/live-write proof
after cancelled stats wait, actual final cleanup/AF/pool and scoped audit.
Invalidation hermetic control does not qualify Linux native snapshots. Full W4/
OCI cutover/live continuity/GA/internal boot/unpublished/legacy/RSS/platforms open.
