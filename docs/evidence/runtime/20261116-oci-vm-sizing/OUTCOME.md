# Outcome031: explicit macOS HVF VM CPU/memory

Bounded implementation, final installed-archive native acceptance and owned
cleanup are complete on this existing macOS arm64 host. Full default verification
is **FAIL**, with six retained native disk-service cases; the SDK suite also has
one retained DiskManagement failure. This phase is not enterprise GA or W4 Pass.
The latest user authorizes commit/merge/push. Source commit `7327411e51d40b48ee1ed600d5495672dc2b2b16` is on local
main; actual remote synchronization remains blocked.

## Implemented contract

Box now admits the existing shared VM profile on the explicit Mac OciSdk route:
1/2/4/8 vCPUs and128..8192MiB in128MiB steps. It validates before resource planning,
requires immutable record/request CPU-memory equality, and emits only the exact
Box-owned annotation. Pre-existing input annotations cannot override that intent.
Other platform qualification profiles and the default BoxVm/libkrun route remain
unchanged. Cgroup Update changes workload limits, not immutable boot resources.

The existing validated VmConfig moved to public koci-sdk and remains re-exported
at koci_kbox::VmConfig, preserving the Windows x64 one-CPU constructor gate.
Required `dev.kbox.vm-resources` version1 binds the bounded closed JSON descriptor
and exact configuration digest through the existing attachment mechanism. Missing
versions/capability, unknown/duplicate fields, integer truncation, invalid profile
values, optionalized or substituted manifest references refuse explicitly.
Absence retains legacy1/512; an explicit descriptor always requires support.

The immutable typed value now reaches Host session options, paired shim flags,
parent handoff, private worker and actual native set_vm_config. Final shutdown
report validation compares against the admitted value; it still refuses resource
drift. No report/Agent ABI, new configuration system, dependency, native gitlink
or resource state machine was added. Reports are diagnostic; Guest hardware
readback establishes the native hardware result.

## Actual checks

Receipts retain exact commands, stdout/stderr, process exit and actual test counts.

| Check | Actual result |
| --- | --- |
| SDK contract red / green | red5:3pass/2fail; green6/0/0 |
| Box admission red / green | red5:2pass/3fail; green5/0/0 |
| Annotation projection |2/0/0 |
| Shim paired argument tests |3/0/0; first fixture failure retained |
| Host exact argv tests |2/0/0 |
| Strict report parser after native defect |19/0/0 including exact min/custom/max and drift refusal |
| koci-kbox library / binary / CLI integration |80/0/4;21/0/3;3/0/0 |
| Final koci-runtime all targets |444pass/0fail/2ignored,368.87s |
| Full SDK |235pass/1fail/0ignored; ordinary disk backing check needs DiskManagement |
| Final verify-commit.sh |exit101; workspace/all-target compile and fmt pass; Runtime2278pass/6fail/48ignored |
| Default follow-through |CLI935/0/0; contract library24/0/0; installer8 scenarios pass |
| Final six-package Clippy |pass, no lint suppression |
| Layout, wire, unsafe-doc, KCL and shell checks |pass; layout rerun with installed Python3.12 after Python3.9 lacked tomllib |
| Windows x64 and ARM64 pure SDK all-target check |both compile; no runtime claim |
| Windows broader SDK/VM/Runtime all-target check |both blocked in lzma-sys C compilation: MSVC stdlib.h unavailable |

Final gate command: `bash scripts/verify-commit.sh` (checks/verify-final2.json).
The six actual failed tests are retained verbatim in that log: two Secret/RAM
cases and four APFS/guest-ext4 provider cases. No assertion, ignored list, default
route or pass threshold was weakened. Prior immutable binaries already reproduce
these disk-service failures; no system service repair or repeated network retry
was attempted here. The script stops after Runtime failure; later checks listed
above were invoked independently rather than relabeling the script green.

## Final archives and dependencies

Both archives are rebuilt from frozen source
`d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871` (1717 compiled inputs), dirty build
base `e01bd356861c678b81ac7061ad3fbe4320e2c187`. Source maps are retained at
checks/source-frozen-2.json; changing test-only source is also included in that
map. Lock SHA256 `3cec984fb77851fcf5e6bbd3838cad56b71c2249cb29e5ec01ec19c36918adf8`.

| Artifact | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box3.4.3 | `aa924efec61b7cce329061d8fe356d83ddc05cadb520d6ff18c8c1d4b9001da6` |23404407|44175360|
| OCI HVF0.3.6 | `d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79` |33429314|114962432|

Box archive: `/Users/roylin/code/kense/kbox/artifacts/oci-vm-sizing-bcb75b402f5943f7af5ccfe32a91e081/final-dist-sizing-2/kbox-3.4.3-macos-arm64.tar.gz`. Isolated prefix: `/Users/roylin/code/kense/kbox/artifacts/oci-vm-sizing-bcb75b402f5943f7af5ccfe32a91e081/installed-sizing-final-2`.
OCI archive: `/Users/roylin/code/kense/kbox/artifacts/oci-vm-sizing-bcb75b402f5943f7af5ccfe32a91e081/oci-merged-final3-dist/koci-hvf-0.3.6-macos-arm64.tar.gz`. Isolated prefix: `/Users/roylin/code/kense/kbox/artifacts/oci-vm-sizing-bcb75b402f5943f7af5ccfe32a91e081/oci-merged-final3-unpacked/koci-hvf-0.3.6-macos-arm64`.

Each archive verifies11 companion entries. CLI/shim/guest-init/Agent/native libs,
architectures, strict signatures, dynamic dependencies, system-image identity,
embedded-vs-standalone Agent bytes and source/lock binding were checked
(checks/final-identities-2.json; no nonzero identity command). Native executions
use these actual installed assets and captured loader paths/hashes; the frozen
Rust client is a control harness, not a borrowed VM runtime. Final native client
SHA256 `0de73bda9497c695c3b7539845f85e116f402111852848098c16d6bed5615e24`.

OCI native archive SHA256 `d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc` is the exact existing
separately qualified uncached native bundle, with retained owning-repository
worktree provenance. It is not claimed to be recreated solely from a clean
parent gitlink. Parent gitlinks stay libkrun3c2889c76460d81e3fb16d6144580fd31eb1bae3 /
libkrunfwfc3ea06895301f1df6a274dad75fbd9317b3bfee. Native submodule sources were
not edited here. R35 old archives and all failed candidate archives remain
unchanged. No archive was published. Runtime RSS was **not measured**; configured
MiB, Guest physical map and installation disk use are separate quantities.

## Final actual native acceptance

Four current-asset Rust tests each execute exactly1/0/0, with result and cleanup
files: sizing matrix attempt2, old Host attempt2, old Shim attempt4 and new Host /
old Shim legacy-default attempt1. The earlier1/512 baseline is separate prior-
installed-asset research evidence, not a current custom-sizing pass.

The static aarch64 Linux probe mounts a fresh readonly sysfs using explicit Guest
SYS_ADMIN, reads actual online/present CPU masks and physical memory reg cells
from the live ARM device tree, and unmounts its own probe mount. It does not use
configuration, MemTotal estimates or cgroup values as hardware evidence.
Six initial hardware pairs are (1,128),(2,512),(4,768),(8,1024),(1,8192),(8,128).
Twelve readbacks include two simultaneous independent VMs, unchanged2/768 after
cgroup Update, normal restart generation2, stopped recovery generation3, and
CLI-created4/1024. All CPU counts and physical bytes match exactly. Each probe
reports and asserts its fresh mount removed. Probe source and ELF identity are
retained in checks/vm-hardware-probe.c and checks/probe-identity2.json.

Existing native helpers additionally establish actual directory Host denials14,
file denials16 with two fresh mounts/child attack/live reads/writeback, exact
identity and wrong-binding refusal, stale-generation/no-late-write fencing,
committed response loss/replay without duplicate effects, retained mixed BoxVm /
OciSdk routes, and Host death followed by authenticated stopped recovery. Boot
hardware remains2/768 across those operations. Live session continuity is not
claimed.

Old Host lacks the required extension and refuses before resource planning.
Old Shim actually emits `unexpected argument '--vm-vcpus'`; no old-shim VM or
workload fallback occurs. Pending-Create cleanup may reconstruct an Agent-only
VM using the **new** shim, then exact-delete it without Start; this is explicitly
validated separately from the original refusal. Workload markers remain absent
and Start receipt delta is zero. The legacy-default positive control really
boots new Host + old Shim at1/512, with exact legacy argument omission.

A–F regression is newly executed against final installed Box companions:
OCI lifecycle, Host readonly attack+restart and primary process stream are3
Python native scenarios; prewarm and directory authorization are2 Rust native
tests, each actual1/0/0. All5 pass, with complete native loader maps and separate
result files (checks/af-final1/native-results.json). Historical passes were not
relabelled: three Box companion hashes changed, so exact-byte reuse was rejected.
Thus final qualification is **6 Rust native tests +3 Python native scenarios**;
maintenance/research candidates are separately counted below.

## Retained failures and cleanup

The first custom VM did boot with actual1/128, but Delete exposed a missed fixed
1/512 shutdown report validator. The old-Shim refusal candidate hit the same bug
while draining its pending Create. Both processes aborted during subsequent
cleanup assertions; raw logs, states, hardware result and archives remain.
The shared parser now retains exact admitted VmConfig through finish;19 focused
checks and the complete444 Runtime suite pass, including wrong CPU/memory denial.
Both archives were rebuilt before final native replay.

Two exact failed homes were then reconciled through public SDK force-delete and
existing owner cleanup, with2 native Rust maintenance tests plus2 SDK cleanup
scenarios. Exact IDs/generation/config digest were checked; no Start was added,
no record was edited by hand, and no caller file was deleted.

Harness failures are also retained: Zig optimization initially defined NDEBUG
(the probe now forces assertions retained), a shim fixture omitted owner-pid,
OCI packaging omitted an inherited nativeArchive context field, a maintenance
reader used configDigest instead of the actual config_digest field, and two
old-Shim wrapper assertions initially confused new-shim cleanup entry with old-
shim entry / searched only removed service logs. Corrected checks retain the
real refusal text, exact installed paths, zero-Start proof and every failure.

The first cleanup audit incorrectly mined a global ps discovery snapshot for
phase socket ownership and found an unrelated pre-existing socket. It was not
removed or stopped. Raw discovery is retained locally by SHA in
checks/audit-scope-correction1.json; versioned evidence limits unrelated process
information. Final socket ownership comes only from actual phase runtime roots,
while independent process discovery still checks exact phase prefixes.

Final audit: **15 isolated homes**, zero managed executions/volumes,
VM/shim/engine/Host/Agent processes, current-phase mounts/disks/FD holders/sockets;
18 observed owned socket directories and
73 socket paths absent. Immutable cached inputs:
31 native +531 A–F files unchanged,
including ownership/mode; only mutable catalogs are separately recorded.
Caller files/directories remain, permitted writeback matches expected bytes and
exact file grants return to one link after teardown. The audit itself deletes
nothing. Final result: checks/final-cleanup.json and final-home-inventories.json.
The older unknown-actor seven-RAM preservation failure remains a failure.

## Git convergence and next action

Source commit `7327411e51d40b48ee1ed600d5495672dc2b2b16` is on local main with all prior work retained. The
worktree was clean immediately after that commit; final compiled input hashes
match both accepted installed archives. Actual local receipt:
checks/git-local-closeout1.json. The host hook reported missing lefthook; manual
verification was run and its failures were retained, without disabling the hook.
All prior work is retained on local main. `git merge-base --is-ancestor
codex/macos-oci-release-baseline HEAD` returns0, so the earlier baseline branch is
already integrated locally. The source/evidence diff is reviewed before staging;
Local closeout is recorded in that receipt; remote closeout remains unfinished.

Fresh UID501 lookup still fails and github.com DNS still returns error8
(checks/git-prerequisites1.json); the prior real push failed128 and HTTPS fetch
failed DNS. No remote equality, fresh merge or successful push is claimed, and
no credentials/network/DirectoryServices settings were changed. There is no
applicable GitHub connector fallback. Once those external prerequisites return:
`git fetch origin`, inspect new remote commits, integrate normally without force,
rerun checks/affected native acceptance if source changes, then `git push origin
main` and compare actual remote SHA. Do not start another feature before this
requested convergence step.

## Explicitly not claimed

Full OCI Host Service/default route switch, whole W4 parity, live-session
continuity, enterprise GA, fresh-host/platform support promotion, conformance,
long soak, resolution of the earlier parallel warm-boot failure, or resolution
of the prior unrelated RAM disappearance. Full default gate and RAM Secret
positive acceptance await DiskManagement recovery. Broader Windows compilation
awaits MSVC headers; Windows ARM64 current virtualization dependencies remain
unsupported and there is no real ARM64 Windows run. Pure SDK cross-compilation
proves compilation only. Linux OpenSSL/native verification remains externally
blocked as recorded previously. Runtime RSS is unmeasured. No user home data,
unrelated resources or native submodule source was modified; nothing published.
