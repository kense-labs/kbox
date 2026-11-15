# Generic MicroVM release baseline — A through D

Date: 2026-11-14. Host: macOS-26.6.2-arm64-arm-64bit. Toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14).
Base commit: 405c8b89f151e9e2879438d9e457dd60bfd18b90, with the pre-existing
uncommitted generic-engine cleanup retained. No commits or publication occurred.

## Result

The default Runtime baseline changed from **2144 passed / 15 failed / 12 ignored**
to **2163 passed / 0 failed / 12 ignored**. CLI has **933 passed / 0 failed**.
The complete default contract test command, workspace all-targets check, scoped
Clippy, full Rust formatting, support matrix and archive installer tests pass.
The final strengthened verification entry point ends with `verify-commit: OK`.

The exact final archive installed into an isolated prefix, then passed native
OCI, primary-stream and clean-environment prewarm checks on this macOS HVF host.
Seven private test homes have zero managed executions. Retained process inspection
found no owned VM/shim/engine leftovers. This is existing-host evidence, not a
fresh-host readiness promotion.

## Behavior changes and failure classification

- Resource compilation now preserves explicit CPU shares/quota/period, cpuset,
  memory reservation/swap, exact byte limits and PID limits. It reuses existing
  update validation and rejects numeric overflow and invalid cross-field values.
  Partial OCI updates still emit only selected fields.
- Enabling unsupported MicroVM StateSnapshot now returns typed
  `UnsupportedCapabilities` at driver construction before state creation. Default
  discovery does not publish StateSnapshot. Generic contract types are retained.
- Snapshot abort recovery accepts an already-running validated provider without
  replaying thaw. Existing in-flight state is reconciled without claiming a
  published snapshot. Unsupported new captures refuse before freezer effects.
- Cached image metadata can produce a pure anonymous-volume ownership plan.
  Runtime-owned OCI preparation refuses anonymous-plan drift before creating
  image-declared volumes.
- Stale tests that treated identical MicroVM requests as two isolation backends
  now exercise explicit creation policies, retained record routes, guest-vs-host
  process identity, creation-policy tampering and rejection without fallback.
  Adversarial ownership/generation checks were preserved.

## Packaging and installation

`scripts/install-package.sh` is source-owned. `dist/` holds generated outputs.
The package builder compiles the static Linux/aarch64 guest-init with cargo-zigbuild
and the macOS CLI/shim from this checkout. Native libraries come only from the
same build's release staging directory. No installed runtime is borrowed.
The Hypervisor-entitled shim is ad-hoc signed before hashes are recorded.

The archive retains VERSION, build-info.json and SHA256SUMS. The installer checks
all required companions, aliases, checksums, guest/host architectures and signature,
then stages bin/lib while preserving workload data and unrelated runtime files.
Normal errors/signals restore previous runtime contents. Failed rollback retains
backups. Root aliases, corrupt checksums, missing companions, wrong guest CPU,
unexpected symlinks, failed verification and failed renames have regression cases.

Support-matrix references now point to source-owned release documentation. Its
guest-architecture contract references runtime manifests rather than a client.
The prior `dist/` source-file deletions were retained.

## Actual checks

| Command / scenario | Actual result |
| --- | --- |
| Initial `cargo test --locked -p kbox-runtime --lib` | 2144 pass, 15 fail, 12 ignored |
| Final `cargo test --locked -p kbox-runtime --lib` | 2163 pass, 0 fail, 12 ignored |
| Final `cargo test --locked -p kbox-cli --lib` | 933 pass, 0 fail |
| Final `cargo test --locked -p kbox-contract` | All default unit/integration/doc tests pass |
| `cargo check --workspace --all-targets --locked` | Pass, warnings retained |
| `cargo clippy --locked -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps` | Pass, warnings retained |
| `cargo fmt --all -- --check` | Pass; formatting-only paths are recorded separately |
| `python3 scripts/check-support-matrix.py` | Pass: 4 targets, 0 supported, 1 required future target |
| `python3 scripts/test-install-package.py` | 7 pass on macOS arm64; synthetic Mach-O/ELF fixtures, no VM |
| `bash scripts/verify-commit.sh` final rerun | Pass; full transcript in checks/verification-entry-final.log |
| `bash scripts/package-macos-arm64.sh` final build | Pass, built and installed its exact staged companions |
| Final `native_smoke.py` | Pass in 77.05 s |
| Final `native_primary.py` | Pass in 77.33 s |
| Final `native_prewarm.py` | Pass in 30.95 s |
| Real final-binary v1 journal/intent refusal | Pass; old journal bytes unchanged, no request accepted |
| Final inventory | 7 private homes empty, no owned runtime processes |
| Current-source product-coupling scan | 0 matches |

Native command arrays, script hashes, source paths, times and copied results are
in checks/native-results-final.json. Complete raw native logs remain under the
private artifacts paths named there; no failed run was deleted. Native scenarios
used a newly built generic Python/Node/bubblewrap image, pinned by its OCI digest:
`docker.io/library/kbox-qualification-tools@sha256:74dc9800a03c96762c1061646bd5d9100b1a8361c0945fc9d9de2c271ce2b3f7`. No application-specific base or external client is required.

OCI acceptance covers pull/build/run/exec, Unicode directories, named volumes,
HTTP, export excluding mounts, commit/rerun, CLI snapshot/restore, engine restart,
stale-intent refusal and direct guest-security baseline equality. Primary-stream
acceptance preserves binary bytes, backpressure, concurrent exec, EOF, exact exit,
engine-crash/disconnect refusal and post-cancel late-write observation.
Prewarm acceptance proves same original host process, atomic generation advance,
old-generation refusal, reconstructed-driver reuse, unclaimed-exec refusal,
2 MiB external file transfer, child process creation, and dirty-environment retirement.
The prewarm client is the same-source library test binary and uses the archive's
actual shim/guest/native companions; it is not a CLI scheduler performance claim.

## Final artifact and separate resource measurements

- Archive: `dist/kbox-3.4.3-macos-arm64.tar.gz`
- SHA-256: `8a405ba0ddd4d2653b2e3caa85918830949034dd143d4d869ba78e8908fb8005`
- Compressed bytes: 22475371 (21.43 MiB).
- Installed filesystem usage: 42512384 bytes (40.54 MiB).
- Runtime memory: Not measured; native scenarios configured 512 MiB guest memory. No peak RSS claim.
- Installed qualification prefix: `/Users/roylin/code/kense/kbox/artifacts/release-baseline-cb6c4b1f6bac450ca5519f26a754ce8d/installed-final`.
- Source/native/companion identities: checks/final-build-info.json and checks/final-companions.json.

## Failed attempts retained

The first added constructor-refusal test used unwrap_err on a non-Debug driver;
its diagnostic remains in runtime-after-a1.log. After correcting that test, one
remaining snapshot regression exposed duplicate thaw; runtime-after-a2.log retains
it and runtime-after-a3.log shows the full passing fix.
Installer fixture runs initially found lipo argument ordering and an incomplete
synthetic ELF program-header fixture. Those logs remain. The final seven cases
also cover root aliases and both pre-rename and partial-install rollback failures.
The first prewarm native attempt failed because the selected Python/bubblewrap
fixture lacked Node. Its VM reuse/generation work was not counted as a pass;
its complete failure and successful cleanup remain in native-results.json.
A new generic tools image supplies Node; all final native checks were rerun against
the final archive and pass. The initial archive is retained in the private
artifacts directory with its original checksum.

## Explicitly not claimed

- No Linux KVM, Windows WHPX or Windows ARM64 native qualification.
- No supported-readiness promotion, Developer ID signing, notarization or publication.
- No automatic/rolling v1 journal or live-workload migration. Typed refusal,
  original-state preservation and documented fresh-home use are verified.
- No runtime memory peak, broad performance result or long-duration soak claim.
- No SIGKILL/power-loss transactional installer recovery; staged backups survive,
  but crash-safe hot upgrades need durable recovery work.
- No MicroVM StateSnapshot implementation, memory capture or retired Sandbox lane qualification.
- No macOS host-enforced read-only feature or full unified-backend cutover (milestone E).
- No Windows ARM64 feasibility result or backend implementation (milestone F).
- No remote CI run; the local reusable full verification entry point is verified.
