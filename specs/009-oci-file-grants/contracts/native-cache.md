# Required native live-file cache boundary

Strict installed execution exposed cached Guest inode length after a completed
Host in-place size update. Keep the failing immediate read assertion. A runtime
copy, sleep, Guest cache-drop, polling retry or workload wrapper is not a fix.

The existing libkrun C API exposes add_virtiofs and add_virtiofs2 (DAX size only).
Implement a scoped macOS add_virtiofs_uncached primitive in its own worktree:
per-device CachePolicy::Never, attr_timeout ZERO, entry_timeout ZERO. Existing
APIs/defaults remain unchanged. Shim uses the required new primitive only for
file projections; no workload/tag special cases in native source.

Native worktree: /Users/roylin/code/kense/libkrun-oci-files-38d1ee741be9, existing
libkrun repository, base 3c2889c76460d81e3fb16d6144580fd31eb1bae3. Parent native
source worktrees and gitlinks remain untouched. No commits/push/publication.
Record the uncommitted patch, added tests, full source identity and actual tools.
Do not describe the patch as landed or as an updated gitlink.

Keep the original OCI native archive, pin a separate new archive, retain the
existing OCI firmware/kernel bytes and record exact ABI/architecture/symbols.
Preserve minimum macOS 11.0 of the old library (first 14.0 candidate rejected).
Build with repository Rust 1.97.1 and no machine-local CPU tuning. Run native
library tests, positive live read/writeback + Host PEP, old wire/rootfs/routes,
final isolated archives, affected checks and cleanup. No other-platform claims.


Native owning-package gates found preexisting macOS eventfd counter failures.
The shared adapter now accumulates Rust notifications across clones, preserves
semaphore/overflow/blocking behavior and positive raw shutdown words. No i8042
assertions changed. Stale epoll test call signatures were updated to the current
implementation API. Full devices60 and utils21 passed; record this native change
in the new source/archive identity. This is not a platform-support promotion.


Product worker lifetime: the private runtime AgentSession explicitly passes
macOS --runtime-owner. Shim selects RuntimeOwner only after existing exact-parent
watchdog registration succeeds. It waits for Guest exit or the canonical owner's
shutdown/death; standalone CLI/library smoke retains its90s deadline. Startup,
protocol and cleanup timeouts are unchanged. No live recovery claim follows.
