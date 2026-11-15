# Spec011 named VolumeStore mounts

Status: accepted bounded named-volume slice on this existing macOS arm64 host.
Final installed native matrices, A–F and scoped resource audit passed.
Default routing, support levels, W4/readiness and enterprise GA remain unchanged.

## Implementation and observed failures

The current Mac profile admits existing named VolumeStore allocations through
ExecutionManager → persisted OciSdk → public SDK → independent HVF Host.
Existing resource guard owns references; existing storage attachments use
Caller/DetachOnly. Each named allocation has one exact RO/RW bind per execution;
unsupported driver/quota/duplicate or missing binding refuses before claims.
Ordinary binds, file grants, BoxVm and exact generation fencing retain their
contracts. There is no second resource state machine or configuration/dependency.

Before regressions proved: profile refusal (0/1); raw mutable-name storage IDs,
corrupt catalog movement and unenforced driver (0/3); traversal/corrupt-catalog
mutation (0/3); normalized name/ admitted (0/1); copied prior config recreating
the same allocation ID (0/1). Synthetic traversal/deletion targets were owned
temporary canaries, never user data. Existing alternate-case canonical alias
guard passed; no additional filesystem identity adapter was added.

Normal VolumeStore get/list/prune/mutations now read strictly and preserve bad
catalogs; explicit legacy load recovery API remains for compatibility. Names
are bounded plain components, with separators/colon/NUL refused before IO.
Canonical allocation aliases, inconsistent paths/drivers and malformed/duplicate
allocation identities refuse. Saves reuse the durable atomic writer. New
materializations receive protected store-issued UUIDv4 in existing labels;
copying an old config creates a new UUID. Existing no-label catalogs delegate
stable validated legacy identities without rewriting their bytes.

Native attempt1 (installed prior OCI7):0passed/1failed/0ignored,42.08s. Host
RO14denials+child passed, then guest returned only cached 5-byte length after
Host grew the file to57bytes. `named-live-observation.json`, worker loader logs
and cleanup remain. Shared Mac shim now uses the already-pinned uncached native
API for caller directories. Runtime-control share and native/FW/kernel are
unchanged; no fallback to cached/writable/different isolation.

Native prototype2:1/0/0,148.52s; live growth, shared references, stable IDs,
restart and stopped SIGKILL recovery passed. cliChecked=false and candidate1
assets mean this is explicitly a prototype, retained separately from final3.

First full verify failed101 with Box2201passed/4failed/20ignored. Contract
volume resolver compared exact label counts and rejected the new managed UUID.
The shared caller now permits only the allocation key in addition to exact role/
spec/output labels; unknown and conflicting labels still refuse unchanged
catalog bytes. All8 artifact caller tests pass. No assertion was dropped.
A receipt-review helper initially assumed A–F map fields for the Host probe's
integer14 summary; it failed TypeError, was corrected against the owning helper,
and asserts all actual native fields. `checks/receipt-audit-correction.json`.

## Frozen installation assets

Source HEAD405c8b89f151e9e2879438d9e457dd60bfd18b90 with all retained dirty
work; source-input SHA256 `07ccdf500f00db25334d538ebed0dd1bcf10dd11e3dfaf417ca45f79cad2ef35`. Full included paths/digests
are in `artifacts/oci-named-d21528e85dea4edf863f24f89312cd0b/source-frozen-1.json`.
Box and OCI were rebuilt from matching source, not copied from old binaries.

| Asset | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box3.4.3 final1 | `d4209a7ff4048690770089126c0026bfc09abe0cc26d5b074992cf8fc83595cc` | 23344608 | 44081152 |
| OCI0.3.6 final1 | `5f9578d20cf4db524b7fb83223443f83d0aca257688370b854772d191c478288` | 33334546 | 114851840 |

Actual archive/prefix paths are in [run-context.json](run-context.json).
All prior Box/OCI archives plus checksum sidecars are preserved in this run's
`dist-before-named/` and [checks/preserved-archives.json](checks/preserved-archives.json).
Both installers verified11file hashes. Actual CLI/shim Mach-Oarm64 signatures,
architectures, dylib dependencies and native libraries were inspected; static
ELFaarch64 guest-init/Agent architectures and ext4-embedded Agent equality checked
in [checks/final-identities-1.json](checks/final-identities-1.json).

Native source remains selected from the separately qualified uncommitted009
libkrun worktree; no new native edit here. The OCI bundle sidecar/manifest binds
archive `d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`, selectedlibkrun7dcaf940…,
firmware841bc9d5… and kernelb1180b50…. BoxVm keeps its own prior baseline native
libraries, independently checked. Complete digests are retained in the identity
record, included build-info.json and native-source.json; no parent gitlink switch.

## Actual commands and counts

Existing macOS26.6.2(25G83), Apple M5Max arm64, euid501, pinned Rust1.97.1.
All runtime homes use owned artifact/private paths from context; no ~/.kbox.
Scripts under this run's artifact root record exact argv/env and output files.

- `cargo test --locked -p kbox-runtime --lib named -- --nocapture`:
  24passed/0failed/1ignored after5; ignored native was actually executed separately.
- `cargo test --locked -p kbox-runtime --lib kbox_contract_driver::artifact_tests -- --nocapture`:
  8passed/0failed/0ignored after caller fix.
- `cargo test --locked -p koci-kbox --all-targets`:75lib/3ignored,
  18binary/3ignored,3integration pass. No ignored native success claim.
- `bash scripts/verify-commit.sh`:exit0,218.65s; Box2206passed/20ignored,
  CLI933passed, all contract suites, workspace all-targets/fmt/scopedClippy and
  installer8actual tests pass. [Actual per-binary counts](checks/actual-counts-source1.json)
  retain separate summaries, including0testdoc targets and child invocations;
  do not sum repeated child runs as independent coverage.
- `cargo clippy --locked -p koci-kbox --all-targets --no-deps`:exit0,3.73s.
- `python3 <run>/build-box-final1.py`, `build-oci-candidate2.py` and
  `package-oci-final1.py`:exit0; isolated install/metadata/identity assertions pass.
- `python3 <run>/run-named-final.py <context> 3`:1/0/0,163.51s,
  **cliChecked=true** using final installed Box1/OCI1. Live RO/RW, shared data,
  same storage identity/references, in-use remove refusal, retries, normal restart,
  old-generation rejection, reconstructed off-policy client and stopped Host
  SIGKILL recovery pass. Three native RO probes each report14Hostdenials plus
  child and writable-remount/fresh-mount conditions. No EROFS substitute.
- `python3 <run>/run-failed-create-final.py <context> 5`:1/0/0,39.82s,
  actual installed CLI unbound-kill guard and exact SDKCreate/delete recovery
  pass with zero added workload Starts, preserving caller files.
- `python3 <run>/qualification-runner.py <af-context-final1>`:all five actual
  installed scenarios pass (OCI lifecycle, HostRO, primary process flow,
  prewarm and directory grants). Latter two each1/0/0 Rustentries; first three
  one completed scenario each. Actual result files and loader proofs in
  [checks/af-final1/native-results.json](checks/af-final1/native-results.json).
- `python3 <run>/run-full-final.py <context> 4`:1/0/0,377.26s,
  cliChecked=true. Actual file/rootfs/mixed-route/cgroup/Exec/Host SIGKILL/fault
  relay matrix passed using both installed archives. Five file proof receipts
  each assert16Hostdenials, child, fresh mounts and live writeback; concurrent
  source independence, replacement pin/restart refusal and caller preservation
  passed. Foreign root ownership and private rootfs lifetime remain qualified.
- `python3 <run>/final-resource-audit.py` and `descriptor-audit.py`:exit0;
  actual inventories described below; audit itself deleted nothing.

## Data and cleanup

Named final3 runtime cleanup preserved all3named metadata/data records with
empty reference lists; RO input device/inode/uid/gid/mode/linkcount and bytes
unchanged. Only afterward the explicit test owner removed its three known
fixture allocations. Retained snapshot and final fixture cleanup are in
`<privateRoot>/attempt-3/`; [checks/named-receipts-checked.json](checks/named-receipts-checked.json)
rechecks the actual receipts. Attempt1/2failed/prototype cleanup also retained.
Final audit covers11owned homes across all attempts and A–F,0executions,
0volumes,0owned runtime processes/mounts/workload disks/descriptor holders,
98observed socket paths all absent and8immutable source blobs unchanged.
Seven inherited RAM devices and the unrelated mounted image remain untouched.
See [checks/final-cleanup.json](checks/final-cleanup.json), per-home inventories,
file-source preservation record and [descriptor audit](checks/final-descriptor-audit.json).
All failed/prototype and final-attempt receipts, packages and source snapshots
remain. Final source snapshot unchanged, index empty and parent native repos
clean with unchanged gitlinks. No cleanup of user or earlier run resources.

## Explicitly not claimed

No full Box parity, anonymous image VOLUME, Secret lifecycle, pools/snapshots,
TEE/devices/custom networks, unrestricted quotas or arbitrary old-library updates.
Full OCI Host Service/default cutover, live-session continuity, fresh-host and
other-platform matrices, conformance/security review and long soak remain open.
Windows ARM64 dependencies unsupported, no native run; no Windows waiting/port.
No runtime RSS measurement; package bytes above are not runtime memory.
No commit, staging, push, publication, parent native edit, real user home change,
readiness flag or platform-support promotion. Named slice closed; continuous W4
work proceeds next to image-declared anonymous VolumeStore allocations using
existing Box authority. All other open gates remain open, not externalized as
a reason to stop this task.
