# Spec034: preserve failed-boot resources until backend finalization

Default-libkrun safety correction. Shared cleanup_boot_failure takes the installed
handler, logs stop error and continues deleting box/anonymous-volume resources.
Normal destroy already retains that same owner and returns on finalizer error.
Apply that existing invariant to failed boot/restore/maintenance cleanup: restore
the exact handler slot, preserve state, source/rootfs/mount/volume/network/socket
resources and allocation identities, keep bounded diagnostics, and let the actual
owner retry. A later successful finalization permits normal cleanup. Even an
already-exited provider needs successful backend finalization; do not infer
ownership release from is_running=false. No invented terminal guest status.

Trace boot, restore, maintenance, no-launch OCI preparation and pool callers.
Preserve original boot error, generation, routing and selected original-owner
path. No duplicate state machine, queue, public signature, default/timeouts or
new dependency. This closes premature destruction while the manager is retained;
pool drop-on-error outer ownership remains explicit next work, not a claim that
all failed-stop ownership or GA is solved. Keep that overall task active.

Establish real filesystem red with existing failing handler, allocation/canary
identities and retry/terminal-finalizer controls. Native injected one-shot stop
error wraps a real installed handler, proves the VM still runs and writes while
its resources remain, then retries actual stop and proves no late writes/cleanup.
This is not a genuine OS/hypervisor stop-error qualification. Rebuild affected
archive, actual installed regressions/counts, full gate and owned-resource audit.
No real ~/.kbox, native-submodule edit, platform promotion or publication.
