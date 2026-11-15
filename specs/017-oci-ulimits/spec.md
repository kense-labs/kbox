# Bounded Box ulimits on explicit Mac OCI execution

Status: bounded spec017 accepted with final installed Box2 and unchanged OCI016
assets, actual native results and owned cleanup. Evidence:
[OUTCOME](../../docs/evidence/runtime/20261115-oci-ulimits/OUTCOME.md).
The starting Mac profile rejected every nonempty ulimit; the starting legacy
Shim silently ignored malformed input. Both gaps are closed within scope.

Scope: reuse/move that parser through existing Core types,keep all recognized
legacy resource names and valid SOFT:HARD values,explicitly refuse malformed
input. Mac bounded subset nofile/fsize/core (others remain unqualified); apply
static stored intent to OCI init,captured exec,stream,PTY and children. No new
config,deps,workspace,mounter,state or native edits. Keep pids cgroup authority
and stored config/identity/generation,existing no-override process defaults and fingerprints and all
accepted rootRO/tmpfs/storage/HostRO semantics. No dynamic rlimit update API.

Require actual targeted before tests; malformed/duplicates/overflow/soft>hard/
unsupported combinations refusal beforeprepare; actual init/exec/stream/TTY
readback plus nofile exhaustion and file-size denial with writable controls,
children,restart/stopped recovery,exactroute/generation/CLI,replay/conflict and
owned cleanup. Rebuild affected final archive,isolatedinstall,verify-commit and
impacted native regressions/counts/evidence. OCI assets may be reused only if
unchanged with their original3ac0fbd8… provenance. Other platform qualification
remains unchanged,Windows ARM64 unsupported without nativevalidation,fullW4/
defaultcutover/GA stayopen. Continue local W4 after bounded closure.
