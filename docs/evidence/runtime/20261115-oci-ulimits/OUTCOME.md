# Spec017: bounded process ulimits and managed CLI sessions

Status: **bounded spec017 accepted** on the existing macOS 26.6.2 (25G83),
M5 Max arm64, euid501, Rust1.97.1 host. Continuous local W4 work remains active.
No platform, readiness, default-route or release promotion.

Mac Box ResourceLimits.ulimits now reaches the existing OCI process.rlimits
executor for init, captured exec, streams, real PTY and children. The admitted
subset is nofile (16..1048576), fsize and core. A shared Core parser preserves
all 15 recognized legacy Shim names and Linux IDs, validates bounded numeric
SOFT:HARD/-1-infinity input, rejects duplicates/unknowns/overflow/soft>hard, and
replaces the old Shim warning-and-omit path. Other Mac resources remain refused.
No override preserves exact process bytes, including the OCI builder's actual
NOFILE1024:1024 default and explicit None. Requested types replace only their
matching defaults. Public SDK complete-process fingerprints bind rlimits;
legacy operation IDs are unchanged. No new executor, state, config, dependency,
workspace or native source edit.

Native regression found a separate CLI security bypass: managed BoxVm exec,
PTY, attach PTY and health probes used raw Guest sockets, bypassing the existing
ExecutionManager's security inheritance and fencing. These managed callers now
use the same manager as OciSdk, retaining each stored route and generation.
No second policy implementation was added. Captured caller K_SEC_* spoofing
cannot override stored cap_drop ALL/no_new_privs/seccomp. Legacy log routing and
unmanaged compatibility remain unchanged. [Retained failures](FAILURES.md)
include actual full capability masks and successful hard-limit escalation on
candidate1, the red CLI test, and the exact correction.

## Final installed artifacts

Frozen source2 is **8c687c06d486c721afcc33605d83ec208cd9b92f3a72d7732529ebd2d7ddd736**,
1682 inventoried inputs. Cargo.lock is unchanged at
0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb.

| Artifact | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box final2 | c61f0a5a90567ee9e0a2a2cde627b6b990b647821bb35dd79dc087c6c592d0c5 | 23,374,170 | 44,138,496 |
| Unchanged OCI016 archive, fresh isolated installation | c7db5a96c1acb2621872154a4f946b8c565fe26ece8222453319df22d63e67cc | 33,355,655 | 114,860,032 |

Box root: `/Users/roylin/code/kense/kbox/artifacts/oci-ulimits-e5c6b94fd37f419bb3be11cecf9d3042`;
archive `final-dist-ulimits-2/kbox-3.4.3-macos-arm64.tar.gz`, actual prefix
`installed-ulimits-final-2`. OCI prefix is `oci-ulimits-final1-unpacked/koci-hvf-0.3.6-macos-arm64`.
The OCI production inputs are unchanged, with their original
3ac0fbd8f65db2518b9586038f1d13ef00bde5b9aefdae60c069ef29455b9c7a provenance;
no rebuild or new-source claim is made for those bytes. Candidate1 and prior
archives, checksum sidecars, source inventories and all failures remain.
[Install and build provenance](checks/install-ulimits-final2.json),
[actual sizes](checks/artifact-sizes-final2.json),
[all companion identities and architecture/signature commands](checks/final-identities-2.json).
Both installations verify all 11 SHA256SUMS entries. Host executables are arm64
and pass strict ad-hoc signature verification; Hypervisor companions retain
signatures/entitlements. Guest Init f2b6d327… and Agent3f3f16a7… are static
AArch64 ELF. Embedded Agent equals its standalone archive copy. Box retains its
independent lib9ab3eefe…/FWca7bba4e…; OCI retains lib7dcaf940…/FW841bc9d5….
Native dependency selection and both clean parent gitlinks are unchanged.

The C kernel probe source/binary SHA256s are recorded separately in
[probe identities](checks/probe-identities1.json): Zig static AArch64 cross-build
is preparation only. The final native test actually executes that exact binary.
The frozen test client SHA256 is
2514f1cb9c7166cd168ec2aca2b1f773e253abee54c36ad3b4922c0a081354b9;
it is separate from the shipped CLI. All VM loaders resolve only installed
archive companions; no development library or real user installation is used.

## Actual checks

| Check | Actual result |
| --- | --- |
| Runtime before / Shim before | 0/2/0 and 0/1/0; failures retained |
| Focused Core / Runtime / Shim after2 | 2 / 5 / 9 passed |
| CLI managed-route before / commands after | 0/1/0, then 691 passed |
| Owning Core / integration / Guest host / Shim | 484 / 8 / 156 / 31 passed |
| Owning Clippy1, CLI Clippy2 | exit0 |
| Final verify-commit2 | exit0, 222.33s; Box Runtime2236 passed/31 ignored, CLI934 passed, installer8; fmt/workspace/all-targets/scoped checks passed |
| Final installed OCI ulimits3 | actual1 passed/0 failed/0 ignored, 176.49s, CLI true |
| Final BoxVm ulimits/restart/bad input | one completed scenario,6.56s |
| Final BoxVm capability/spoof/real attach controls | two completed controls,2.09s and1.77s; second also reads actual init-process limits |
| Final BoxVm rootRO/tmpfs/realPTY/restart | one completed scenario,5.31s |
| Final installed A–F | five scenarios PASS; prewarm/directory each actual1/0/0; lifecycle/HostROrestart/process flow each one completed scenario |

[Final receipt/count/source/route review](checks/final-receipt-review3-summary.json)
checks actual result files and logs, not process exit alone. Full command argv,
stdout/stderr, failed receipts and native loader maps remain in checks and the
owned private root from [run context](run-context.json).

OCI main execution23ddab4d-3868-4019-a6cc-d58ee3b97e2d advances generations1→2→3.
Each wave verifies five actual kernel results (init/exec/child/stdin stream/PTY):
NOFILE64:128,FSIZE4096:4096,CORE0:0; hard-limit increase EPERM1; 61 open files
then EMFILE24; regular tmpfs file stops at4096 bytes with EFBIG27. True PTY and
stream stdin are checked. Reconstructed clients, normal restart, authenticated
Host-SIGKILL stopped recovery, one-effect Exec replay and changed-request refusal
pass. A separate explicit OCI default control remains NOFILE1024:1024. CLI
repeats limits across restart. Actual BoxVm main-process limits are read from
/proc of the identified sleep process; captured exec and real exec/attach PTYs
cannot raise hard128→129. All five kernel capability sets are zero,
NoNewPrivs=1,Seccomp=2 even with attempted caller environment overrides.

Commands include:

```sh
cargo test --locked -p kbox-core -p kbox-runtime -p kbox-shim ulimit -- --nocapture
cargo test --locked -p kbox-core -p kbox-shim -p kbox-guest-init --all-targets
cargo test --locked -p kbox-cli --lib commands:: -- --nocapture
cargo clippy --locked -p kbox-cli --all-targets --no-deps
bash scripts/package-macos-arm64.sh
bash scripts/verify-commit.sh
```

Exact focused filters, full invocations and actual results are retained in checks receipts.
Unchanged OCI016 file/named/anonymous detailed matrices retain their historical
acceptance; they are not relabelled as spec017 runs. Current ulimits, inherited
rootRO/tmpfs, probe RO grant, CLI, A–F and source/loader/cleanup are newly tested.

Final audit: **21 isolated homes,184 observed socket paths absent,zero managed
executions/volumes,owned VM/shim/Host/Agent processes,mounts,workload disks,
private builders or open descriptor holders**. Fifteen main and131 AF baseline
immutable blob files remain unchanged (both AF roots checked). All seven
pre-existing RAM devices and unrelated image remain untouched. Audit deleted
nothing. [Resource audit](checks/final-cleanup.json),
[descriptor audit](checks/final-descriptors.json). Source2 unchanged through
review; native submodule trees clean, index empty. The review's own argv-check
failure and correction are retained separately; no passing native test was rerun
or assertion weakened to repair that checker.

## Explicitly not claimed

Process rlimits do not make trusted management File/Filesystem APIs a workload
quota. CORE0 is actual readback; no crash-generated core experiment is claimed.
No unqualified Mac resource names, dynamic rlimit update, full Box parity,
Host physical disk RO, live-session continuity, full W4/Host Service/default
cutover, readiness/GA, fresh-host, cross-platform or support promotion. Secret,
pools,snapshots,TEE,devices/custom network remain open/refused as applicable.
Windows ARM64 dependencies remain unsupported without native validation.
Runtime RSS was not measured. No commit,push,publish,real ~/.kbox changes,native
submodule edits or unrelated cleanup. Two known missing historical OUTCOMEs stay
missing. Next local W4 task: bounded managed Secret transport/qualification.
