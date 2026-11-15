# Spec018: actual memory-backed Secret file grants

Status: **bounded spec018 accepted** on the existing macOS26.6.2 (25G83),
M5 Max arm64, euid501, Rust1.97.1 host. Continuous local W4 work proceeds.
No platform, readiness, default-route or release promotion.

The explicit OciSdk route now carries already-authorized Box-managed regular
Secret files through existing exact readonly file grants. Required SDK
memory-secret-files v1 / Agent manifest v8 binds the memory constraint and
Secret classification. Existing v1–v7 bytes and meanings remain intact. There
is no second mounter, resource authority, config system, workspace or dependency.
ExecutionManager identity, route and generation fencing remain authoritative.

Actual Host/shim enforcement validates source and projection against native
mounts and hdiutil ram:// device identities. APFS is followed from the exact
volume through every PhysicalStore; all stores must be real RAM images.
A shared SDK platform helper serves Box, Host capability probing and Shim PEP.
The Host advertises this capability only when its configured runtime share is
actually RAM-backed. Existing pinned descriptors revalidate identity after
Seatbelt confinement. Source numeric owner and mode are preserved. Cross-FS,
physical disk roots, unsafe file aliases, missing required capabilities,
raw anonymous disks plus Secret files, and environment projection refuse.
Refusal is a safety boundary, not implementation of those combinations.

## Final installed assets

Artifact root: `/Users/roylin/code/kense/kbox/artifacts/oci-secrets-3b9553cebce5421bb376de8b21634a4e`.
Frozen3 source: **e1aa0d07bea7f853f81ff91e7f9cad778035a132c765cfab17bb3a4adca15585**,
1688 inputs. Native test client:
**bad58865dc903703ad11372c7e4e86b5bc7b6d32a080d47b6f95613b7e72ad65**.
Cargo.lock remains 0f82d0eaafd8b255f5a73c9de5a9106fc85f711bc2ed1d4ba9c02ef07aa63dbb.

| Archive | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| final-dist-secrets-3/kbox-3.4.3-macos-arm64.tar.gz | 9ad22c544c91a835ba78ee88b197ae90b2a512ba651985f0d1f1f3dba6a0925c | 23,384,813 | 44,154,880 |
| oci-memory-final3-dist/koci-hvf-0.3.6-macos-arm64.tar.gz | 3331cbc98ba351465226fe42d960015fcb90e1013069fbb32532a21ef99a0bdf | 33,372,500 | 114,892,800 |

Both archives were freshly installed to installed-secrets-final-3 and
oci-memory-final3-unpacked/koci-hvf-0.3.6-macos-arm64. All 11 SHA256SUMS entries
per installation, source provenance, arm64 Mach-O signatures/architecture,
AArch64 ELF companions, dependency paths and the embedded Agent are verified
in [identities](checks/final-identities-3.json). The new Agent is becf4831806d8fa3afde16356dee736f8052cb8277c07c3469dc5d554ebb9d17;
its ext4 image is ecd801bf1b1688a39c1bf5baf3a21389f5367409c7124e180d4e54b7b0ed161d.
Guest Init remains f2b6d327… . Box retains independent lib9ab3eefe…/FWca7bba4e…;
OCI retains lib7dcaf940…/FW841bc9d5… from selected native archive d7a398ec… .
Both parent native repositories remain clean at their original gitlinks.
Candidate1/2 archives, their checksums/provenance, every failure and calibration
remain. Final3 only changes two native test files relative to production2.
Every runtime loader in final acceptance uses the installed archive companions.

## Actual checks

SDK red tests actual0/2/0 and protocol red actual0/1/0 established missing
constraints/downgrade refusal. Focused Runtime2/SDK3 and shared RAM parser3
passed. Owning tests2 passed SDK223, protocol67, Runtime438/2ignored,
Kbox80/4ignored. Scoped Clippy passed. Final verify-commit3 exited0 in210.76s:
Box Runtime2238 passed/33ignored, CLI934 passed, installer8; workspace,
all-targets, fmt and required scoped checks passed. Exact commands and complete
logs are in checks; ignored tests alone are not native evidence.

Final installed Secret positive7: **actual1/0/0**, 228.53s including native
provisioning, generations1→2→3, CLI, init/captured exec/child/stream/realPTY,
16 actual Host denials and two fresh RW mounts per wave, unchanged source
identity/mode/bytes, replay one effect and changed request refusal, stale
fencing, independent material, normal restart and Host-SIGKILL stopped recovery.
Cleanup detaches aliases and preserves sources until the existing Secret owner
explicitly retires each identity. Synthetic material only; evidence contains
hashes, never material. [Native result](checks/secret-positive-7/result.json).

Old Host2, old Shim4, old Agent2 and physical runtime-root2 each executed
**actual1/0/0** and cleaned their exact RAM devices. Negative assertions check
an unchanged writable entry marker, source identity/mode/bytes and no workload
Start. Failed Create cleanup uses the established exact pending replay and
force-delete path, with zero added Start receipts; protected unbound kill/remove
refusals remain. Old Agent transport failure alone was insufficient proof:
the exact retained 3f3f16a7… ELF also ran in a real current HVF Guest against
its actual v8 manifest and explicitly returned unsupported-schema/exit1, with
its temporary Guest mount removed. [Actual old ELF proof](checks/secret-positive-7/native-root/old-agent-schema-proof.json),
[old image case](checks/secret-old-agent-2/result.json).

Final A–F: all five scenarios PASS in151.80s (OCI lifecycle/build/export/commit,
Host RO plus restart, primary process flow, prewarm, directory authorization).
Prewarm and directory each executed1/0/0; other drivers completed one native
scenario each. [Actual counts and result files](checks/af-final3/native-results.json).
The final installed file/named/anonymous/rawRO/rootRO/tmpfs/ulimits regressions
all executed **1/0/0 each**. Their respective elapsed times are46.80/167.72/471.70/8.02/321.91/167.93/201.53s; complete per-case commands, stdout/stderr,
results and installed loader maps remain in checks and the owned private root.
The [final receipt review](checks/final-review3.json) verifies source stability,
archive companion hashes, all actual counts and result files: **14 Rust native
tests and3 completed Python native scenarios**, without counting ignored tests.

Final [owned resource audit](checks/final-cleanup2.json) covered **16 existing
isolated homes and134 observed socket identities**, all absent. Managed
executions/volumes, owned VM/Host/shim/engine processes, mounts, workload disks,
private builders and FD holders are zero. RAM cases additionally retain their
pre-unmount logical cleanup and exact ownership receipts. All seven baseline
RAM mounts and the unrelated mounted image are preserved. The audit deleted
nothing. It checks all23 main and213 AF baseline cache files: every original
blob and immutable metadata file is unchanged. Only last_used/path updates on
existing index entries and one exact successful AF OCI build entry are admitted
and recorded in [cache audit](checks/final-cache-audit2.json); original identity,
digest, size and pull metadata remain exact. The initial index bytes were
recovered from the unchanged prior cache only after matching the original hash.
[FD audit](checks/final-fd-holders2.json), [environment](checks/environment.json),
[source/asset/count review and script hashes](checks/final-review3.json).

Exact invocations include owning cargo tests/Clippy recorded in checks,
`bash scripts/verify-commit.sh`, `bash scripts/package-macos-arm64.sh`, the pinned
OCI Host/Shim/Agent/system-image build commands, `run-memory-native-apfs.py`
positive7/old-host2/old-shim4/old-agent2/disk-projection2, `run-native.py` attempt3
for all seven affected modes, and qualification-runner.py with af-context-final3.
All test execution/loader environments use the actual installed archives.

## Failure retention and limits

[FAILURES.md](FAILURES.md) records invalid fixture identities, HFS sparse-file
ENOSPC, explicit owner requirements, protected failed-Create cleanup, loader
recovery calibration, old-Agent console limits, wire-key/compile failures and
the stale archive directory literal. Failed wrappers remain failed even when
an inner Rust test passed; fresh complete receipts supply acceptance evidence.
OldShim2 post-child cleanup verified exact APFS UUID, RAM owner identity and
no FDs before detaching; the original failed wrapper receipt is unchanged.
OldShim1's archived Killing record is historical evidence from reclaimed test
RAM, not a successful logical cleanup receipt.

## Explicitly not claimed

Secret environment projection and raw storage combinations are not implemented
by this slice. Memory backing means validated ram:// storage; it does not claim
host swap/hibernation/crash-dump resistance or measured runtime RSS. No real
Secrets, real ~/.kbox or unrelated resources are touched. Full OCI Host Service
cutover remains incomplete; default macOS remains BoxVm and HVF recovery is
stopped-only. Windows ARM64 remains unsupported by current dependencies and
has no native run. No fresh-host, other-platform, support/readiness promotion,
VMM/firmware port, formal publication, commit or push is claimed. The two known
missing historical OUTCOME files remain missing and are not reconstructed.

The next bounded local W4 phase is019 managed Secret environment delivery.
Proceed immediately; this accepted slice does not end the continuous task.
