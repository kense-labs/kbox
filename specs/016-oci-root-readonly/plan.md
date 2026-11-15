# Reuse OCI rootfs finalization

Trace BoxConfig.read_only → dedicated qualification → Mac bundle completion →
standard root.readonly → publicSDK/Host → Agent InitPlan → rootfs::finalize →
remount/readback before process release. Inspect exec/PTY/File/FS caller entry
points for namespace and access consistency, preserving management contracts.
Fix failing focused tests first, then actual installed native init/exec/PTY and
inner mount controls. Use existing interfaces; change OCI only for a proven gap.
Native rootro1 proved SDK rootfs-image v1 rejects readonly=true before Host.
Keep that old assertion; required rootfs-image v2 binds the readonly root view,
new Host advertises both versions and preflight chooses the exact requested
version. Update automatic/public attachment declaration and Agent descriptor
validation; rebuild OCI/Agent/system image. Old Host capability refusal and
untampered legacy v1 roundtrip/fingerprints need explicit tests/native proof.
No duplicate rootfs guard, resource store, dependency, workspace or native edit.
Constitution I/II/V/X/XV failclosed/oneauthority/effects/unchanged gates,III/IV
retained actualnative evidence,VI/VII no extra systems,XII/XIV native boundaries
and no promotion. No waiver. Existing fresh-host/crossplatform gates remain
external; do not wait for them before other local work.
