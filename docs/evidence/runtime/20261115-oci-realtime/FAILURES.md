# Retained failures

Setup1 stopped before any VM/native resource when it assumed the reused OCI
archive had a `.sha256` companion. That historical archive stores hashes in
its installed SHA256SUMS and prior receipt. Setup2 records actual archive SHA
and existing companions explicitly; none is fabricated or required retroactively.

Native1 executes0passed/1failed/0ignored (32.61s native/37.08s wrapper).
Configured init exited before init.json; later exec correctly refuses because
there is no running configured process. Failure is not accepted as enforcement.
All failed execution resources cleaned; diagnostic1 uses the same installedBox1/
OCI022 and sleeping init plus captured exec to reveal the first probe failure.

Diagnostic1 captured exact first error: sched_getparam returned ENOSYS38. The
installed Zig0.16 musl implementation unconditionally returns ENOSYS for the
three POSIX process scheduler wrappers (source copies/hashes retained). This
is a probe libc stub, not evidence the Guest kernel rejects scheduling. C2
uses the actual Linux sched_getparam/getscheduler/setscheduler syscalls with the
same arguments/errno/readback/assertions. No product or security change; C1
source/ELF/failure retained. Frozen Rust/source/client/Box1 remain unchanged.

Native2's readiness command `/bin/test` returned exec ENOENT: this pinned image
provides the shell `test` builtin, not that executable. The fixture will call
`/bin/sh -c 'test -s /scratch/init.json'`; readiness still requires actual success
before reading the file. No product PATH fallback is introduced. Diagnostic2
independently captures C2 against installed1 before the updated fixture matrix.
