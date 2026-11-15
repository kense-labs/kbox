# Final A–F macOS arm64 archive acceptance

Status: **PASS**, 2026-11-14. Host: macOS 26.6.2 (25G83), Apple M5 Max / arm64;
rustc 1.97.1. This is acceptance on the existing development host with isolated
runtime prefixes, not fresh-host or other-platform qualification.
Base commit: 405c8b89f151e9e2879438d9e457dd60bfd18b90 plus retained uncommitted A–F.
Initial commands/environment/resources: `checks/initial-commands.json` and logs.
Run identity/paths: `run-context.json`. No product behavior changed in this round;
only qualification artifacts, bounded spec and release/evidence documentation.

## Final product

- Frozen archive:
  `artifacts/final-archive-cf1a062d97174e699fd16a7cdf6a981c/final-dist/kbox-3.4.3-macos-arm64.tar.gz`.
- Generated current dist: `dist/kbox-3.4.3-macos-arm64.tar.gz`, identical bytes.
- SHA-256: `1289af08712159ef5dc70c8240b72388af32e1699589c3f798388738fbdceb4c`.
  The adjacent `.tar.gz.sha256` is retained with both copies.
- Compressed size: **22496416 bytes (21.45 MiB)**.
- Installed filesystem usage: **42528768 bytes (40.56 MiB)**, from `du -sk`
  multiplied by 1024. Logical regular-file bytes excluding symlink aliases:
  42504771. These are package/disk measurements, not memory measurements.
- Runtime memory: **not measured**; no peak RSS, total resident memory or
  memory-efficiency claim is made.
- Actual isolated installation:
  `artifacts/final-archive-cf1a062d97174e699fd16a7cdf6a981c/installed`.

Previous archive, checksum and staging tree remain in this run's `previous-dist`.
Its old SHA-256 `8a405ba0ddd4d2653b2e3caa85918830949034dd143d4d869ba78e8908fb8005`
was verified before replacement. `checks/previous-dist.json` hashes its contents.
This is a local dirty-source candidate at the existing version, not a publication.

## Build, source and native identities

Actual build: `K_BUILD_LIBKRUN=1 bash scripts/package-macos-arm64.sh`, PASS,
128.37 s. `checks/package-attempt-1.json` records environment normalization,
command, UTC start and exit; full output is `package-attempt-1.log`.
The script invokes the locked Linux/aarch64 release-dist zigbuild and host
CLI/shim release-dist build, stages that build's libraries, signs the shim,
verifies the staging installer and produces the archive.

Root source input SHA-256:
`2e6124f1fd17c4b11991893c5a899477f5d681c4e19d9fe2f94a2296cecc9839`.
The independently recalculated path-NUL-bytes-NUL digest matches the archive's
build-info.json. `checks/source-inputs.json` gives the exact selection/each file
hash; `checks/build-info.json` records Cargo.lock, compiler and targets.
The hash covers the builder's kbox input scope; external local dependency source
is recorded separately below, not silently included in this root-only digest.

`checks/dependency-inventory.json` retains 409 resolved Cargo package identities
(host-filtered graph, not a per-binary feature attestation), including versions:
CLI 3.4.3, shim/guest-init/Runtime/libksys 3.2.7, koci-kbox 0.3.6 and SDK 0.3.2.
External local `kense-kcl` is clean at
76bf580454f1d69d7b826223ac50161efa685f85; its 32-file package source digest is
`a0988eb55974399f34de357a394be58075cdc94ad6276631d46159fde6c8c84b`.
Those files predate this build. No external source was edited.

Native source identities remain unchanged and clean:

- libkrun: 3c2889c76460d81e3fb16d6144580fd31eb1bae3, built from the root submodule.
- libkrunfw root gitlink: fc3ea06895301f1df6a274dad75fbd9317b3bfee, recorded but
  **not the firmware source consumed by this macOS build**.
- Actual firmware: locally compiled from the pinned
  `boxlite-ai/libkrunfw` v5.3.0 `libkrunfw-prebuilt-aarch64.tgz` kernel.c bundle,
  SHA-256 `12b9401d7735d1682450e4d025273c5016ec2237dcbfb76b2f0a152be6e606d6`.
  The tarball hash and extracted kernel.c hash were verified; the tarball is
  frozen under this run's `source-inputs`. No ambient firmware override or system
  libkrun was selected. `checks/native-provenance.json` and retained native build
  output identify the single build whose library bytes match this installation.

`checks/companion-audit.json` records full companion hashes, file sizes and actual
file/lipo/codesign/otool commands/logs. Host CLI/shim and both dylibs are arm64,
minimum macOS 11.0. Guest-init is ELF64 little-endian Linux/aarch64, machine
0x00b7, with no PT_INTERP. Every Mach-O strict signature passes; shim has
com.apple.security.hypervisor=true. The shim's only rpath is
`@executable_path/../lib`. Signing is ad-hoc, not Developer ID/notarization.

The exact frozen archive was extracted and installed through its own install.sh
to a previously absent prefix. All installed SHA256SUMS entries match, before
and after native acceptance. Commands/exits: `checks/install.json`, raw log:
`checks/install.log`; sizes/ELF details: `checks/sizes-and-guest.json`.

## Actual installed-archive native checks

All drivers run outside the developer cwd, with the archive prefix and system
tools on PATH, fresh K_HOME values, and normalized native loader variables.
The writable image store is a copy of our prior owned fixture cache, not a user
cache. Every VM companion comes from the actual archive installation (some
drivers copy its shim/guest-init into their private home and symlink its libraries).
Generic guest image is pinned:
`docker.io/library/kbox-qualification-tools@sha256:74dc9800a03c96762c1061646bd5d9100b1a8361c0945fc9d9de2c271ce2b3f7`.

| Scenario | Actual count / result | Elapsed |
| --- | --- | --- |
| `native_smoke.py` OCI-image/engine lifecycle | 1 complete PASS scenario; build/run/exec, mounts/HTTP, export, commit, rootfs snapshot/restore, engine restart/identity, stale-intent refusal, native guest policy equality | 34.94 s |
| `native_readonly.py` | 1 complete PASS scenario; privileged writable remount and fresh mount succeed, then **14 EPERM operations + guest child**, repeated after restart; live host updates, same record/BoxVm route, writable controls and overlap refusal | 4.35 s |
| `native_primary.py` | 1 complete PASS scenario; binary I/O, 2 MiB stdin, backpressure/concurrent exec, EOF/tails, exit 37, disconnect/engine death/no replay and late-write observations | 66.29 s |
| Native prewarm test | **1 passed / 0 failed / 0 ignored**; same original process, generation advance/old fence, reconstruction, unclaimed refusal, 2 MiB transfer/child, dirty retirement | 15.15 s |
| Native directory test | **1 passed / 0 failed / 0 ignored**; actual RO remount, directory grants/live view/canary, different execution identity, old-generation refusal, moved-source cleanup | 16.79 s |

Commands, exits, exact result paths and checked counts: `checks/native-results.json`.
Behavioral flags/denial errnos are independently checked in
`checks/native-acceptance-summary.json`; copied PASS reports are `*-result.json`.
Raw commands, stdout/stderr, failure/cleanup files and workload outputs remain in
the named private artifact roots. No zero-test exit is counted as acceptance.

The prewarm/directory client is the freshly resolved same-source Runtime test
executable, frozen and hashed in `checks/native-client-identity.json`; it is a
test driver, not a shipped CLI feature. Its dynamic dependencies are only system
libraries. It launches the archive's actual shim/guest/native companions.
Nineteen actual worker loader samples (OCI 7, RO 2, primary 5, prewarm 2, directory
3) identify the loaded shim and libkrun/libkrunfw paths and hashes. All sampled
native libraries resolve into the installed prefix; no sampled library is taken
from target/, Homebrew or a user install. See `checks/native-loaded-libraries.json`
and `loader-*.log`. These timings are test durations, not cold-start benchmarks.

## Verification and cleanup

`bash scripts/verify-commit.sh`: **PASS**, exit 0, **231.88 s**, with an explicitly
private verification K_HOME. `checks/verification-entry.json` and full log retain:

- Runtime **2165 pass / 0 fail / 12 ignored**; CLI **933 pass / 0 fail**.
- All default contract unit/integration/doc tests, including >10000 exact replays.
- Workspace all-targets check, Rust formatting and scoped Clippy. Existing
  warnings remain visible; no assertion, test or lint gate was weakened.
- Support matrix **4 targets / 0 supported / 4 rejected / 1 required**, unchanged.
- Archive installer **7/7**.

Final actual CLI inventory queries cover all seven private homes/prefixes:
**zero managed executions and volumes**. No owned VM/shim/engine/client process,
mount or attached disk image remains. The 18 distinct sampled Box identities
have no nonempty socket directory. Seven earlier foreign RAM devices are
unchanged. No final audit action killed a process, detached a disk or deleted
data; the native drivers and default-test fixtures performed their own cleanup.
`checks/final-resource-inventory.json`, raw cleanup commands and final mount log
retain the evidence. Frozen archive, source tarball, logs and fixture outputs stay.

## Failure retained

The first evidence extraction attempt used tarfile.extractall(filter="data") on
the system Python 3.9, which does not accept that keyword. It stopped before any
installer/native test invocation. The observed TypeError, Python identity and
correction are retained in `checks/unpack-attempt-1.failure.json`. We validated
member/link paths and used the available extraction API, then installed normally.
No product source fix was needed. Build, installer, all five native scenarios
and full verification pass; no failed native result was discarded.

## Continuation

[A–D](../20261114-release-baseline/OUTCOME.md),
[E](../20261114-macos-readonly/OUTCOME.md) and
[F](../20261114-windows-arm64/OUTCOME.md) remain historical phase records.
This round unifies their current source in the final installed macOS archive.
No locally actionable acceptance item remains. The next bounded development
task is native OCI Host Service path qualification, with retained OciSdk/BoxVm
identity, recovery and generation tests before any default route cutover.

## Explicitly not claimed

Full OCI Host Service cutover remains **incomplete**. These OCI results cover
the Box-owned OCI-image/engine path, not a standalone Host Service refresh.
Windows ARM64 remains **unsupported by current dependencies**, with **no native
run**; no feasibility research, Windows host wait or VMM/firmware port was started.
No other-platform or fresh-host native qualification, supported-readiness
promotion, long soak, broad performance, measured runtime memory, Developer ID,
notarization, crash-safe hot installer upgrade or new StateSnapshot claim.
Read-only unsupported combinations remain explicitly refused as documented in E.
No real `~/.kbox` mutation, native source/gitlink edit, commit, push or publication.
