# Existing Box shared-memory size on Mac OCI

Status: accepted before integration merge after accepted029. Owning: crates/box/runtime.

CLI --shm-size already parses bytes into ExecutionRecordPolicy.shm_size. The
VmLocalExecutionBackend already converts this intent into a size-only /dev/shm
tmpfs entry. Mac qualification currently refuses the policy and its reserved
tmpfs destination. Reuse this caller and the existing OCI mount/Guest executor.

P1: admit bounded page-aligned4KiB..512MiB requested capacity, preserving the
default /dev/shm when omitted. Explicit /dev/shm tmpfs supports size-only intent
under the same bounds; preserve1777 and rw,nosuid,nodev,noexec. Bare /dev/shm,
flags/owner overrides, other /dev targets, duplicates, caller/image overlaps,
invalid sizes and simultaneous policy/tmpfs declarations must refuse.
P2: actual statfs/statvfs capacity, POSIX shared-memory mappings/child visibility,
ENOSPC/unlink/reuse, noexec denial and sticky ownership effects must pass across
init/capture/child/stream/realPTY, CLI, independent instances, replay/fencing,
normal restart and stopped recovery. /dev/shm data is generation-local.

ExecutionManager remains sole owner, immutable request/route/generation retained.
No schema, dependency, configuration language, Host memory grant or native-source
change. Preserve cap_drop ALL, Host RO and accepted029 capability behavior.
Other platform qualification and default BoxVm remain unchanged. Refusal is a
safety boundary, not shared-memory implementation evidence.

Success: red regression then owning/default gates, final regenerated Box archive
installed to fresh prefix, unchanged OCI freshly extracted with honest original
provenance, exact native counters/result files, affected tmpfs and A–F regressions,
source/cache/resource audits and OUTCOME. Retain failures and old artifacts.

Explicitly not claimed: reserved/committed Host RAM, RSS, arbitrary tmpfs flags,
live resizing or immunity from explicit Guest SYS_ADMIN remount. Full W4/default
cutover/live continuity/fresh-host/platform promotion stay open; Windows ARM64
dependencies unsupported/no native run. Continue next local W4 item on acceptance.
