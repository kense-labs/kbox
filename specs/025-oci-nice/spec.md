# Bounded nice-priority limits

Status: accepted on this existing macOS arm64 host. [Final installed evidence](../../docs/evidence/runtime/20261115-oci-nice/OUTCOME.md).

Carry existing Box nice intent through OCI RLIMIT_NICE and the existing Guest
executor on explicit macOS OciSdk. Preserve zero/infinity/u64, exact soft/hard,
malformed/duplicate/inverted/overflow refusal, defaults and other profiles.
This limits future priority improvements per process; it does not set initial
nice, reserve CPU or create a per-UID budget. Keep public/CLI user defaults and
the existing NPROC/SIGPENDING/MSGQUEUE identity boundary unchanged. Current Mac
profile cannot grant CAP_SYS_NICE/CAP_SYS_RESOURCE or privileged execution.

With nice25, actually move from positive nice to-5, refuse-6 with EACCES and
retain priority after refusal. Prove inherited child behavior, hard-raise EPERM,
entered user namespace priority refusal (root mapping itself requires refused
CAP_SETFCAP), successfully mapped nonroot namespace, and root-with-empty-
capability execution, independent infinity(-20) and zero(no improvements after
lowering priority) controls. Cover init/exec/child/stream/realPTY/CLI/health/
attach/replay/fencing/normal restart/stopped recovery from installed archives.

No config/state/transport/schema/native source/dependency/workspace change.
Reuse Core parser, Box rlimit helper and Agent setrlimit/getrlimit. Frozen assets,
actual effects/counts, affected/A–F regressions, verify and exact owned cleanup
precede acceptance. Full W4/default cutover/live continuity/platform gates remain
open; Windows ARM64 dependencies unsupported/no native run; RSS unmeasured.
Continue remaining local W4 after this bounded slice.
