# macOS host-enforced read-only boundary — E

Date: 2026-11-14. Status: bounded E implemented and verified on this host.
Host: macOS 26.6.2 (25G83), arm64, Apple M5 Max; rustc 1.97.1.
Base: 405c8b89f151e9e2879438d9e457dd60bfd18b90 plus retained uncommitted A–D.
No commit, publication, native submodule edit or real `~/.kbox` mutation.

## Result

The actual HVF workload successfully remounted the export writable and created
a fresh writable mount of the same virtio-fs device. Fourteen mutation/alias
operations and a guest subprocess still received host EPERM, before and after
restart. Source identity/content/permissions, live owner writes, writable
controls and writable hard links were preserved. Unsafe overlap now refuses
before provider launch with the actual read-only reason. All owned execution
inventories are empty.

`scripts/verify-commit.sh` passed: Runtime **2165 pass / 0 fail / 12 ignored**,
CLI **933 pass / 0 fail**, all default contract tests, workspace all-targets,
format, scoped Clippy and installer **7/7**. Shared OCI platform tests **69/69**
(two child entry points invoked by parents), Box shim **30/30**, and their
all-targets Clippy also pass. Native OCI-image/engine, primary-stream, prewarm
and Runtime directory/generation regressions pass with the frozen companions.

## Confirmed implementation and boundary

The previous macOS `:ro` preflight refusal is retained in
`checks/baseline-readonly-refusal.log`. The Box path carried `FsMount.read_only`
only into guest mount flags. OCI already used a host Seatbelt worker policy.
Both now share `MacosReadOnlyMounts` and the same Seatbelt installer/rule builder.
Box pins read-only directories, supplies fdesc roots to libkrun, confines before
libkrun context/thread creation, and revalidates sources at actual VM entry.
The OCI worker retains its existing stricter deny-default policy and pinned
asset/control semantics. No record route, provider identity or generation is
rewritten. Runtime preflight uses the same export validator before provider launch.

Read-only paths and ancestors cannot be moved by the confined process. Kernel
write checks cover create/data/metadata/unlink/rename; links involving protected
sources are denied. Writable source directories and their ordinary hard links
remain writable. Host owner updates stay live. The policy is process scoped;
it does not chmod caller data or claim to constrain the authorized host owner.

Unsupported boundaries refuse: read-only/writable overlap, root replacement,
pre-existing hard-linked files in a read-only tree, more than 100000 entries,
unidentified/pre-opened writable source descriptors, ordinary read-only
single-file binds, and read-only exports with snapshot boot. These refusals are
safety checks, not the evidence that working read-only exports are enforced.

## Tests and failures retained

- `cargo test --locked -p koci-kbox --lib`: 69 pass, 0 fail, 2 child-only ignored
  entries, both invoked by their parent tests. `checks/shared-platform-tests.log`.
- Shared native boundary: actual denied writes via paths and directory handles,
  source/ancestor replacement, inward symbolic links, hard links, threads and
  host child execution; positive writable control. No VM required for this test.
- Shim conversion: 1 pass; Runtime admission/staging filters: pass; CLI `create`
  integration: 1 pass with preserved `:ro`, explicit BoxVm route and cleanup.
- `cargo test --locked -p kbox-runtime macos_`: 21 pass, 0 fail. The existing
  RAM-root fixture exposed an ignored detach failure; the repair and real
  cleanup verification are recorded below.
- First boundary test found macOS fdesc paths cannot be traversed as ordinary
  `/dev/fd/N/child` paths. Traversal now reuses the installed cap-std adapter.
- The next native test proved Seatbelt does not revoke a file descriptor opened
  writable before confinement. Admission now enumerates and refuses such source
  descriptors; the Box policy is installed before libkrun can open workload files.
  Failed assertions remain in `checks/shared-boundary-*.log`.
- A native link-policy spike proves a source-specific `file-link` denial blocks
  protected aliases while preserving writable-to-writable hard links.

## First HVF run (failed diagnostic, retained)

Frozen companions: `checks/frozen-companions.json`; exact command:
`checks/native-readonly-command.json`. Full isolated results:
`artifacts/macos-readonly-40df3a5afe4e4326ad3bdade292bbe71/native-readonly/2cf679dffc7140edb0bd979e9c02d7e9`.
Pinned guest image:
`docker.io/library/kbox-qualification-tools@sha256:74dc9800a03c96762c1061646bd5d9100b1a8361c0945fc9d9de2c271ce2b3f7`.

The guest successfully remounted the read-only virtio-fs writable and mounted
`vol0` again writable. Fourteen mutation/link/alias operations and a guest child
each returned host EPERM (1). Writable controls and guest children worked;
host writes were visible live. Restart preserved the record ID and BoxVm route
and repeated every host denial. Raw probes are `003.stdout` and `007.stdout`.

The overall first run FAILED its overlap diagnostic assertion: the shim refused,
but the CLI later surfaced a missing guest diff-baseline error. This is not
counted as a passing native gate. Runtime now performs the same export validation
before provider launch; a focused typed-refusal test passes. The native harness
now copies per-box logs before removal. The first run's raw CLI/probe output and
zero-resource cleanup remain; its removed per-box shim files were not retained.
`checks/native-readonly-first.log` records the failed assertion.

## Final native qualification and cleanup

Fixed binaries, companion hashes and changed-source hashes:
`checks/qualified-companions.json`. Exact command:
`checks/qualified-native-command.json`. Actual qualified run:
`artifacts/macos-readonly-qualified-92aff37c9486411a905f7b113bdd1655/native-readonly/7fa92f86f25348f38fb5e0be6ef8eda7`.
The new binaries use the unchanged Linux/aarch64 guest init and unchanged native
submodule revisions; their exact bytes are retained in the frozen prefix.
This is a dirty-source diagnostic qualification, not a newly released archive.

| Command/scenario | Actual result |
| --- | --- |
| `native_readonly.py` exact command above | PASS, 16.57 s; 14 EPERM operations + child, repeated after restart |
| Runtime `directory::native::native_directory_grants_preserve_live_files_fences_and_cleanup` | PASS, 1 test executed, 15.53 s; privileged RO remount, RW live view, stale generation refusal, moved-source retirement |
| `native_smoke.py` with frozen prefix and pinned generic image | PASS, 132.16 s; OCI image/build, CLI lifecycle, engine and security defaults |
| `native_primary.py` with the same prefix/image | PASS, 79.96 s; streams, process identity, disconnect/cancel and late-write checks |
| `native_prewarm.py` with frozen same-source Runtime client | PASS, 21.70 s; reuse, fencing, transfer, dirty retirement and cleanup |
| `bash scripts/verify-commit.sh` | PASS, 259.45 s; `checks/verification-entry.log` |
| Shared platform/shim full tests and Clippy | PASS; `shared-platform-final.log`, `shim-all-final.log`, `shared-clippy.log` |
| Support matrix and final whitespace | PASS; readiness unchanged, 0 supported targets |

Commands, exit codes, raw logs and copied result/cleanup reports are under
`checks/`; full raw native files remain under the named private artifact root.
The first Runtime directory harness invocation used an incorrect exact filter
and ran zero tests; its original command/exit files remain and are not counted.
The qualified invocation requires one executed test and a PASS result artifact.

The existing RAM-root test silently leaked manually mounted HFS volumes because
Disk Arbitration did not see their actual mounts. Replacing its swallowed
cleanup statuses with assertions exposed the failure. The correct fixture
cleanup uses `/sbin/umount <exact-owned-mount>` then `hdiutil detach -force
<exact-owned-device>` and asserts both. The newly created disk12/disk13 were
removed; the repaired focused test and full default gate both eject their own
device. Seven earlier RAM fixtures were inventoried and left untouched.
Failed eject diagnostics and successful direct cleanup are retained in
`owned-ram-*.json`, `ram-*-regression.log` and the RAM inventory plists.

`checks/owned-resource-inventory.json` records empty private execution homes,
no owned shim/VM/engine process and the untouched earlier RAM devices. Source
directories and fixture output files deliberately remain as evidence.

## Explicitly not claimed

No Linux/Windows native result, supported-readiness promotion, full OCI service
cutover, automatic record migration, native single-file read-only support or
complete Secret workload qualification. Source aliases deliberately introduced
by the authorized host owner after admission are outside the workload boundary.
No standalone OCI Host Service end-to-end refresh is claimed here: the shared
OCI worker's native confinement tests and the Box-owned HVF path were exercised.
