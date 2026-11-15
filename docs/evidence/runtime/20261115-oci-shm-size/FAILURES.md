# Spec030 retained failures

## Admission red1

Actual2passed/2failed/0ignored,14.21s wrapper. Valid policy.shm_size fails the
existing qualification guard; size-only /dev/shm tmpfs fails the reserved system
mount guard. Invalid sizes/conflicts/other profiles remain refused. The existing
VmLocalExecutionBackend already builds /dev/shm:size=N; preflight now validates
that same generated intent. Only the exact size-only standard target is admitted,
with Guest default1777/rw/nosuid/nodev/noexec fixed. Bare /dev/shm refusal stays.
No native result is claimed yet. Prior029 assets/evidence remain preserved.
