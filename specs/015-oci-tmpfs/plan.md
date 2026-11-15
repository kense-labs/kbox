# Reuse the shared parser and OCI mount executor

Trace: BoxConfig.tmpfs → dedicated VM qualification → VmManager runtime-owned
ext4 bundle → vm/oci_hvf_spec.rs complete → public SDK/Host → Agent MountPlan.
Guest parser currently lives in linux/mounts.rs; move/reuse it from existing Box
Core so the legacy Guest and portable producer share admission. Adapt flags/data
through existing semantics, not a second parser/framework or mount service.
Check image anonymous allocation planner so explicit tmpfs does not allocate a
hidden VOLUME. Keep persisted identity/binding/generation logic unchanged.

Establish missing spec mount and actual preflight rejection first; then minimal
shared implementation/callers/tests, native artifacts and cleanup. Reuse exact
existing executor capabilities; only change OCI code if evidence proves a gap.
Constitution I/II/V/X/XV failclosed/oneauthority/effects/unchanged gates, III/IV
real retained native evidence, VI/VII one workspace/KCL/no unnecessarydeps,
XII/XIV native boundaries/no promotion. No waiver.
