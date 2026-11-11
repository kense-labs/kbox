# Native stream framing under socket pressure

2026-11-11, macOS 26.6.2 arm64 / HVF, Linux/aarch64 guests. The independent
OCI regression passed in 146.53 seconds using a KBox-only prefix and the
unchanged 512 MiB default guest. It covered pull, COPY/RUN build, run/exec,
external Unicode files, named volumes, HTTP, live export excluding mounts,
commit and rerun, snapshot restore, engine restart with the same execution
identity, stale intent refusal and zero private executions.
[Result](../20261111-frame-pressure-oci/80aef5243feb4f1b8443ff919af2e356/result.json)
· [Invocation](checks/frame-pressure-oci-command.json)
· [Runtime hashes and ABI](checks/frame-pressure-prefix.json)

The same common base contains the measured bubblewrap dependency. The guest
probe ran bubblewrap successfully. Its effective/bounding capability masks,
NoNewPrivs and seccomp state matched the direct native baseline. No task,
turn, tool or PID restriction was added to obtain this result.

## Root causes and repairs

The libkrun Unix socket sender used the descriptor capacity instead of the
packet's declared payload length. Its focused regression failed before the
repair and the seven Unix proxy tests passed afterwards. Polling updates are
also consumed while holding the same proxy mutex as the operation producing
them; an older event can no longer clear a newer writable registration.
[Before](checks/native-sandbox-vsock-before.log)
· [Focused tests](checks/native-sandbox-vsock-tests.log)

The subsequent raw export retained an ENOMEM source error inside an unfinished
data frame. Guest frame writes now preserve partial progress and reduce the
write chunk on socket allocation failure. A failed archive transport closes
the stream instead of appending another frame; source archive failures emit
one error frame. New tests cover short writes, allocation failure, write-zero,
transport failure during tar finalization and a source failure.

All 192 Linux guest tests passed inside KBox in 33.82 seconds, with an empty
final private execution inventory. A previous run passed 191 tests and exposed
a test that assumed the image had no `node` account. That test now uses an
explicit empty rootfs fixture, preserving named image-user resolution.
[Final native tests](../20261111-frame-pressure-guest/69ae2ccebcaa4048835532b8255eda26/tests.stdout)
· [Exact command and hashes](../20261111-frame-pressure-guest/69ae2ccebcaa4048835532b8255eda26/command.json)
· [Cleanup](../20261111-frame-pressure-guest/69ae2ccebcaa4048835532b8255eda26/cleanup.json)
· [Earlier failure](../20261111-frame-pressure-guest/974feddf2e4d48e489375ad5956545e1/tests.stdout)

Host guest-library tests passed 156/156. Scoped guest clippy, workspace
all-target checking, the Linux release build and Linux test build passed.
The rebuilt libkrun retains `blk,net` and all 60 prior exported symbols.
[Host tests](checks/frame-pressure-final-guest-test.stdout)
· [Clippy](checks/frame-pressure-final-guest-clippy.stderr)
· [Workspace check](checks/frame-pressure-final-workspace-check.stderr)
· [Library build](checks/native-sandbox-libkrun-full-release-exit.json)

## Explicitly not claimed

The full devices suite has 57 passes and two existing i8042 failures; it is
not reported as fully passing. The current workspace-format comparison has
the same 50 pre-existing paths and no new paths. Native verification here is
macOS HVF, not KVM or WHPX. Earlier malformed/stalled export runs remain in
`20261111-common-base-oci` for diagnosis.
[Devices suite](checks/native-sandbox-devices-all.log)
· [Format comparison](checks/batch-down-kbox-fmt-comparison.json)
