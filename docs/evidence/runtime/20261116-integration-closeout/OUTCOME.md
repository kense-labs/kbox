# Integrated macOS baseline: commit and push closeout

The user explicitly requested convergence, commit, merge and push, superseding
the earlier no-commit/continuous-next-feature scope. No new W4 feature was started.
Accepted003–030 work was committed as
`0634a7e9e198dc0bfde7d60a6ffccfcaa15652ac`; upstream main
`e4a68092c3f60ad5bee8578c2bd334be3e3a37ed` was integrated in
`466f52caf5f53c60fc4589dbe110675bced0a523`. Final delivery also includes the
shared image-builder fix and this evidence. [Tasks](TASKS.md), [context](context.json).

## Integration changes and checks

Two shim conflicts preserve Host RO prepare/confine/reverify while retaining
upstream unsafe contracts and explicit FFI blocks. The retired duplicate ulimit
parser and business assembly implementation were not restored. The incoming wire
inventory now classifies11 retired business contracts as consumer-only; actual
cross-repo sets remain17 kbox/28 consumer. The pinned KCL rev is unchanged from
the previously used clean parser checkout, now referenced by its immutable Git
revision. Root workspace/lints and the new containerd-shim member are preserved.

CI and the documented Windows check target the actual supported x64 MSVC ABI;
the dependency guard still rejects GNU and ARM64. Add one concrete SAFETY argument
to the existing readonly-FD pwrite denial assertion. No lint allowlist, runtime
assertion, capability ceiling or boot deadline is relaxed. Required C/PTY probe
sources are versioned byte-identically. [Fixture identities](checks/versioned-probe-sources.json).

Actual checks: workspace layout19members, unsafe35functions, cross-repo wire and
KCL, core Compose35, runtime build-plan8, shim31, whole-workspace Clippy and
Windows x64 MSVC stub Clippy all pass. Windows is compile-only evidence.
Final `scripts/verify-commit.sh` exits0 in217.76s: Runtime2274passed/46ignored,
CLI935,installer8,workspace all-target/fmt/contracts/scoped Clippy.
[Layout](checks/layout2.json), [unsafe](checks/unsafe1.json),
[wire](checks/wire2.json), [KCL](checks/kcl1.json),
[whole Clippy](checks/workspace-clippy2.json),
[Windows Clippy](checks/windows-clippy2.json), [final verify](checks/verify3.json).
The machine hook still warns that lefthook is unavailable; these gates were run
manually and no hook was disabled.

## Rebuilt and freshly installed artifacts

Existing macOS26.6.2(25G83), M5 Max arm64, euid501, Rust1.97.1.
[Environment](checks/host-environment.json), [asset context](run-context.json).
Compiled source inputs SHA256
`d8f86d9b3e6fbcc137d0ee16448d2d00437b8ab06ae417691faeb6631d435e8c`,1708 inputs,
from merge commit466f52c. Frozen native client
`7cd527b76f0e89a4f686ab85b1eb07a5f998c3ba3b37b90f22d6cae1ecbfafa5`.

Artifact root: `artifacts/integration-d5cfdcaeba8544c38f9c8f1fae5974fd`.
Private root: `/private/tmp/kbox-integration-d5cfdcaeba85`.

| Final archive under artifact root | SHA256 | Compressed / installed bytes |
| --- | --- | --- |
| `final-dist-integration-1/kbox-3.4.3-macos-arm64.tar.gz` | `db1ddbef1296ae17b46a8ff84b3d618e45a49eab1b2992c41a83bf104f91d5ff` | 23,400,361 / 44,175,360 |
| `oci-merged-final1-dist/koci-hvf-0.3.6-macos-arm64.tar.gz` | `77171eb4743e37bb5ee6c3599519836f6dc20c2d042d50df76743316e1caeacb` | 33,425,445 / 114,954,240 |

Box, OCI Host and Agent were recompiled. Each archive was installed/extracted to
a new isolated prefix and has11 verified companions. CLI/shim/guest-init/native
libraries, architecture, ad-hoc signatures, embedded Agent and build provenance
pass. Cargo.lock SHA is
`c10845a30dd6cdf3a77b06afa54c6d271961f7b215b4af600b2fab482a36e129`.
Native gitlinks remain clean: libkrun3c2889c76460d81e3fb16d6144580fd31eb1bae3,
libkrunfwfc3ea06895301f1df6a274dad75fbd9317b3bfee. The two distinct native bundles
are retained; OCI archive dependency SHA stays
`d7a398ec8afe454b0c77452adf1feab1ff88240be51a6199eee37d0076f024bc`.
[Box install](checks/install-integration-final1.json),
[OCI install](checks/oci-merged-final1-install.json),
[identities](checks/final-identities-1.json).

The OCI system-image step initially referenced the deleted nested Cargo.toml.
The shared helper now reads the actual Agent package version0.3.6 rather than
root workspace0.1.0. Its separate source SHA is bound in OCI build-info and
[fix record](checks/system-image-builder-fix.json); this shell change does not
change the compiled input digest above. The affected image/package were regenerated.
Actual Agent `a7159a45271c2843901f8cc958bbaf83355ed8ebac5b5bf24e921e79194fd566`,
system image, OCI shim and native payloads remain byte-identical to030; Box
CLI/shim/guest-init and OCI Host CLI differ. These facts are measured, not assumed.
[Payload comparison](checks/premerge-payload-comparison1.json).

## Installed native acceptance and retained failure

Final selected matrix: **5 Rust native tests +3 Python native scenarios**.
Files,cap_add and shm_size each actually execute1/0/0 from the new installed
archives/client. They preserve Host file RO/restart,45 capability observations
(including nonroot exec and dropped/ALL controls),25 shared-memory observations
plus3 boundary/default inspectors, actual syscalls/denials,CLI/realPTY,replay,
fencing and three generations with stopped recovery. Combined SYS_ADMIN Host
file attacks still record16 actual denials in each of two generations.
[Files](checks/box-native-files-attempt-1.json),
[capabilities](checks/box-native-cap-add-attempt-1.json),
[shared memory](checks/box-native-shm-size-attempt-1.json).

The first A–F aggregate remains **FAIL**: OCI RUN warm-pool boot exceeded the
unchanged15000ms Guest exec readiness deadline during parallel validation. Its
other four cases passed. A fresh, isolated OCI-only retry with the same source,
assets,image and assertions passed in36.04s and retained13 live log snapshots.
Final-AF references those four actual passes and that one actual retry; its
five scenarios are2 Rust1/0/0 plus3 Python. No generic printed summary is counted.
[Original aggregate](checks/af-final1/native-results.json),
[actual single retry](checks/af-final2/native-results.json),
[selected final results](checks/final-af-results.json),
[reconciliation/limitation](checks/af-reconciliation.json).

**The original intermittent warm-boot timeout is not proven fixed.** Its first
per-box logs were removed by the existing finally cleanup, and its cause was
not reproduced by the isolated retry. This remains a scheduling/startup risk
for the open concurrency/soak qualification work, not an invisible pass.
[FAILURES.md](FAILURES.md) retains all build/gate/preparation failures.

## Cleanup and explicit limits

Audit checks all9 test homes, including the failed build home without a managed
catalog. Zero executions,volumes,owned VM/shim/Host/engine processes,mounts,
workload disks or FD holders remain; all99 observed socket paths are absent.
Initial7 RAM devices and unrelated resources stay intact. All35 native/465 A–F
baseline cache files retain bytes/UID/GID/mode, with validated index changes and
one exact owned A–F image. All17 probe sources retain pre-grant identities and
bytes; no audit deletion occurs. [Cleanup](checks/final-cleanup.json),
[sources](checks/final-caller-sources.json), [strict review](checks/final-review1.json).
The38 historical native-root scratch trees remain locally with an identity
inventory in [LOCAL-RUNTIME-STATE.json](../LOCAL-RUNTIME-STATE.json).
Final [source, dependency and documentation reference closure](checks/final-source-references.json)
checks the unchanged1708 compiled inputs, shared helper, four actual probe
sources and clean native gitlinks before the final scoped commit.

## Explicitly not claimed

Full W4/default OCI Host Service cutover,live continuity,fresh-host/full
conformance/long soak and platform promotion remain open. Default macOS remains
BoxVm; HVF recovery stays stopped-only. Windows ARM64 dependencies remain
unsupported with no native run. Runtime RSS is unmeasured. No real ~/.kbox,
native-submodule source edits, unrelated resource cleanup or release publication.
Next technical task: capture and reproduce the intermittent parallel warm-pool
readiness timeout with owned per-box logs retained before cleanup. Git delivery
uses ordinary commits/merge and a non-force main push; final ref equality is
checked after pushing, separately from artifact acceptance.
The final Git delivery receipt is retained locally at
`artifacts/integration-d5cfdcaeba8544c38f9c8f1fae5974fd/delivery-receipt.json`;
it records the resulting local/remote main identities and command exits.
