# Spec032: cold warm-pool admission failure cleanup

Owning path: kbox-runtime pool/warm_pool.rs and the existing VmManager lifecycle.
Base is recorded in current evidence context. Goal contribution: default libkrun
must not leak an already booted VM when idle admission fails.

## Actual gap and acceptance

The common cold branch used by on-demand and replenishment boots calls boot(),
then wait_for_exec_available(120s) and Ready publication with early error exits.
VmManager has no Drop reaper; those exits currently discard the running handler.
The existing process-prewarm error and restored branch already request teardown.
Original intermittent parallel15s boot timeout remains separately unlocated.

1. Lost/missing exec endpoint after a boot must retain the original typed refusal,
   publish no idle VM, and request exact owned teardown before returning.
2. Successful authentication and optional process-prewarm retain current behavior.
   Every post-boot error follows the existing destroy_manager_or_reap path; no
   fresh resource owner/store, backend fallback, new timeout or configuration.
3. Hermetic tests prove old failure and actual lifecycle cleanup of handler,
   private directory/socket/volume effects; a positive authenticated Frame control
   proves valid VM admission. Cleanup-stop failure must remain visible and never
   become a claim of clean teardown.
4. Current installed Box archive native test: boot a real VM in an isolated home,
   remove only its exact owned exec endpoint to force the post-boot check to fail,
   retain the existing120s admission bound, observe actual process termination /
   no late writes, and verify source-preserving cleanup. Run affected A-F / pool
   concurrent controls with installed assets and inspect actual counts/results.
5. Preserve old archives/failures, run focused/full affected checks and
   scripts/verify-commit.sh, record native assets/source identity and own cleanup.

## Boundaries

ExecutionManager/VM authority, persisted identity/generation, libkrun default,
platform support and current timeout values remain unchanged. No VMM/firmware
source edit, extra workspace/dependency/configuration/state machine. This does
not certify original timeout cause, GA, long soak, full W4/default cutover,
Windows ARM64, fresh hosts or runtime RSS. Existing disk/Git external blockers
stay explicit and are not blind-retried. All unsafe-resource cases refuse rather
than report healthy idle membership.
