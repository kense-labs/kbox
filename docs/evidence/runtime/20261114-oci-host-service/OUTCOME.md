# Box / independent macOS OCI Host Service acceptance

Status: **PASS for spec007's explicit bounded profile**, UTC 2026-11-14.
Full OCI Host Service cutover remains incomplete. This is an existing development
host: macOS 26.6.2 (25G83), Apple M5 Max / arm64, rustc 1.97.1. It does not
promote any platform. Base commit 405c8b89f151e9e2879438d9e457dd60bfd18b90 plus
preserved uncommitted A–F and this task's changes. Start/environment/paths:
[run-context.json](run-context.json), checks/initial-commands.json.

## Implementation and actual boundary

The real chain is Box ExecutionManager → persisted OciSdk → public SDK →
independent same-UID macOS Host Service → HVF. MacosHvfOciMigrationConfig and
MacosHvfOciBundleProvider reuse the existing router, DedicatedVm preparation,
platform adapter and SDK v2 external-storage handoff. CLI discovery uses the
explicit HVF endpoint; `off` with that endpoint changes new reservations only.
Default macOS BoxVm, ExecutionManager ownership and generation fencing remain.
No second lifecycle store, workspace, configuration system or dependency added.

OCI preparation uses existing case-sensitive APFS directory transport rather
than BoxVm's direct guest ext4 staging. Config.json is created at 0600. Caller
directory attachments retain Caller/DetachOnly ownership and exact source
identity. Shared Mac host confinement enforces RO after guest remount/fresh
mount, symlink/rename/replacement/truncate/delete/create attacks and fork.
Typed cgroup resources and the admitted guest capability ceiling reach both
init and HVF exec. Unsupported combinations refuse; no fallback or isolation
change. Foreign-UID image refusal is a limitation, not ownership support.

Two shared bugs were fixed through actual failures: active missing-binding
records can no longer discover a current generation, and projection death
between readiness/drain requires exact worker terminal confirmation before
cleanup, preserving unknown Box exit status without forged drain evidence.
Shared process serialization now orders capability sets; v7 Exec fingerprints
are stable across clients. v1–v6 retain their existing algorithms/readability.
Exact bundle digests and ordered argv/environment remain unchanged.

## Final installed products

| Artifact | SHA-256 | Compressed bytes | Installed filesystem bytes |
| --- | --- | ---: | ---: |
| Box 3.4.3 archive | `1f12f1fb296394f151538dd15073500d0383cd935fb966ada1ac143b993622ad` | 23293906 | 44027904 |
| Separate OCI HVF qualification assets 0.3.6 | `17ffecf4643a3ec16da398e6db865c87824b70a2a68cc1ca028c2c829682385b` | 32988952 | 113643520 |

Box archive: `/Users/roylin/code/kense/kbox/artifacts/oci-host-service-ebc8e8fb78904d1b8c7696506ae725a7/final-dist-3/kbox-3.4.3-macos-arm64.tar.gz`. Actual installed prefix: `/Users/roylin/code/kense/kbox/artifacts/oci-host-service-ebc8e8fb78904d1b8c7696506ae725a7/installed-3`.
OCI archive: `/Users/roylin/code/kense/kbox/artifacts/oci-host-service-ebc8e8fb78904d1b8c7696506ae725a7/oci-v7-dist/koci-hvf-0.3.6-macos-arm64.tar.gz`. Actual extracted prefix: `/Users/roylin/code/kense/kbox/artifacts/oci-host-service-ebc8e8fb78904d1b8c7696506ae725a7/oci-v7-unpacked/koci-hvf-0.3.6-macos-arm64`.
These are local dirty-source candidates, not publication/Developer ID releases.
The two archives have different native dependency selections and must retain
those selections. The OCI archive includes raw/compressed system image and the
static Agent; its disk usage is separate from the minimal Box bootstrap package.
Runtime memory **was not measured**; disk/configured guest limits are not RSS.

Actual commands/exits/sizes: checks/package-final-3.json/.log,
checks/install-final-3.json/.log, checks/oci-v7-builds.json,
checks/oci-v7-archive.json and checks/companion-audit-final-3.json (10 companions,
45 successful file/lipo/codesign/otool commands). Host binaries/libraries are
arm64; guest-init and Agent are ELF64 little-endian static AArch64 with no
PT_INTERP. Mach-O strict signatures pass, including Hypervisor entitlements;
signing is ad-hoc. No notarization is claimed.

Box source-input digest: `c24d5a84c2e955c2789f0be1d8b7946f63f8d9b8ad8743145ff42d82ed3f9243`.
The independent path-NUL-bytes-NUL inventory matches installed build-info.json:
checks/source-inputs-final-3.json. OCI input digest: `6c844a37e54811fcdc52241028fb5da54d125a4251d9b61dfd518977a36a68f1`
(build-info.json inside its archive). Cargo.lock is unchanged. Current resolved
host graph has 409 packages; external local source identities are recorded in
checks/dependency-inventory-final.json. checks/source-scope-final-3.json confirms
24 captured existing source/crate-doc files changed and no baseline source was deleted.

Native source gitlinks remain clean: libkrun
3c2889c76460d81e3fb16d6144580fd31eb1bae3; libkrunfw
fc3ea06895301f1df6a274dad75fbd9317b3bfee. Box reused the exact matching clean-source
native build output. Actual Box firmware is the locally compiled pinned
boxlite-ai/libkrunfw v5.3.0 kernel.c bundle, not the root libkrunfw gitlink.
The retained tarball digest is
12b9401d7735d1682450e4d025273c5016ec2237dcbfb76b2f0a152be6e606d6.
checks/native-provenance-final-3.json verifies output/library/tarball identity.
OCI uses its own pinned kmacos-arm64.tar.xz, SHA
5486f38e91eb4da0e58888b543c93fe669c918ad4b84dd495f0d1dfdffc43b56,
with libraries bound by the system-image manifest. No native source or gitlink
was edited. Final manifest SHA: `19960ea11e95125eefc3496fc2939276b1f9647ef2cad0fb5de56534db0c938b`; image SHA:
`cb2f2b577972b51084b6cd6dcd848a2705596150b1a0ae544611378d1d216d77`; Agent SHA: `926aeebde2e5eaca4d533f8db776078409bae3a5b6372504297aed213f3ed69d`.

Final review also fixed normalized ALL precedence: four spellings override guest
SYS_ADMIN additions. The red regression failed on ` all `; all three focused
spec tests now pass. Native15 additionally reads five guest kernel capability
masks with ` cap_all ` and verifies every mask is zero (checks/final3-box-cap-drop-all.json).

The old A–F archive remains in previous-dist and the previous final-archive run,
SHA `1289af08712159ef5dc70c8240b72388af32e1699589c3f798388738fbdceb4c`. Candidate1, its installation, old OCI assets,
failed test clients and all logs remain. Final checksum revalidation:
checks/checksums-after-native-3.json. See [retained failures](FAILURES.md).

## Actual validation

`bash scripts/verify-commit.sh` on final source: **PASS**, 217.09 s.
Runtime **2173 passed / 0 failed / 14 ignored**; CLI **933 / 0 / 0**; every
contract suite passes; installer **7/7**. Workspace all-target check, fmt and
scoped Clippy pass. The ignored native entry is separately executed below;
ignore count is not runtime evidence. Contract logs include nested subprocess
suites and are not misleadingly summed into one count.

OCI owning library suites: Agent **23/0/0**, Runtime **426/0/0** (including the
877-point durable fault registry), SDK **208/0/0**. OCI scoped all-target Clippy
passes. Focused checks include portable configuration, provider/rootfs routing,
v2 resources/mounts, private control mode, missing binding, projection race,
capability-set replay and v1–v6 encoding compatibility. Actual count lines and
commands are indexed by checks/test-result-inventory-3.json.

Final **archive-installed** Box/HVF matrix: attempt15, **1/0/0**, 55.34 s.
It uses the final Box archive's CLI, shim, guest-init/libraries and the extracted
OCI archive's fixed Host/shim/native bundle/manifest/image. The test client is
frozen from the same source, SHA `41579238365bf7d980775bbd4e762bfad8be058ac727e5694e335342948e94b1`. Private K_HOME,
normalized loader variables, system tools only; no user install or developer
library is used. Actual loader maps: checks/loader-attempt-15.json and raw logs.

Verified: mixed real BoxVm/OciSdk, create replay, reconstructed managers and
new-create policy changes; installed CLI old-route inspect/exec plus new OCI
create/exec/remove; exact driver/isolation/binding; four malformed binding cases
and stale generation refusal; stdout/stderr/exit23 and streaming stdin/exit7;
file roundtrip, refused RO upload and ungranted symlink, Unicode directory API;
actual cgroup readback and PID update; 14 actual host write denials + child after
writable remount and fresh virtiofs mount, repeated after restart with writable
controls; committed Exec response deliberately lost at the public byte stream,
client disconnect, keyed replay and first concurrent duplicate from independent
clients with one real host side effect; Host SIGKILL, unchanged live BoxVm PID/
start identity, authenticated exact stopped/SIGKILL SDK evidence, old input
failure/no late write, explicit Box/SDK generation1→2 restart. Terminal Box
reconciliation releases the runtime binding while keeping identity/route history.
Evidence copies: checks/final3-box-*.json; raw root: `/private/tmp/kbox-hvf-ebc8e8fb7890/attempt-15`.

The independent final-source Host Service matrix also passes **23 operations,
25/25 waves and authenticated owner-death recovery**, 26.81 s:
checks/host-service-v7-matrix.*. It complements rather than substitutes for the
Box native matrix. HVF recovery is stopped-only, not live-session continuity.

Final archive A–F regression results (checks/archive-3/native-results.json):

| Scenario | Actual count | Elapsed |
| --- | --- | --- |
| oci | 1 completed scenario | 54.64 s |
| readonly | 1 completed scenario | 6.91 s |
| primary | 1 completed scenario | 69.10 s |
| prewarm | 1/0/0 | 17.54 s |
| directory | 1/0/0 | 17.27 s |

OCI-image/engine lifecycle includes export/commit/rootfs snapshot/restore,
engine restart/identity and stale-intent refusal. RO uses remount/fresh-mount,
14 EPERM operations plus child, repeated after restart. Primary covers process
streams, timeout/cancel/no late writes and shutdown; prewarm and directory tests
check exact native test counts, grants, stale generations and cleanup. Every
case has a result.json and complete installed native loader proof.

## Cleanup and remaining gates

Final inventory: **30 CLI execution inventories empty**, volume catalogs empty,
no owned VM/shim/engine/Host processes, mounted images or live sockets. Known
outside-root sockets observed in owned worker commands are absent. All seven
preexisting RAM devices retain the same device/mount identities; none was
removed. Aborted attempt10 was reconciled, killed and removed through the
canonical manager (maintenance cleanup10-2, exactly1/0/0). Artifact files remain.
Raw results: checks/cleanup-inventory-final-3.json, final3-images.plist,
final3-processes.log and final3-mounts.log, including the unchanged RAM identities. Both installed
manifests and all preserved archive checksums still match after native tests.
The first inventory checker treated the `{"volumes":{}}` catalogue envelope
as an entry; the retained schema-first record and final typed entry counts
show zero actual volumes. No resource or assertion was removed to obtain this.

[W4-GATES.md](W4-GATES.md) lists each open cutover gate and the next executable
local task: guest-native ownership-preserving rootfs transport for foreign-UID
images. Fresh KVM/WHPX/macOS release qualification, full feature parity, OCI
conformance/security review, rollback and longer soak remain outside this
bounded acceptance. Those limitations do not block this completed explicit path.

Final reference review also found two preexisting ROADMAP evidence targets
missing from both this checkout and its available Git history:
`crates/oci-runtime/docs/evidence/2026-10-18-guest-command-status/OUTCOME.md` and
`docs/evidence/code-harness/20261018-macos-guest-owner/OUTCOME.md`. Their old
links were retained; no historical evidence was invented. They need the original
archive/source if those historical claims are to be audited. This task's new
references resolve. The related Runtime README E-evidence relative path was
corrected. See checks/final-review-3.json for the exact reference audit.

## Explicitly not claimed

Full OCI Host Service/default W4 switch; platform readiness promotion; fresh-host
attestation; other-platform native results; Windows ARM64 support (current
dependencies do not support it, no native run); live-session continuation after
HVF Host death; unsupported profile combinations; old Host rollback on v7 state;
Developer ID/notarization; runtime memory measurement. No real ~/.kbox, native
submodule source edit, new dependency/workspace, commit, push or publication.
