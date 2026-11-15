# Fail closed when Guest rootfs read-only enforcement fails

Status: accepted bounded slice; see final evidence. W4 priority1 confirmed safety gap, found while tracing
next tmpfs control. Owning paths: Box Guest Init and Runtime native harness.

Current init returns success after both mount attempts fail. Exec/PTY call a
void helper that warns and returns on bind/remount failure. Workload can then
run writable despite explicit BOX_READONLY/K_SEC_READONLY_ROOTFS. Reuse shared
Guest enforcement and propagate errors before any init/exec/PTY child launch.

Independent acceptance: reproduce current failure using real implementation;
verify success/alreadyRO/idempotence and bind/remount/stat/verification errors;
prove init/exec/PTY no launch/no marker after refusal; normal readonly root
rejects writes while explicit workspace/tmpfs mounts retain their contracts.
ExecutionManager/generation/routes remain sole authority. No new state/config/
dependency, native source, default migration or support promotion. No real ~/.kbox,
staging/commit/push/publish or broad cleanup. Preserve012 archives/evidence.

Tests: focused owning tests and Linux Guest test artifact executed in actual
isolated MacHVFGuest as applicable (cross-build alone is not native pass), normal
BoxVm/explicitOCI impacted native regressions, final rebuilt/installed affected
archives and verify-commit; exact counts/source/assets/failures/owned cleanup in
docs/evidence/runtime/20261115-rootfs-readonly/OUTCOME.md. No readonly capability
claim from refusal alone. After closure continue W4 tmpfs or higher-priority
proven unsafe permission guard. Secret/path restrictions remain separate until
actually qualified. Fresh hosts/platforms/readiness/WindowsARM64 remain open.
