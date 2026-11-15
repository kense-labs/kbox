# Bounded POSIX message-queue limits

Status: accepted on this existing Mac arm64 host;
[final installed evidence](../../docs/evidence/runtime/20261115-oci-msgqueue/OUTCOME.md).
Full W4/default cutover/platform gates remain open.

Admit existing nonroot Box msgqueue intent through OCI RLIMIT_MSGQUEUE and the
existing Guest executor. Reuse the common real-UID/capability/NNP boundary and
CLI authoritative default UID across init/exec/child/stream/PTY/health/attach.
Root/implicit/different UID and privileges refuse before entry; malformed,
duplicate/inverted/overflow limits and unsupported profiles refuse before
allocation. Keep zero/infinity/u64 and unmentioned default bytes unchanged.

Qualify actual mq_open byte-budget EMFILE separately from FD exhaustion and
per-queue message-capacity EAGAIN. Prove payload/priority/FIFO order, real-UID
sharing across processes, unlink-while-open lifetime, close+unlink quota reuse,
namespace bypass refusal, independent infinity and zero controls. Cover all
process paths, CLI/health/realTTY/attach, replay/fencing, normal restart and
stopped Host recovery with installed archives and owned cleanup.

This is standard POSIX message-queue accounting including kernel overhead, not
aggregate memory/RSS, SysV IPC, a disk quota or a ban on all IPC. No broker,
duplicate counter/state/config/dependency/workspace/schema/native source change.
Keep nproc/sigpending and prior A–F behavior and persisted routing intact.
Full W4/default cutover/live continuity/platform gates remain open; Windows
ARM64 current dependencies unsupported/no native run. Continue local W4 after
this bounded slice closes.
