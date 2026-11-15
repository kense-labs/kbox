# Spec015: Box tmpfs intent on explicit macOS OCI execution

Status: **bounded spec015 accepted** on the existing macOS26.6.2(25G83), M5Max
arm64,euid501,Rust1.97.1 host. This closes tmpfs intent in the explicit HVF
profile only. Continuous local W4 work remains active; next is OCI root read_only.

Source inputs `31aeb63a32b8b388fe5526eddf9f3172eb3c577656fd8e1b6342309a17fbcb16` (1674 inputs), unchanged Cargo.lock
`0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`.
Box final1 `/Users/roylin/code/kense/kbox/artifacts/oci-tmpfs-06c672b2c50b40b5ae90b3703bae89f7/final-dist-tmpfs-1/kbox-3.4.3-macos-arm64.tar.gz` SHA256
`1f81ad37106ed31275027bd5c135a1b9f72bcfd755d5af252d58e75dac9e1f30`,
23,370,310 compressed bytes / 44,122,112 installed disk bytes. Eleven file
hashes verified. Guest Init `f2b6d3278dcad3033389148bd7335113cc14d0cd21cab48de0bc7e5e8ce2cece`.
Prior Box archive c6eb7199… plus checksum retained under run previous-dist.

OCI12 archive df4b882c… is unchanged and freshly extracted for this run, preserving
original dada2886… source provenance. Eleven hashes, Mach-O arm64 signatures,
static aarch64 Guest binaries, libraries and embedded Agent equality are checked
in [final-identities-1](checks/final-identities-1.json). It is not relabeled as a
new OCI build. Actual worker origins must match these installed prefixes.

Implementation uses the existing Core guest-exec module, Guest nix mounter and
OCI Guest MountPlan. No dependency, workspace, native source, wire, state store or
configuration system added. Shared bounded parser separates generic flags and
kernel data. BoxVm retains advanced data options. Mac admission supports ro/rw,
nosuid/suid, nodev/dev, noexec/exec and size (positive <=512 MiB, bytes or K/M/G),
octal mode <=07777 and numeric UID/GID below u32::MAX. At most128 disjoint mounts,
32 options/mount,8192 bytes/spec,4096-byte path. System /dev,/proc,/sys subtrees,
/run/kbox runtime state,workspace/caller/tmpfs overlap,conflicting flags,duplicate
data and unqualified options refuse. Missing size uses existing kernel default.
Image VOLUME at the same normalized target is overridden without allocation;
subtree conflict refuses before runtime-owned preparation. Existing file/storage
mount indexes and identity annotations remain intact. LinuxKVM/WHPX qualification
continues to refuse this Box tmpfs input.

Before: admission-before1 actual2failed; spec-before2 actual1failed. Zero-test
spec-before1 is explicitly not proof. Focused-after3 Core2/Runtime16 passed.
Core owning tests482+8integration and Guest host156 passed; LinuxClippy2 and
hostClippy1/fmt passed. Installed BoxVm final1 rootRO/inner tmpfs flags/noexec/PTY/
restart retained-data regression passed1 scenario,6.79s. Native calibration2
passed1/0/0,138.89s: realtmpfs,EROFS,ENOSPC,noexec EACCES,uid123/gid456/mode1770,
per-execution isolation,generations1→2→3,normal restart and authenticated stopped
Host recovery,empty new tmpfs and retained named/caller data. Its CLI flag is
false because it intentionally used prior014 CLI; final3 below requires and
observes true through the actual new installed CLI. All earlier failures and corrections retained in
[calibration failures](checks/calibration-failures.md).

Final installed tmpfs3 actual **1/0/0,163.07s**, CLI true: main execution
`b95603eb-000c-410c-b0ff-f25b12262c01`,three observed workers,generations1/2/3.
All effect markers and actual mountinfo readback checked for all three waves;
no extra anonymous allocation despite three image VOLUMEs. Numeric owner process
writes as123:456. Caller/named writes use their actual501:20 owner with cap_dropALL.

Final installed regressions: file Host PEP **1/0/0,42.07s**; named lifecycle/CLI/
three HostRO waves/normal+stopped recovery **1/0/0,162.01s**; anonymous Guest
ownership/seed/independent allocations/caller+named overrides/auto-remove/CLI/
normal+stopped recovery/fileHostRO repeated **1/0/0,421.59s**. All original
assertions and the600s runner deadline retained. A–F five scenarios passed,
175.67s total; prewarm/directory actual1/0/0 each, OCI/HostROrestart/primary
each one completed native scenario. All actual result files and complete native
worker executable/library origins checked, with no development-prefix borrowing.
Installed BoxVm rootRO/init/exec/PTY/inner tmpfs safety flags/noexec/restart passed
one scenario,6.79s. Linux init tmpfs tests actually ran inside that installed
BoxVm: **2/0/0,1.33s**, immutable Guest test2067a9ef…; cross-build17.12s is
recorded separately, never counted as native execution.

`verify-commit1` **exit0,261.49s**: Box2228passed/28ignored,CLI933passed,
installer8passed plus workspace/all-targets,fmt,
Clippy,shared contracts/support matrix. Core482+8integration and Guesthost156
passed separately. Linux Clippy2 exit0. Ignored defaults and zero-test targets
are not native passes. Native parents above each explicitly executed one test.

Final audit: **13 isolated homes,106 observed socket paths absent;0 managed
executions,volumes,owned VM/shim/engine/Host/Agent processes,mounts,workload disks,
private builders or open descriptor holders**. Original seven RAM devices and
unrelated mounted image preserved. Native15 and AF111 baseline immutable blob
files unchanged. Audit deleted nothing; fixtures removed only their own exact
managed resources and explicit named allocations. Caller canaries and retained
named data were checked before fixture cleanup. [Cleanup](checks/final-cleanup.json),
[descriptors](checks/final-descriptors.json),[retained stdout summary](checks/final-receipt-review1-summary.json).

OCI archive33,345,582 compressed/114,847,744 installed bytes and Box sizes above
were measured with actual stat/du; see [size commands](checks/artifact-sizes.json).
Native OCI selected archive d7a398ec…/library7dcaf940…/firmware841bc9d5…/
kernelb1180b50… remains spec009-qualified; Box retains its independent native
companions. Parent gitlinks3c2889c7…/fc3ea068… clean and index empty.
Both build identities and per-file digests are in final-identities-1.json.

Commands (full argv,env,exit,counts and logs in checks receipts):

```sh
cargo test --locked -p kbox-core -p kbox-runtime tmpfs -- --nocapture
cargo test --locked -p kbox-core -p kbox-guest-init --all-targets
cargo clippy --locked -p kbox-core -p kbox-runtime -p kbox-guest-init --all-targets --no-deps
# Exact existing Zig CC/CXX/AR environment is in guest-linux-clippy2.json
cargo clippy --locked --profile release-dist --target aarch64-unknown-linux-musl -p kbox-guest-init --all-targets --no-deps
# K_HOME and K_BUILD_LIBKRUN=1 recorded, no user installation input
bash scripts/package-macos-arm64.sh
bash scripts/verify-commit.sh
cargo zigbuild --locked --profile release-dist --target aarch64-unknown-linux-musl -p kbox-guest-init --tests --message-format=json
```

Run-local scripts and hashes are retained under
`artifacts/oci-tmpfs-06c672b2c50b40b5ae90b3703bae89f7`: build-box-final1.py,
check-final-identities1.py,run-native.py (tmpfs3/files1/named1/anonymous1),
run-rootro.py,run-linux-tests.py,qualification-runner.py,final-resource-audit.py
and final-receipt-review1.py. Source frozen1 and old archives/sidecars retained.
Source was unchanged through final receipt review. The two short preflight
compile failures, initial DAC fixture failure and corrected Linux log variables
remain visible; no assertion/failed record removed.

## Explicitly not claimed

No full W4/Host Service/default cutover,public readiness,GA,fresh-host or other
platform promotion. macOS default remains BoxVm; persisted explicit OciSdk and
stopped recovery only. Windows ARM64 dependencies unsupported and no native run.
No independent setuid/device-open probe: nosuid/nodev have kernel mountinfo
readback; no-new-privileges/cap_dropALL remain active. RO/noexec are Guest mount
controls, not revocation of explicitly granted Guest SYS_ADMIN or interpreters.
No new Secret/custom mount families,tmpcopyup,advanced Mac tmpfs options or
unqualified overlapping targets. No runtime RSS measurement;512 MiB is VM config.
Known two missing historical OUTCOME files remain missing. No commit,push,release,
real ~/.kbox or unrelated resource cleanup.

Evidence indexing correction during016: the run-check wrapper had overwritten
the machine-readable review summary with its command receipt at the same name.
Original stdout still contains the printed PASS/count/source summary; that exact
object is recovered in final-receipt-review1-summary.json with provenance and
explicit missing script-hash field. No historical test/source assertion rerun
and no missing data fabricated. Original command receipt/logs retained.
