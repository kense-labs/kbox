# Managed Secret and raw anonymous volumes: accepted

Spec027 qualifies the existing managed RAM Secret files/environment with image
VOLUME anonymous ext4 disks on the explicit persisted macOS OciSdk route.
Required `dev.kbox.memory-secret-files` v3 and Agent transport v11 identify only
this combination. Files v1/v8, environment v2/v9 and raw v1/v7 retain their
bytes/meaning and refuse old combinations. Existing NPROC v10 may explicitly
bind transport v11. Old Host/shim/Agent refuse; no fallback is added.

Box passes the existing typed capability inventory into OCI resource planning
and requires memory3+raw1 before claiming anonymous identities. Restart checks
persisted allocations; bundle completion selects the exact new version. Host
advertises3 only when the actual projection is RAM-backed. The existing Host
RAM/source pins, native raw FD checks, Guest pre-namespace source protection,
allocation owner and ExecutionManager/generation authority remain in place.

Native1 exposed a real rootfs publication bug: generic COW copying expanded the
4 GiB sparse root disk when crossing from the disk home to a 1 GiB RAM handoff.
The Mac publisher now reuses the existing sparse extent copier, retaining
create-new, bounded I/O, sync and fail-closed errors. The non-Mac path is unchanged.
The regression checks full SHA/capacity/trailing holes/allocated bytes/source
identity/mode and no overwrite. Native3 passes at the same 1 GiB RAM capacity.

## Final installed assets and environment

macOS26.6.2(25G83), Apple M5 Max arm64, euid501, Rust1.97.1.
[Environment](checks/host-environment.json), [owned paths](run-context.json).
Final Box/global source2 SHA256:
`540b51f53a939c18fcf86f065886157bade3116e377e353e67cd0e5332ecad19`,1703 inputs.
OCI/SDK/Agent source1:
`c07659de97c45cd962d08dd29f49e882ac9266bed570226b116af767bbc1734c`.
The only source1→2 changes are four Box runtime sparse-publication/test paths;
OCI retains its actual original provenance. [Strict proof](checks/final-review3.json).

| Archive | SHA256 | Compressed / isolated installed bytes |
| --- | --- | --- |
| Box2 `final-dist-raw-secrets-2/kbox-3.4.3-macos-arm64.tar.gz` | `f9f48ab58e6ea972c98bf3c8a6774c1ecb69a541ea871b438e10c00c38e5c2c5` | 23,393,755 / 44,171,264 |
| OCI1 `oci-raw-secrets-final1-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `30eb5e29bbe0e3676a55a6c67f95b20d2752e5aacbf791a2cd70222e9f4f8d62` | 33,424,560 / 114,954,240 |

Artifact root: `artifacts/oci-raw-secrets-0291dc5e2dfb47a2981ced4ccba0ca49`.
Both archives have11 verified companion checksums and were installed/extracted
to fresh isolated prefixes. CLI, Box shim, guest-init, OCI Host/shim, Agent,
system image, architecture, ad-hoc signatures and library links are verified.
The standalone/embedded Agent SHA256 is
`a7159a45271c2843901f8cc958bbaf83355ed8ebac5b5bf24e921e79194fd566`.
[Box install](checks/install-raw-secrets-final2.json),
[OCI install and dependency identities](checks/oci-raw-secrets-final1-install.json),
[final identities](checks/final-identities-2.json).

Native library sources remain clean: libkrun
`3c2889c76460d81e3fb16d6144580fd31eb1bae3`, libkrunfw
`fc3ea06895301f1df6a274dad75fbd9317b3bfee`. OCI native archive SHA256 remains
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`;
library `7dcaf94032e22d2044aed36b7c43f17dd311fed65369f0f292c2e65337856ddf`,
firmware `841bc9d5eecbc2aeeb6098fbc75d484427680d7503f5ed9bcdfe9d072a9420d4`,
kernel `b1180b50148ed14f5fbeadf17288ce8abcf245daa468255b7ff41113bbf01199`.
Box and OCI native bundles remain distinct. Cargo.lock SHA256 is unchanged
`0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb`.

## Actual verification

Final matrix: **12 outer Rust native tests,21 top-level Guest unit tests and3
Python native scenarios**, all pass. The NPROC Guest harness additionally runs
two isolated single-test child harnesses (zero/last-slot), both1/0/0; these are
reported separately from its5 parent tests. Earlier expected29 counts in the
checkpoint were wrong; strict review checks the actual stdout/result files.

| Installed-asset scenario | Actual result |
| --- | --- |
| [Raw + environment3](checks/raw-environment-positive-3/result.json) | 1/0/0,288.12s native; Guest8/0/0; normal restart and stopped recovery across generations1/2/3 |
| [Old Host](checks/raw-environment-old-host-1/result.json), [old shim](checks/raw-environment-old-shim-1/result.json), [disk projection](checks/raw-environment-disk-projection-1/result.json) | Each1/0/0; no entry/fallback; sources preserved and owned state removed |
| [Legacy environment](checks/environment-positive-1/result.json), [legacy files](checks/environment-file-positive-1/result.json) | Each1/0/0; environment Guest8/0/0; prior v2/v9 and v1/v8 semantics retained |
| [NPROC](checks/box-native-nproc-attempt-2.json) | 1/0/0; Guest5/0/0 plus two child1/0/0; zero refusal/last slot/hard limit restore/no child left |
| [Anonymous](checks/box-native-anonymous-attempt-2.json), [Host Service](checks/box-native-host-service-attempt-2.json), [ordinary files](checks/box-native-files-attempt-2.json) | Each1/0/0; current installed Box2/OCI1/client2, actual loader records |
| [A–F](checks/af-final1/native-results.json) | All5:2 Rust(prewarm/directory)+3 Python(OCI lifecycle,Host RO/restart,process flow) |

Raw positive exercises16 actual Host write refusals per attack wave after
remount/fresh mounts/child, protected environment bytes after visible path
replacement, malformed/retarget/literal refusal before entry and metadata
redaction. Three actual disk-backed anonymous images retain exact allocation/
inode/size/owner/private mode, Guest1001:2002/mode600 and soft/hard links, writable
markers, restart and recovery identity. A separate UID501 files-only control
uses NPROC v10→v11 and all five process paths, shares no source inode and does
not disturb the main execution when removed. CLI exec/realTTY/attach2 use the
actual installed binary and `test -t`; replay causes one effect, stale generation
refuses. Sources persist until explicit owner cleanup; disks persist on kill and
are deleted by the recorded owner on remove.
[Main effects](checks/raw-environment-positive-3/native-root/result.json),
[file control](checks/raw-environment-positive-3/native-root/raw-file-control.json),
[old Agent actually executing in Guest](checks/raw-environment-positive-3/native-root/old-agent-schema-proof.json),
[old shim actual v11 refusal](checks/old-shim-v11-proof1.json),
[real CLI TTY](checks/raw-environment-positive-3/native-root/raw-secret-cli-pty/result.json).

Focused SDK8,protocol attachment21,Box production23/backend79,portable sparse
publication1/ext4 cache6 pass. Full SDK228 library+18 integration,protocol71,
OCI Runtime438/2ignored and owning Clippy pass. Final
`scripts/verify-commit.sh` exits0 in226.38s:Runtime2261/43ignored,CLI935,installer8,
workspace all-target check/fmt/scoped Clippy/contracts.
[SDK](checks/sdk-owning1.json), [protocol](checks/protocol-owning1.json),
[OCI runtime](checks/runtime-owning1.json), [Clippy](checks/oci-owning-clippy1.json),
[sparse regression](checks/portable-disk-after1.json), [verify2](checks/verify2.json).
Every receipt includes the actual command, exit code and retained log.

The Guest test ELF originally exceeded the unchanged32 MiB file-upload limit.
Only nonallocated debug/symbol tables were stripped using the existing LLVM
toolchain:78,615,168→24,916,008bytes. Original/debug-only/failed Zig outputs are
retained. An independent ELF parser proves all14 allocated code/data/unwind/
TLS/BSS sections and the entrypoint identical; real Guest execution/counts above
remain required. [ELF proof](checks/guest-tests-allocated-sections4.json).

## Failures, commands and cleanup

[Failures](FAILURES.md) retains red tests, duplicate capability test-fixture
registration, draft compilation failures, native1 ENOSPC/native2 upload rejection,
all candidates, failed stripping and audit/review format/count mistakes. No
assertion, upload limit or native RAM size was weakened. Verification1 is kept
with its original source and is not the final result.

Commands/drivers under the artifact root: `build-oci-candidate1.py`,
`package-oci-final1.py`, `build-box-final2.py`, `check-final-identities2.py`,
`run-memory-native2.py`, `run-native.py`, `qualification-runner-final1.py`,
`final-resource-audit3.py`, `final-review3.py`. Their commands and results are
retained in checks; native drivers are hashed in
[driver identities](checks/native-driver-identities2.json).
Previous archives and SHA records remain in
[prior identities](checks/prior-artifacts-retained.json).

Final audit covers17 homes(13 disk and4 detached RAM homes with actual installed
CLI empty inventories captured before detach),8 exact owned RAM image closures,
119 observed socket paths, owned processes/disks/FDs and all7 initial RAM devices.
Zero managed executions/volumes/owned processes/mounts/disks/sockets/FD holders.
Native23 and A–F381 baseline cache files retain bytes/owner/mode; only validated
index metadata updates and one exact owned A–F build are admitted. Audit deletes
nothing; the unrelated mounted image and original RAM devices are untouched.
[Cleanup](checks/final-cleanup.json), [RAM closure](checks/final-ram-closure.json),
[FD holders](checks/final-fd-holders.json), [cache](checks/final-cache-audit.json),
[strict final review](checks/final-review3.json).

## Explicitly not claimed

Full W4/OCI Host Service/default cutover, live-session continuity, arbitrary
Secret/storage combinations, unqualified controls, fresh-host/conformance/soak
and other-platform matrices remain open. Default macOS is BoxVm; HVF recovery is
stopped-only. Windows ARM64 dependencies unsupported/no native run; support and
readiness levels are unchanged. Runtime RSS is unmeasured. No real Secret,
real ~/.kbox modification, native-submodule edit, unrelated cleanup, commit,
push or publication. The two missing historical evidence files remain missing.

Continue the existing W4 custom writable-root disk-capacity gap. The current Box
qualification refuses nondefault disk_mb despite the existing bounded ext4
builder and rootfs-image identity contract. Trace and prove that route before
admitting any bounded new value; preserve all other unsupported-control refusals.
