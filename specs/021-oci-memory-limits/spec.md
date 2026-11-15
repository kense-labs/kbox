# Box data, stack and locked-memory process limits

Status: bounded021 accepted; [final installed evidence](../../docs/evidence/runtime/20261115-oci-memory-limits/OUTCOME.md).
Bounded continuation of the existing W4 unqualified
process-limit gap, after accepted020. Existing Mac default and platform gates
remain unchanged.

Carry existing Box data/stack/memlock ulimits, in bytes, through standard OCI
process.rlimits and the existing Agent setrlimit/readback executor. Preserve the
shared parser's u64, zero, infinity, duplicate and soft<=hard contract, existing
CPU/nofile/fsize/core semantics, omission/default bytes and persisted route,
identity, replay and generation authority. These are process controls, separate
from cgroup/VM memory and RSS; they do not promise an execution aggregate.

Require native data allocation/mmap ENOMEM with positive controls, actual stack
SIGSEGV on an alternate signal stack with an unlimited recursion control,
mlock success within the limit/ENOMEM above it and successful unlock/reuse,
hard-raise EPERM, init/exec/child/stream/realPTY, independent executions, CLI,
replay, normal restart and stopped recovery. Test the actual kernel effects,
not only serialized fields. Resource limits may intentionally prevent startup;
zero must not silently become infinity or another value.

Reuse existing callers, parser and executor. No new config, ABI, schema,
dependency, workspace, resource state or native source edits. Other unqualified
types, including RSS, remain refused; refusal does not implement them. Freeze
and install final affected archives, verify actual test counts, run owning
checks/verify and affected native/A–F regressions, audit only owned resources
and retain every failure. Continue the next W4 local item after closure.
