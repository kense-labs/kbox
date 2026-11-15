# Tasks: macOS host-enforced read-only mounts

- [x] E1 Read constraints/A–D and trace mount, router and actual host entry.
- [x] E2 Retain reproduction; test native write/link/replacement/child boundaries.
- [x] E3 Share OCI platform enforcement and wire Box shim before VM entry.
- [x] E4 Admit supported macOS mounts; update callers, refusal paths and fixtures.
- [x] E5 Qualify real HVF privileged write attempts, live view and cleanup.
- [x] E6 Run affected focused tests/Clippy and `scripts/verify-commit.sh`.
- [x] E7 Review scoped diff and finalize evidence/limitations.

## Continuation record

Initial host: macOS 26.6.2 / arm64, rustc 1.97.1. A–D already complete,
dirty worktree preserved. Native submodules are read-only for this task.
Confirmed gap: Box shim ignores `FsMount.read_only` at host boundary; macOS
runtime refuses `:ro`. OCI already confines its worker with Seatbelt.
Implemented shared MacosReadOnlyMounts; Box and OCI use one Seatbelt rule builder
and installer. Native failures found fdesc traversal and pre-opened writable-FD
bypasses; fixed with cap-std traversal, FD admission and early confinement.
Focused native/platform/Runtime/CLI tests pass. Ordinary read-only single-file
binds now explicitly refuse before staging. No native submodule edits.

First real HVF gate: writable remount + fresh mount succeeded, 14 operations and
guest child returned EPERM, restart and live host view passed, source preserved.
Overall run failed only at the explicit-overlap diagnostic: shim refusal was
masked by missing guest baseline. Shared Runtime preflight now refuses before
provider launch, with focused typed regression. First run cleaned to zero;
raw failure/probes retained. Harness now retains per-box logs before deletion.

Final native_readonly gate PASS (16.57 s): source-specific link denial also
preserves RW hard links; physical APFS aliases and pre-opened alias FDs refuse.
Runtime directory native fixture PASS (1 executed test, 15.53 s), including RO
remount, generation fencing, moved-source retirement and exact cleanup. The
first fixture invocation selected zero tests; it is explicitly not accepted.
Native OCI/primary/prewarm baseline regressions all PASS with the same frozen
companions. Full verify-commit PASS (259.45 s): Runtime 2165/0/12 ignored,
CLI 933/0, all default contract tests, check, fmt, scoped Clippy, installer 7/7.

RAM fixture cleanup root cause fixed: Disk Arbitration does not list the manually
mounted HFS volume; use actual /sbin/umount path then hdiutil detach and assert
both statuses. Owned disk12/disk13 removed; seven older foreign fixture devices
were inventoried and left untouched. No submodule content or gitlink changed.
Final inventory and scoped review complete; shared platform 69 pass, shim 30
pass, support-matrix check pass. E OUTCOME is final. Next: F source/API research,
native architecture admission and available Windows ARM64 checks.
