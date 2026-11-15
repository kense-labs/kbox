# Nonroot per-UID process-count limits

Status: accepted; final installed assets and owned cleanup qualified in
[OUTCOME](../../docs/evidence/runtime/20261115-oci-nproc/OUTCOME.md).

Admit Box nproc intent through the existing OCI RLIMIT_NPROC executor for an
explicit numeric nonzero UID with no capabilities and no_new_privileges=true.
Require the same configured UID at every init/exec/stream/PTY entry; root,
different UID, implicit-root requests and privileged combinations must refuse
before workload entry, with static configuration refusals before allocation.
Linux root's exemption must not be presented as enforced process quota.
This is a real-UID process/thread budget in one dedicated Guest, not a second
execution PID counter; existing cgroup pids control remains independent.

Keep public Exec/Pty user=None ->root0 semantics unchanged. CLI callers for a
qualified persisted OciSdk nproc record explicitly select its configured UID
when omitted, including health/attach; explicit overrides stay explicit and
the manager validates them. Other CLI routes/requests retain their defaults.
Use authoritative managed metadata, not a stale flattened user mirror.

Require actual fork EAGAIN, numeric UID/capability/no-new-privileges readback,
hard-raise EPERM, owned child reaping and budget reuse, separate unlimited
control and instance isolation, all five process paths, root/different-UID
refusal without entry, CLI exec/PTY/attach/health, replay/fencing/normal restart/
stopped recovery. Keep source-file identity rules and use separate probe inodes.
Own checks, final installed archives, actual counts, affected regressions/A–F,
owned cleanup and retained failures precede acceptance.

No new configuration language/dependency/workspace/resource state/native source changes.
Default route/support levels/live continuity/full W4 remain unchanged. Continue
the next local item after this bounded slice; no Windows port or waiting.

Native1 exposed a real kernel setuid equality bypass. Shared Agent finite NPROC
now prepares one temporary fork slot before credentials, forks/reaps only trusted
code after UID/capability application, then restores and verifies exact soft/hard
limits before workload entry. Only raise the temporary hard ceiling if soft+1
needs it; infinity saturates. Failure to allocate this bounded admission child
refuses launch. Zero budget refuses and a one-process budget retains its last
slot. Workload PID/supervision and cgroup PID policy remain unchanged.
Because old Agent code lacks this effect, Box requires Host nproc-admission v1
before preparation. NPROC utility-VM manifests use explicit v10 with a bound
v1-v9 transport schema, preserving every prior transport's validation/bytes and
blocking downgrade. Both HVF and KVM manifest producers wrap complete existing
transports; no operation protocol or resource store is added. Current/old Host,
shim and Agent installed assets must prove refusal and cleanup without Start.
