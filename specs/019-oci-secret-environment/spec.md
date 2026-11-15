# Bounded managed Secret environment delivery on explicit macOS OCI

Created2026-11-15. Status bounded phase accepted; actual installed evidence in OUTCOME.md. Existing W4 local scope.

Deliver caller-authorized environment values from existing BoxTransientSecretStore
RAM files through ExecutionManager→persisted OciSdk→public SDK→independent HVF
Host→existing Agent process executor, without plaintext in host disk, config,
receipts, snapshots, Debug or evidence. Source owner/mode/identity and normal /
stopped recovery remain exact. Consume and remove the existing non-sensitive
K_BOX_SECRET_ENV_V1 manifest before init/exec/child/stream/realPTY launch.

Reuse memory-secret-files with explicit required version2 and Agent manifestv9;
version1/manifestv8 retains files-only meaning. Missing/old Host, old shim/Agent,
unbound variables/paths, caller mutation, malformed/oversized/duplicate data,
physical backing and unsafe raw combinations refuse before workload entry.
No fallback, new product config/workspace/dependency/state machine or native edit.

Reads for exec must stay anchored to protected exact authorized sources, even
if a privileged workload replaces its visible Secret path. Prepare pinned RO
sources before entering workload namespaces; consume bytes only after Start,
keep source descriptors CLOEXEC and plaintext out of serialized process plans.
Preserve existing Native Sandbox Secret v1 metadata checks.

Test synthetic values/hashes only, all five process paths/CLI, independent
identities, source/path substitution, replay/conflict and stale fencing, restart/
stopped recovery, compatibility and source-preserving cleanup. Owning tests,
scopedClippy/fmt/verify, rebuilt archives/isolated install/native qualification,
resource audit and evidence required. Full W4/default cutover/platform promotion
remain open. Windows ARM64 current dependencies unsupported and not native run.
