# Bounded real-time process limits

Status: accepted on this existing macOS arm64 host. [Final installed evidence](../../docs/evidence/runtime/20261115-oci-realtime/OUTCOME.md).

Carry existing Box rtprio/rttime intent through standard OCI RLIMIT_RTPRIO and
RLIMIT_RTTIME into the existing Guest executor. These coupled controls admit
bounded FIFO/RR priority and limit continuously running real-time CPU time in
microseconds. They do not reserve CPU or bound normal scheduling/wall time.
Keep exact soft/hard, zero/infinity, default bytes, root/CLI identity semantics
and existing per-UID counter restrictions; refuse malformed/overflow/duplicate,
privileged/SYS_NICE/SYS_RESOURCE and unsupported platform combinations.

Actual installed native acceptance must prove allowed FIFO/RR priority and
EPERM beyond the ceiling, inheritance/reuse and no namespace capability bypass;
actual SIGXCPU/SIGKILL under continuous real-time CPU load, measured child CPU,
blocking-reset and non-real-time/unlimited/zero controls. Bound every child and
loop and reap exactly owned children. Cover init/capture/child/stream/realPTY,
explicit nonroot, CLI/health/attach, replay, identity/generation, normal restart
and stopped Host recovery. Actual result counts and resource cleanup are gates.

No native source/config/state/schema/dependency/workspace change is planned.
Full W4/default Host Service cutover/live continuity/platform release gates stay
open. Windows ARM64 dependencies unsupported/no native run; RSS unmeasured.
After acceptance immediately continue the next existing local W4 gap.
