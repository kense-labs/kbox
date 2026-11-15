# Spec016: OCI Guest root read_only intent

Status: **bounded spec016 accepted** with final installed archives,native
counts and owned cleanup. Continuous local W4 work remains active.
This is existing macOS26.6.2(25G83),M5Max arm64,euid501,Rust1.97.1 evidence.

Box config.read_only now reaches standard OCI root.readonly in Mac completion.
The existing OCI Agent InitPlan and fallible rootfs finalizer perform actual
remount+ST_RDONLY readback before release. Exec/stream/PTY and descriptor-confined
file/filesystem helpers reuse the container view. No second mounter/lifecycle/
config,dependency,workspace or native source edit. Other Box profiles remain
explicitly unqualified; default BoxVm/retained routes/generation authority stay.

Native calibration exposed rootfs-image v1's explicit writable-only contract.
Required rootfs-image **v2** therefore binds a read-only Guest view while the
physical private boot disk stays writable. v1 constructor/bytes/root restrictions
and old assertions remain intact. Public SDK automatic/explicit extension
versions match descriptor/root access; HVF advertises1+2; Box selects2 before
preparation for readonly intent. Agent manifest retains its existing versioned
rootfs descriptor field and exact bundle binding. Old Hosts/Agents cannot silently
reinterpret2 as1; old Host preflight refusal is demonstrated separately.
Existing v1 records retain routing,identity,version and access semantics.
Rollback of new v2 records to binaries that understand only1 is unsupported.

Final frozen2 inputs **3ac0fbd8f65db2518b9586038f1d13ef00bde5b9aefdae60c069ef29455b9c7a**,1676 files; Cargo.lock
0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb unchanged.

| Artifact | SHA256 | Compressed bytes | Installed disk bytes |
|---|---|---:|---:|
| Box final2 | 52418165c730223d4284daf4a7c5050a2f9475c128cd5715076460f6d6d57721 |23,370,870|44,122,112|
| OCI final2 |c7db5a96c1acb2621872154a4f946b8c565fe26ece8222453319df22d63e67cc|33,355,655|114,860,032|

Both rebuilt and installed from their archives in fresh owned prefixes.
[Build/source/file/signature/architecture/embedded-Agent identities](checks/final-identities-2.json),
[actual size commands](checks/artifact-sizes-final2.json). OCI Agent
`3f3f16a7e5b39e2c8f15befa04fc0f05233f80f01fb59a1139fcb5d0f361c8ad` equals its system-image copy; Box Guest Init
f2b6d3278dcad3033389148bd7335113cc14d0cd21cab48de0bc7e5e8ce2cece unchanged.
Native selected OCI archive d7a398ec…/lib7dcaf940…/FW841bc9d5…/kernelb1180b50…
unchanged from009; Box retains its separate companions. Parent gitlinks unchanged.
Run archives under `/Users/roylin/code/kense/kbox/artifacts/oci-rootro-cc006d2a334b4da188696678cf03f32a`: final-dist-rootro-oci-2 and oci-rootro-final2-dist.
Previous015archives and016archive1/candidates/source snapshots/sidecars retained.

Final installed Root5 actual **1/0/0,317.42s**,CLI true and explicit OCI writable
control route. Main execution bf0a18b3-0844-46d4-b730-7a30c88bc916,generations1/2/3.
At each wave: actual init root refusal; ten root mutation attacks through captured
exec,stdin stream and realTTY (creation,modify/truncate,delete,rename,links,child,
symlink through writable tmpfs) report EROFS; file upload and FS mkdir/move/remove
return actual OS error30. Original /etc/passwd bytes retained. Writable tmpfs
File/FS controls and caller/named/anonymous writes succeed as their actual
owners (caller501:20,anonymous1001:2002). New VM tmpfs resets; caller/named/
anonymous data and identities persist. Old generation operations return Conflict.
Normal restart and authenticated Host-SIGKILL stopped recovery pass; no live
session continuity claim. Explicit OCI writable root control succeeds. Installed
CLI repeats readonly effects across restart. No Box fallback in final loader set.

Final old Host3 **1/0/0,2.46s** refuses readonly creation before preparation,
without altering a valid existing v1 reservation,route or generation. No volume,
Host container,workload or fallback. This refusal is a safety boundary,not readonly
implementation proof. Final tmpfs1 **1/0/0,167.08s**,fileHostPEP1 **1/0/0,46.76s**,
named1 **1/0/0,174.54s** passed. Anonymous1 **1/0/0,431.91s** passed, including original ownership,independent
allocations,overrides,auto-remove,CLI,restart/stopped recovery and repeated file
HostRO checks. Installed BoxVm rootRO/
tmpfs safety flags/realPTY/restart regression **one scenario,2.80s** passed.
All five A–F scenarios passed193.08s: prewarm/directory actual1/0/0 each,
OCI lifecycle/HostROrestart/primary one completed scenario each.

`verify-commit1` exit0,240.95s: **Box2231passed/30ignored,CLI933passed,installer8**;
workspace/all-targets,fmt,scopedClippy,shared contracts and matrix passed.
OCI owning tests1 exit0,367.43s: **SDK217,protocol66,Runtime438/2ignored,
Kbox80/4ignored**. Linux Agent Clippy exit0,10.28s; host owning Clippy2 passed.
SDK before-v2 actual0/1/0,after5/0/0; initial Box before0/2/0,after3/0/0.
Ignored and zero-test targets are not native passes.

[All calibration failures and corrections](checks/calibration-failures.md) retained:
v1 contract rejection,old-Host fixture expecting start instead of earlier create
refusal,interactive rm in realTTY (now rm -f with same ten EROFS assertions and
30s deadline),wrong anonymous owner corrected from actual image metadata,and
positive create using LegacyOnly (caught by loader) corrected to explicit OCI
with a route assertion. Failed output/cleanup and first archives remain intact.
No failing assertion was removed or weakened.

Commands and full argv/env/results in checks receipts; run scripts retained here:

```sh
cargo test --locked -p kbox-runtime hvf_root_readonly -- --nocapture
cargo test --locked -p koci-sdk workload_ -- --nocapture
cargo test --locked -p koci-agent-protocol -p kbox-runtime readonly -- --nocapture
cargo test --locked -p koci-sdk -p koci-agent-protocol -p koci-runtime -p koci-kbox --lib
cargo build --locked --profile release-dist -p koci-cli -p koci-kbox
cargo zigbuild --locked --profile release-dist --target aarch64-unknown-linux-musl -p koci-agent
# Pinned local Alpine archive,exact Agent and native identity are in candidate3 receipts
bash crates/oci-runtime/scripts/build-macos-system-image.sh --alpine-archive … --agent … --output-dir …
bash scripts/package-macos-arm64.sh
bash scripts/verify-commit.sh
```

Exact commands replace the system-image placeholders in rootro-candidate3-builds.json.
K_HOME always owned and isolated; final native dependencies/executables originate
in installed archives. The frozen test client is separate from shipped CLI and
its hash/caller/counts recorded.

Final owned audit passed: **18 isolated homes,107 observed sockets absent,0
managed executions/volumes/owned processes/mounts/workload disks/private builders
or open descriptor holders**. Seven pre-existing RAM devices and unrelated image
untouched.15 native and121 AF baseline immutable blob files unchanged. Audit
deleted nothing; test fixtures removed only exact owned execution/volume resources
and checked retained source data first. [Cleanup](checks/final-cleanup.json),
[descriptors](checks/final-descriptors.json),
[complete receipt/count/source/route review](checks/final-receipt-review2-summary.json).
Frozen2 unchanged through review; native submodule trees clean and index empty.

Evidence writer correction: an earlier review filename collided with its command
receipt. Original stdout/command data retained, summary regenerated separately
without native reruns, wrapper now refuses to overwrite child output and writes
a separate command suffix. Actual collision self-check passed.015 printed summary
was recovered only from its existing stdout,explicitly without reconstructing
the lost script-hash field or rerunning historical source assertions.

## Explicitly not claimed

No Host physical workload-disk RO or revocation of explicitly granted Guest
SYS_ADMIN. This is a Guest root view; runtime boot may update its private disk.
Caller Host RO retains separate native PEP evidence. No full W4,Host Service/
default cutover,readiness,GA,fresh-host or platform promotion. Secret,pools,
snapshots,TEE,devices/custom network and unqualified controls remain open/refused.
Windows ARM64 current dependencies unsupported and no native run. No runtime RSS
measurement;512MiB is VM configuration only. No commit,push,publish,real~/.kbox
changes,native submodule edits or unrelated resource cleanup. Two known missing
historical OUTCOME files remain missing. Continuous local W4 work remains active.
