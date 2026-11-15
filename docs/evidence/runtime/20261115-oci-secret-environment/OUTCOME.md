# Spec019: managed Secret environment from protected RAM sources

Status: **bounded spec019 accepted** on this existing macOS26.6.2 (25G83),
M5 Max arm64, euid501, Rust1.97.1 host. Continuous W4 work remains active.
No default-route, platform, readiness or release promotion.

The existing non-sensitive K_BOX_SECRET_ENV_V1 manifest now uses explicit
required memory-secret-files version2 / Agent manifestv9 on the opt-in Mac OCI
route. Files-only version1/manifestv8 and older transports retain meaning and
encoding. SDK validation binds each variable and canonical reserved path to an
exact readonly memory-file grant, with bounded names/bytes/counts and explicit
old capability refusal. Box validates source/target/mode and required version
before preparation. No plaintext material enters the bundle or public request.

The existing Agent init/exec consumer pins readonly protected sources before
entering workload namespaces. It reads only after Start and before workload
credentials, then consumes/removes the manifest. Descriptor CLOEXEC prevents
passing source handles to the app. A privileged workload replacing its visible
Secret path cannot change this source. New fields in existing VM storage/private
process metadata carry only bindings/paths and are omitted when absent; legacy
Secret materialization keeps exact400/nlink1 validation. Bound aliases allow
only the one original/runtime alias pair already enforced by the Host. Pure
metadata validation rejects retargeting/literal collisions before private process
plan writes. No new state authority, workspace, config system or dependency.
ExecutionManager identity, persisted routes and generation fencing remain exact.

## Final installed assets

Source **03a62c3e7d246fa4f8eb1e758d338aa3fc9bb25a75ee6f6ce94259c736986b35**,
1692 inputs; native client25fe6073727a7651cfa0fe77f51ab1150d677eba7e4d6123b415ada8761ebff4.
Artifact root: /Users/roylin/code/kense/kbox/artifacts/oci-secret-env-1ac68d946a6840fe9fff3d0b9791a84f.

| Archive | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| final-dist-secret-env-2/kbox-3.4.3-macos-arm64.tar.gz | b9b77584fb0776990717c511e6676166c3e5b0c780e0faccd425c5bd2d1f6df9 | 23,390,394 | 44,154,880 |
| oci-environment-final2-dist/koci-hvf-0.3.6-macos-arm64.tar.gz | 542bfe43dd6f2837ad55c3ba79416832584e9032a9b6e1b2bfa4d75bc94203bf | 33,417,962 | 114,925,568 |

Fresh prefixes are installed-secrets-final-2 and
oci-environment-final2-unpacked/koci-hvf-0.3.6-macos-arm64. All11 companion
hashes per archive, source provenance, signatures, native/ELF architecture,
dependency resolution and embedded Agent match in [identity checks](checks/final-identities-2.json).
Agent fa5e1d36b31083388ec902c6a8dde1de7953791a69f63dabf4d88f5c51954589; image 77b2ab47caf668d28ce0dc807cb341cdc4b534c14ddd43989c9d760a509b3886.
Guest Init remains f2b6d327…; Box lib9ab3eefe…/FWca7bba4e… and OCI
lib7dcaf940…/FW841bc9d5… remain independently selected. Parent submodules stay
at their original clean gitlinks. Cargo.lock remains0f82d0ea… . Older archives,
sidecars, candidates and all failure/calibration evidence remain preserved.

## Actual checks

SDK before1 actual0/2/0; protocol before2 actual0/1/0. Current focused SDK7,
complete protocol68, Boxpreflight2 and Linux all-targets check3 passed. Owning
source1 tests passed SDK227, protocol68, Runtime438/2ignored, Kbox80/4ignored;
these owning inputs did not change in source2 (only Linux Agent safety path did).
Scoped owning Clippy passed. Source2 verify-commit2 exited0 in201.26s: Box
Runtime2238 passed/34ignored, CLI934 passed, installer8, workspace/all-targets,
fmt and scoped checks passed. Ignored and cross-compiled tests are not native.

Candidate1 actual native calibration passed1/0/0, including8/0/0 actual Linux
Guest tests, five process paths/CLI, three generations, Host denials, retarget/
literal refusal, visible path substitution with original injected hash,
independent owner, replay/fencing and stopped recovery. Its old018 Agent
becf4831… explicitly rejectedv9/exit1 in the real Guest. Exact RAM cleanup0/0
and no owned process remained. It is not relabelled as source2: the early
metadata validation correction required rebuilding affected assets and Guest
unit ELF. Source2 Guest test ELF is e6bbea5a58e5bb809e8102ed932b2b31844bd93c1d3bc7b3bd3678426818dc88,
7,543,640bytes; it was actually executed in the final2 Guest (8/0/0).

Final2 native environment and retained files-onlyv1 each executed1/0/0, with
three generations, exact source owner/mode/bytes, replay one effect, changed
request refusal, stale fencing, CLI, init/exec/child/stdin stream/PTY, independent
material and authenticated stopped recovery. Environment injection remains
correct after the privileged Guest moves its visible Secret directory and
replaces that path; real direct reads observe the replacement while subsequent
environment hashes still identify the original protected source. Retargeting
and literal collisions are refused with an unchanged entry marker. Raw, JSON-
escaped and Base64 material is absent from persisted runtime metadata and logs.
The exact old018 Agent becf4831… natively returns unsupportedv9/exit1; the
files-only regression separately runs the older3f3f16a7… ELF againstv8 and
observes its explicit refusal. [Environment result](checks/environment-positive-2/result.json),
[actual Guest8 counts/hash](checks/environment-positive-2/native-root/secret-guest-units.json),
[oldv9 ELF proof](checks/environment-positive-2/native-root/old-agent-schema-proof.json).

Old Host, old Shim, old Agent and physical runtime root each executed1/0/0.
They preserve a writable entry marker and source identity/mode/bytes, with no
workload entry or writable fallback. Exact pending Create reconciliation and
force-delete preserve protected unbound kill/remove guards and add zero Start
receipts. Every owned RAM case unmounted/detached0/0. The seven affected native
file/named/anonymous/rawRO/rootRO/tmpfs/ulimits cases each executed1/0/0. Their
actual final result files, argv, loader maps and times are checked in the
[final source/asset/count review](checks/final-review2.json): **15 outer Rust
native tests,8 actual Linux Guest unit tests and3 Python native scenarios**.
Ignored tests and cross builds are counted separately.

A–F fresh absolute-path retry passed all5 scenarios in141.08s: OCI lifecycle /
build/export/commit, Host readonly/restart, primary process flow, prewarm and
directory grants. Prewarm/directory each execute1/0/0; the other drivers each
complete one native scenario. [A–F result files and counts](checks/af-final2-retry1/native-results.json).
The original outer A–F failures and the ulimits missing-probe failure remain
failed; no assertions were removed. New absolute-path invariants and exact
probe hash checks fixed those fixture causes, then full retries passed.
[Failures and corrections](FAILURES.md), [context](run-context.json).

Final [owned resource audit](checks/final-cleanup.json) checks **19 existing or
prepared isolated homes and175 observed socket paths**, all absent. Managed
executions/volumes, owned VM/Host/shim/engine processes, mounts, workload disks,
private builders and FD holders are zero; reclaimed RAM cases also retain
pre-unmount ownership and logical cleanup receipts. All seven initial RAM
mounts and unrelated resources remain unchanged. The audit deletes nothing.
All23 native and227 AF baseline cache files are checked; original blobs and
immutable metadata are unchanged. Mutable index changes only update last_used /
owned current paths, plus two exact owned OCI build refs matched to real inner
CLI results (the failed outer AF attempt and the fresh accepted retry). The
original index bytes are retained under verified initial hashes.
[Cache audit](checks/final-cache-audit.json), [descriptor audit](checks/final-fd-holders.json),
[environment and dependency commands](checks/environment.json).

Commands include cargo focused/owning tests and scoped Clippy, Linux-target
Agent check/Clippy, cargo zigbuild Guest unit ELF preparation, the Box package
script and pinned OCI Host/Shim/Agent/system-image build, verify-commit2,
run-memory-native.py attempt2 for all6 memory cases, run-native.py attempt2 for
six affected modes and attempt3 for the repaired ulimits fixture, and the fresh
qualification-runner-final2-retry1.py. Full argv/output/count/identity/script
hashes remain in checks. Linux Clippy exits0 with the retained Tokio deprecation
and a nonfatal borrow-style warning; no warning-free result is claimed.

The next bounded local W4 gap is Box CPU-time ulimit intent through the existing
OCI process executor. Continue directly; accepted019 does not end the task.

## Explicitly not claimed

Raw anonymous disk plus memory Secret combinations, arbitrary file aliases or
physical backing, live-session continuity, full W4/default OCI Host Service
cutover and platform/readiness promotion remain unqualified. macOS default stays
BoxVm; HVF owner death remains stopped-only. Windows ARM64 current dependencies
remain unsupported and have no native run. Cross-compilation and this existing
Mac host do not establish another platform or fresh-host acceptance. No VMM/
firmware port, real Secrets, real ~/.kbox mutation, unrelated resource cleanup,
commit, push or publication. Runtime RSS is not measured; RAM backing does not
claim swap/hibernation/crash-dump resistance. The two known missing historical
OUTCOME files remain missing and are not reconstructed.
