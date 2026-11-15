# Box CPU-time limits on explicit macOS OCI execution

Status: bounded020 accepted; final installed evidence in
[OUTCOME](../../docs/evidence/runtime/20261115-oci-cpu-time/OUTCOME.md).
Existing W4 process-limit gap; continuous work proceeds to memory limits.

Admit the existing Box `cpu=SOFT:HARD` intent through the standard OCI
`RLIMIT_CPU` executor for init, exec, children, streams and real PTY. Units are
per-process CPU seconds, including all threads, not wall time or an execution's
aggregate quota. Preserve the existing CPU cgroup controls. Require positive
finite seconds or the existing `-1` infinity sentinel, soft <= hard; reject zero
instead of relying on Linux's implicit one-second reinterpretation.

Reuse the shared Core parser, Mac process completion and existing Agent
setrlimit/readback. No new ABI, capability, schema, config, resource authority,
dependency, workspace or native source change. Omission keeps existing bytes
and defaults; nofile/fsize/core and legacy names/values remain intact. Routes,
identities, operation replay, cap_drop ALL and generation fencing stay exact.

Prove real SIGXCPU at the soft limit, SIGKILL at the hard limit after caught
SIGXCPU, independently measured child CPU consumption, inherited child limits,
hard-raise EPERM, idle wall-time control and unlimited positive control. Cover
all process paths, CLI, independent execution, three generations and stopped
recovery. Final installed assets, owning checks, verify-commit, affected native
regressions/A–F, exact resource cleanup and preserved failures are required.

Full W4/default cutover/live continuity/platform promotion remain open. Windows
ARM64 dependencies are unsupported and have no native run. No RSS claim.
