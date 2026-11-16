# Spec045: published pool retains in-progress boot owners

boot_or_restore currently creates a raw VmManager, awaits boot/auth/bootstrap,
then returns it for registration. Cancellation/error before return drops its
actual handler/allocations. Register in the existing retiring collection before
first async boot and carry existing CompletedBoot/guard to foreground and batch
callers, without a second registration. Restore failure must keep typed close
refusal/actual owner and forbid cold fallback before physical finalization.
Proven prelaunch or fully finalized failures must still release owners/admission.
No new lifecycle/config/dependency/workspace/native edits; preserve authority/
identity/routes/generation/default isolation.

Acceptance: real helper cancellation/drop red then green, typed close/retry and
clean-error admission controls, focused/full/default/CLI/Clippy/fmt/frozen rebuilt
fresh install. Native controlled prelaunch gate lets caller lock actual handler
destination AFTER boot precheck, then actual VmController launches VM. Cancellation
while044 pending publication waits must retain actual pool manager/PID/allocation/
live writes; real drain physically stops it/noLate/cleanup. No native orphan or
production test hook; wrapper gates only before actual provider acquisition.
Linux snapshot native, unpublished pool/caller lifetime/providerinner/from_pid/
legacy/capacity/full W4/default OCI cutover/GA/RSS/platforms remain open.
