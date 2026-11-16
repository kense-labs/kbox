# Outcome036: pool template artifact ownership

Bounded036 implementation, final source-bound installed native acceptance and
scoped cleanup are complete. This is not whole-release/GA Pass.

## Actual defect and implementation

An isolated child process reproduced an empty Unbuilt pool deleting a pre-existing
caller `pool/tpl-<image hash>` directory (unowned-red1, parent0/1 and child0/1).
The builder also reused that hash directory across independent pools. Cleanup
now operates only on a remembered descriptor-pinned private allocation, never a
directory derived from an image hash or TemplateState. Each pool allocates its
own0700 directory with the existing tempfile/capability adapters. No dependency,
configuration, execution state machine, route or generation change.

Actual template resources retain a source VmManager after failed boot/finalization.
VMs carry consumer references until a successful backend finalizer or a proven
prelaunch failure; losing a manager without close poisons deletion conservatively.
Cleanup and rewriting refuse active/unproven consumers, replaced namespaces,
linked/symlink snapshot inputs and workload mounts overlapping private inputs.
Cold fallback retains the same private-input checks; security ConfigError does
not become a cached unavailable verdict. Refusal preserves resources and reports
failure. Original owned-artifact deletion assertions remain with explicit owned
fixtures; provider-free fixture failures now fail instead of silently returning.

The shared directory/use types live with VM lifetime, without a VM-to-pool module
dependency. A related pre-existing pool-only prewarm field is now correctly
feature-gated; vm-only-red1 records both missing references and vm-only-green1
passes `--no-default-features --features vm` after the correction.

Directory removal is descriptor-confined, but the adapter's final directory unlink
is not atomic against a hostile concurrent parent rename. The allocation namespace
is private and excluded from workload grants; observed replacement refuses.
Arbitrary same-UID host namespace tampering is not qualified by these tests.

## Failures retained and current checks

directory-primitives1 found tempfile's default0755, fixed by explicit0700;
directory-primitives2 passed3. wiring-check1 found four outdated fixture calls;
the callers/resources were completed without removing deletion assertions.
Candidate1 native-fault1 actually failed0/1 despite an internal resultPASS: real
VM stop and first input removal completed, but a repeated cleanup returnedENOENT.
The result file alone is not acceptance. Raw failure, package and cleanup remain.

An actual Host experiment proves APFS fstat(open unlinked directory).st_nlink
remains2; removed-retry-red1 reproduces0/1. Successful unlink completion is now
recorded inside the blocking worker rather than inferred from nlink. Retry is
idempotent and preserves a same-path replacement, including after its waiter is
dropped. Those checks pass. A preliminary AF harness refused its nested context
path before any VM; only repository discovery was corrected, confinement/count/
loader assertions retained (af-context-path-failure1.json).

Final source checks actually executed:

- Ownership/template filter19/0; backend-close tests4/0/1 ignored; vm-only compile
  and Runtime/CLI scoped Clippy pass.
- verify2: isolation4 pass, workspace all-target compile/fmt/support pass,
  Runtime2315pass/6fail/51ignored, exit101. Same2 RAM/Secret and4 APFS/provider
  disk-service failures remain; this is not a green default gate.
- Final CLI935/0; contract118/0 and installer8/0 also executed this phase before
  the final Runtime-only corrections. Contract/installer source unchanged.

Actual commands/isolated environment/exit codes and raw output are under checks/.
Every Cargo test uses a new task-owned K_HOME. verify2 independently allocates
`/private/tmp/kbox-tpl-own-d2a56efc8988/t/kbox-verify.F8vHbN/home` and short
`/private/tmp/kv.28WFy8`. Earlier default-home claims remain corrected by035;
no real user-home scan, cleanup or repair was attempted here.

## Source, packages and native work

Final source input digest
`cc8175d2d34ecbdc36f293db062891c2942d71d9839bc321459cbfa4870dffea`
covers1723 compiled/package inputs, based on074321a6 plus the recorded changes.
The frozen test client is f228d10b57a4152f232f8737505cb13db9f89e01582dddbc326e8d89dbf0bb65.
Box archive:
`artifacts/pool-template-ownership-d2a56efc8988451dbfcc53fdfbfc8594/final-dist-pooltpl-2/kbox-3.4.3-macos-arm64.tar.gz`,
SHA256 `d5393321f1f4ba5344b6693c6d697a9c8ef3e0c35c2fd890484a0a548b67108f`,
23412340 compressed bytes,44191744 installed bytes. Fresh install is
`artifacts/pool-template-ownership-d2a56efc8988451dbfcc53fdfbfc8594/installed-pooltpl-final-2`.
All11 installed manifest files/source provenance, CLI/shim arm64 signatures,
Linux arm64 guest-init, libraries and embedded OCI Agent identities verified.
Signatures are ad-hoc, not notarization/publication. Runtime RSS was not measured.

OCI inputs did not change; the retained separate installed OCI archive is
d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79,
original source d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871.
Exact library hashes are in final-identities-2.json; parent native gitlinks remain
libkrun3c2889c7/libkrunfwfc3ea068. Existing OCI native patch provenance remains
separately qualified, with no native submodule edit in this parent repository.
Old Box4093b9d5/OCI archives and failed candidatefd27b0b4 remain retained.

Native-fault2 actually passes1/0/0 on this macOS arm64 host using installed
companions/loader identities. A one-shot synthetic finalizer refusal leaves the
same actual VM writing; its template guard/input remain, actual stop retry closes
the reference, deletes the owned input and proves no late writes/caller loss.
This is a cold-boot consumer lifetime check, not a Linux snapshot/restore test
or an actual OS failure injection.

The final installed AF matrix passes5 rows:3 Python scenarios (OCI lifecycle,
Host RO attacks/restart and engine process flow) and2 actual Rust native tests
(process prewarm and directory grants/fencing). Counts are read from actual Rust
stdout, not inferred from result files. Together with fault2:3 Rust native +3
Python scenarios. Host RO records actual14 denials on each first/restart probe,
guest remount/fresh writable mount and child attempts, with RW control intact.

A separate actual CLI cold-pool matrix passes4 daemons,16 initial authenticated
Ready VMs and4 Guest commands;16 loaded shim/library identities are captured from
the installed prefix. Each home contains a pre-existing image-hash template
canary: all4 retain exact dev/inode and bytes after draining. One daemon requests
snapshot-fork on this unsupported Mac backend and takes the existing explicit
cold-boot optimization path without touching caller template inputs. This is not
snapshot functionality or a backend/isolation downgrade.

Final scoped audit covers41 private homes, both verifier short temporary roots,
35 observed VM socket directories,146 exact socket paths,2 engines and699
pre-existing immutable image-cache files. All owned managed executions/volumes,
VM/shim/engine/Host process checks, mounts/disk images, FD holders and sockets are
empty. Caller sources/canaries and prior archives remain. Only the mutable image
catalog changes; before/after hashes are retained. Audit deletes no resource or
data. Native case JSON, failed candidate1 JSON and raw logs are retained under
checks/native-case, checks/native-case-failed1, checks/af-final2 and
checks/pool-wave-final2; final-cleanup.json records exact observations.

Reproduction commands (absolute private prefixes/environment in context.json
and each receipt):

```sh
cargo test --locked --offline -p kbox-runtime --lib template
cargo test --locked --offline -p kbox-runtime --lib vm::tests::boot_stop
cargo check --locked --offline -p kbox-runtime --no-default-features --features vm
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli --all-targets --no-deps
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

The owned scripts under the recorded artifact root freeze/install/verify the
archive, execute native_stop2.py, qualification-runner4.py, pool_stress.py and
final-audit.py. All Cargo commands above were run through the private-home
run-check.py; native children use explicitly owned homes and installed companions.
Package command sets K_BUILD_LIBKRUN=1; it does not edit native source. All raw
failed commands and the old archives remain available.

## Explicitly not claimed

Linux KVM snapshot-native functionality, Windows native/ARM64 support, whole GA,
full W4/default OCI Host Service cutover, live-session continuity, fresh-host
conformance/security/soak, historical user-home nonmodification and RSS remain
unproved. Windows ARM64 current dependencies are unsupported, with no native run;
platform support levels/default libkrun route stay unchanged. Outer pool/legacy
CLI paths can still discard retained managers on errors; this stage protects
template input deletion but does not close that broader owner-lifetime defect.
The original15s startup cause, six disk-service failures/prior SDK failure and
historical unexplained RAM disappearance remain open.

Git prerequisite recheck still reports missing UID501 account and GitHub DNS
error8 (git-prerequisites2.json). Existing baseline branch is already an ancestor
of main; current remote equality/push are unproved. No force push or publication.
After final acceptance/audit/local commit, continue actual outer owner retention
and the remaining authorized local work; no whole-goal completion is claimed.
