# Queued-signal limits: accepted

Bounded spec023 is accepted on this existing Mac arm64 host. The final review
checks actual source/asset identities, nonzero test counts, native effects and
owned cleanup. [Final review](checks/final-review3.json).

Box carries its existing sigpending intent through standard OCI
RLIMIT_SIGPENDING and the existing Guest setrlimit/getrlimit executor. Only the
Box mapping/shared per-UID entry boundary and existing CLI typed resource match
change in production. No OCI/Agent/native source, transport/capability schema,
dependency, configuration, ABI or resource state is added. Numeric nonroot UID,
empty capability sets and NNP must remain identical at every process entry.
CLI capture/health/PTY/attach explicitly select that authoritative configured
UID; public None still means root0 and refuses for these per-UID limits.
Existing nproc admission, identity/route/replay/generation fencing are retained.

The contract is per-real-UID queued signals through sigqueue. It does not ban
signal delivery: ordinary kill can deliver a coalesced pending signal even at
queue exhaustion. Primary source diagnosis uses the pinned
[Linux6.12.76 signal allocator](https://github.com/gregkh/linux/blob/v6.12.76/kernel/signal.c)
and [namespace inheritance](https://github.com/gregkh/linux/blob/v6.12.76/kernel/user_namespace.c).
Actual sources/hashes remain in the artifact root. The native probe verifies
both the quota and this ordinary-kill positive control.

## Installed artifacts and provenance

Existing host: macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1.
Final source3 is
`b0d1c272758563f69ee10d20d6b62a59f29e6d7e5e15b1f9c7370eb97af18129`,1698 inputs.
All original uncommitted changes and failed candidates are retained.
[Actual environment](checks/host-environment.json), [paths/context](run-context.json).

| Installed archive | SHA-256 | Compressed bytes | Installed bytes |
| --- | --- | ---: | ---: |
| Box3 kbox-3.4.3-macos-arm64.tar.gz | a6f6b3712876891e4967fb1c5a050346fee69a478e165c0d194b221342af3888 | 23,392,573 | 44,171,264 |
| Unchanged OCI022, fresh isolated extraction | 37009193a985f12eddced69d1ee5214650ae2e7e1d449ba5787802195bb381e4 | 33,421,132 | 114,950,144 |

OCI retains its original source
`4f9dbe71f19a9d018e40b0a58e5a62e2dd2cde5d07c9698f7e710c9a8f2a648b`.
All OCI/Agent source inputs remain unchanged from accepted022; only seven Box
inputs change for this slice. Source1→2→3 changes only the ignored Sigpending
native fixture. Every packaged runtime payload is byte-identical across all
three candidates; build-info.json records each original source honestly.
All11 companion hashes per installation, actual arm64 Mach-O/static arm64 ELF,
strict ad-hoc signatures, native links and embedded/standalone Agent match.
Native observations bind workers/libraries to these installed prefixes.
[Source delta](checks/source-freeze1.json), [Box3 install](checks/install-sigpending-final3.json),
[OCI fresh install/original provenance](checks/reused-oci-install1.json),
[all file/signature/architecture/dependency identities](checks/final-identities-3.json),
[2→3 payload equivalence](checks/production-equivalence2-to3.json).
Parent native repositories remain clean at libkrun3c2889c7… /libkrunfwfc3ea068…;
Cargo.lock remains0f82d0ea… . No source submodule working-tree edits.

## Tests and native effects

Before1 admission/completion executes0passed/2failed/0ignored; CLI before1
executes0passed/1failed. After the shared mapping/default fix,14ulimit tests
pass/1ignored and the CLI identity test passes1. Zero/infinity/u64, exact
soft/hard mapping, unchanged defaults, root/unknown/different UID/capabilities/
NNP refusal, no mutation on refusal, malformed/duplicate/inverted/overflow
input and other-profile refusal are covered.
[Before](checks/sigpending-before1.json), [after](checks/sigpending-after1.json),
[CLI red](checks/cli-before1.json), [CLI green](checks/cli-after1.json).

The bounded static syscall probe actually queues realtime signals with distinct
ordered payloads, fills the sixteen-entry budget with EAGAIN11, drains/reuses
it and verifies hard-raise EPERM1. Parent and same-UID child share8+8 entries;
draining the child's8 lets the parent refill only those8. At full parent quota,
an entered/mapped user namespace still gets EAGAIN; every child is reaped.
Ordinary kill delivery/coalescing remains possible. Two parallel valid exec
probes at a held full queue enter but cannot allocate a fresh budget. Root,
implicit and other-UID capture/stream/PTY requests refuse before entry; an actual
guest shell proves the canary absent. A separate infinity execution queues and
drains32 entries; zero permits no sigqueue allocation while ordinary kill and
startup remain possible. Distinct caller-file inodes preserve the alias boundary.
[Probe source/ELF/driver hashes](checks/probe-identities3.json).

Main3 executes1passed/0failed/0ignored in224.71s(native),228.91s(wrapper). All
15 observations, zero/infinity controls, CLI exec/healthy health/realTTY/attach/
restart, replay/conflict/fencing and normal/stopped recovery pass. Earlier
main1/2 failures remain failures. [Actual main count](checks/box-native-sigpending-attempt-3.json).
Final matrix totals **10 outer Rust native tests +13 Guest unit tests +3 Python
native scenarios**, all passed; nested helper tests are not counted twice.

Installed2 affected regressions all pass: nproc1Rust+5Guest units; CPU1Rust;
data/stack/mlock1Rust; nofile/fsize/core1Rust; mixed Host Service/recovery/foreign
rootfs/capabilities/RO/replay/fault1Rust; single-file Host PEP1Rust; Secret ENV
1Rust+8Guest units with owned RAM detach; A–F2Rust+3Python scenarios. These are
actual2 source/archive receipts, retained with byte-identical3 payload proof;
none is relabelled as a3 invocation. Guest unit ELFs are reused unchanged with
their original hashes and actually execute on the current installed OCI VM.
[Nproc](checks/box-native-nproc-attempt-2.json), [CPU](checks/box-native-cpu-time-attempt-2.json),
[memory](checks/box-native-memory-limits-attempt-2.json), [old limits](checks/box-native-ulimits-attempt-2.json),
[Host Service](checks/box-native-host-service-attempt-2.json), [files](checks/box-native-files-attempt-2.json),
[Secret/actual Guest8/RAM cleanup](checks/environment-positive-2/result.json),
[A–F actual counts/results](checks/af-final1/native-results.json).

Final verify3 exits0 in198.75s, Runtime2253passed/39ignored, CLI935 and installer8;
workspace all-target check, fmt and scoped owning Clippy pass. Verify2 remains
a separate successful226.02s receipt. [Final verify command](checks/verify3.json),
[actual log](checks/verify3.log). Unchanged OCI/SDK/Agent owning
results remain historical [accepted022 evidence](../20261115-oci-nproc/OUTCOME.md),
not new runs in023. No unrelated unchanged check is repeated.

## Failures, commands and cleanup

Main1 fails only because absent FileDownload returns an actual ENOENT error,
not success=false. The final checker requires successful guest !-e && !-L,
retaining the original absence assertion. Main2 reaches all15observations,
replay/recovery and infinity control, then fails because a new zero control was
created with the intentionally reconstructed LegacyOnly client's new-create
policy. The final caller uses the existing explicit OCI control client and
asserts its persisted OciSdk route; production routing is unchanged.
No quota, permission or failure assertion is removed or weakened. All failed
logs/candidates, source freezes and cleanup-boxes[] remain.
[Full failure ledger](FAILURES.md).

Artifact root: artifacts/oci-sigpending-46afe6c4c9c348f0912c1d19503acba5.
Commands, arguments, actual counts, stdout/stderr and loader/descriptor maps are
in checks/*.json and adjacent logs. Executed drivers include build-box-final3.py,
check-final-identities3.py, run-native.py, run-memory-native.py and
qualification-runner-final1.py; final-resource-audit.py/final-review3.py both
pass. Candidate1/2 archives/checksum receipts remain;
[prior archives and independent SHA verification](checks/prior-artifacts-retained.json).

Final audit covers14 persisted/prepared homes, including both failures and the
failed explicit BoxVm zero control, plus130 observed socket paths. Executions,
volumes, owned VM/shim/Host/engine processes, mounts, workload/runtime disks,
sockets and FD holders are zero. Secret RAM12 unmount/detach exits0; all7initial
RAM devices and the unrelated mounted image remain untouched. Audit deletes
nothing. All23 native and325 A–F baseline cache files retain content, owner and
mode; index changes only update last_used/owned paths and one exact owned A–F
build bound to its original result digest/current byte-identical CLI hash.
[Cleanup](checks/final-cleanup.json), [FDs](checks/final-fd-holders.json),
[runtime disks](checks/final-runtime-disks.json), [cache audit](checks/final-cache-audit.json).

## Explicitly not claimed

This is standard sigqueue accounting, not a prohibition on kill/kernel signals,
aggregate Host memory/RSS or a replacement for cgroup/PID controls. Root and
privileged combinations remain explicitly unqualified. Runtime RSS is
unmeasured. Complete OCI Host Service/default cutover, full W4/live continuity,
fresh-host/other-platform native matrices, conformance/security/long soak and
platform/readiness promotion remain open. macOS default stays BoxVm and HVF
owner death stopped-only. Windows ARM64 dependencies remain unsupported without
native validation. No VMM/firmware port, real ~/.kbox mutation, real Secret,
unrelated cleanup, commit/push/publication or fabricated historical evidence.

Continue directly to existing nonroot msgqueue intent through the same OCI
executor. This accepted bounded slice does not end the continuous task.
