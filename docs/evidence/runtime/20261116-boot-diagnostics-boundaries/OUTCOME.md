# Outcome033: bounded, confined failed-boot diagnostics

The bounded correction is accepted on this existing macOS26.6.2 arm64 host.
The enterprise-GA goal remains active. Original intermittent15s startup cause,
stop-failure ownership, full W4 and default OCI Host Service cutover remain open.

## Actual defect, correction and red/green

The old vm/boot_diag.rs recursively copied arbitrary logs, followed source file/
directory aliases and allowed destination-parent aliases to redirect writes.
Its advertised bounded snapshot had no byte, entry or depth limits. The shared
retention entry now validates the exact home/boxes/id scope and pins each child
with the workspace's existing cap-std/cap-fs-ext no-follow adapters. It copies
only flat regular single-link files, reading four known console/shim names first,
at most64 general entries plus one bound witness, at most64 files,1MiB per-file
tails and8MiB total. FIFO/links/nested inputs are skipped or refused. The private
retention.json records actual copied bytes/files, observed skips/truncations and
limits; it does not claim a complete tree at a limit. New Unix directories/files
are0700/0600. Prior unsafe nested/oversized target slots are preserved/refused,
never recursively erased. Ordinary flat retry/latest-slot behavior is retained.

The cleanup caller awaits spawn_blocking before deleting its box directory;
retention failure is logged without replacing the original boot error or bypassing
teardown. Direct cap dependencies reuse root-pinned4.0.2: byte-level lock proof
shows exactly two runtime dependency names added, zero new packages/versions.
No new resource state, configuration, protocol, unsafe block or native-source edit.

Actual original red:12 tests,3pass/9fail. Old three tests moved to the sibling
without weakening their assertions. Final focused20pass/0fail/1ignored covers
source/destination aliases, file and directory replacement after pinning,
hardlinks, FIFO, byte/entry bounds, priority diagnostics, UTF-8/space/empty paths,
private modes, scope escape, retry, prior-slot preservation and report ownership.
Final review caught report-temp error cleanup removing an existing collision;
actual shared-code candidate2 behavior runs0pass/1fail, corrected20pass. It now
creates the temporary successfully before assuming cleanup ownership. All reds,
compile-import errors, first native fixture failure and candidates are retained.

## Actual checks and remaining full-gate failures

- focused-final4:20pass/0fail/1ignored; earlier broader VM231/0/1.
- verify-final3:exit101; workspace all-target compile, support matrix and fmt pass;
  Runtime2299pass/6fail/50ignored. Same2 RAM/Secret and4 APFS/provider failures
  remain in the log. No default assertion, ignore or acceptance threshold changed.
- Final Runtime all-target Clippy passes. CLI935/0/0, full contracts118 passes
  across all actual suites (24 library cases), installer8 pass separately because
  verify stops at Runtime. Their unchanged suites were not rerun after the local
  report-only correction; final archive self-install and checksum checks did run.
- portable-launch4 compiles the exact final diagnostic module as Windows x64
  metadata using actual locked dependencies. This is compilation only, not a full
  Runtime build/native run. Earlier ARM64 metadata harness attempts were incomplete
  (ambiguous cached artifacts/missing UUID metadata; outside-workspace feature
  selection refused), retained without claiming an ARM64 module compile.

## Final source-bound installation and dependencies

Frozen inputs `9e7b9442c02c43efbbff79f7e79caee4f0b0c318f3af41e354d1ae8ebdec530c`, 1719 files; build base
`151709fa192d8dc6fd46d27d9923aa99bc3b9713` with scoped dirty source. Final archive:
`/Users/roylin/code/kense/kbox/artifacts/boot-diagnostics-d9408c1fc6d840b2aaea332dfd213f96/final-dist-bootdiag-3/kbox-3.4.3-macos-arm64.tar.gz`.
SHA256 `5b59a54662f4b9c1303caf070ee336adac62d259ada02cd7ae60dd5c725b7cf2`; 23410852 compressed bytes /
44191744 installed disk bytes. Fresh prefix `/Users/roylin/code/kense/kbox/artifacts/boot-diagnostics-d9408c1fc6d840b2aaea332dfd213f96/installed-bootdiag-final-3`.
CLI/shim/guest-init/native companions, all11 manifest files, architecture,
signatures/dynamic paths, source/lock and installed provenance checks pass with
zero failed identity commands. Native owner client SHA256 `75e3992f7c2e074853e95fb76e4fe612f7135c241d3de0ed82f7443e9f10b4bb`.

The diagnostic fault test uses the frozen source-bound owner client and actual
archive-installed shim/guest-init/libs. A–F uses the archive-installed CLI. It does
not claim the CLI exposes a diagnostic fault injector. No development runtime
payload or real user installation is borrowed. OCI payload is unchanged/reused
from spec031 with original source d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871,
archive d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79.
Clean native gitlinks3c2889c…/fc3ea068… and separately qualified native bundle
provenance remain; no parent edits inside native submodules. Earlier Box archives
fb8ca51f…/e44e1b8c…/92b68cf8… and all checksum files remain. Runtime RSS unmeasured.

## Native effects, failures and exact cleanup

Final exact diagnostic test runs1pass/0fail/0ignored: a real authenticated Guest
uses existing shell/coreutils to plant two links into its explicit RW log grant,
pointing to an ungranted Host file/directory, and writes a2MiB+marker flood.
The real failed-boot cleanup path stops the VM, skips both aliases, retains the
last1048576 bytes, preserves the outside canary, and releases handler/box/socket.
Actual PID, configuration, loaded installed dylib hashes, live logs and report
are retained. First attempt used absent /usr/bin/python3 (this image's Python
is /usr/local/bin), ran0/1/0 and cleaned its VM; assertions were preserved.

Final A–F4 has5PASS rows:3 Python scenarios (OCI lifecycle, Host RO attacks plus
restart, primary streams) and2 exact Rust1/0/0 tests (prewarm and directory/
identity/fencing). Current final totals:3 Rust native tests +3 Python scenarios.
Native candidate2/AF3 also pass but are not added to these final totals. The
incorrect initial A–F2 context inherited an obsolete uncached ownership image:
all5 rows fail, including2 actual Rust0/1/0 runs. Registry refusal and exact owned
primary-driver interrupt are retained; actual accepted qualification-tools digest
was restored, rather than changing the assertion or importing a different asset.

The first total audit correctly failed on an empty persistent test volume from
that interrupted invalid-image intent. Identity/intent labels, an empty owned
mountpoint and no FD holders were checked before installed CLI removed that one
exact volume. This is recorded in invalid-fixture-volume-cleanup.json; no caller
or user data removed. The audit itself deletes nothing.

Final audit: 16 actual homes (including every failed candidate), zero
managed executions/volumes, owned VM/shim/engine processes, mounts, disk images,
FD holders or sockets. 38 observed owned socket
directories and 157 exact socket paths are absent.
646 original immutable cached files retain
bytes/UID/GID/mode; the mutable index delta is recorded. All caller canaries and
previous archives remain. See final3-cleanup.json, final3-home-inventories.json,
native-fault3.json, af-final4/native-results.json and observed engine identities.

## Commands and Explicitly not claimed

Actual argv, return codes, counts, timing, environment and raw failures are in
checks/*.json/log/stderr. Key commands (run from the repository):

```sh
cargo test --locked --offline -p kbox-runtime --lib vm::boot_diag::tests
bash scripts/verify-commit.sh
cargo clippy --locked --offline -p kbox-runtime --all-targets --no-deps
cargo test --locked --offline -p kbox-cli --lib
cargo test --locked --offline -p kbox-contract
python3 scripts/test-install-package.py
bash scripts/package-macos-arm64.sh
```

Package command runs under an isolated K_HOME, K_BUILD_LIBKRUN=1 and offline Cargo;
see build-box-final3.py and install-bootdiag-final3.json for exact extraction/install
commands. Native exact names and loader evidence are retained in their receipts.
Initial inherited-context.json is an immutable historical input, not current
acceptance/counts. Current context.json/final3 evidence is authoritative.

No full release/GA, global retention quota, crash-atomic whole-directory snapshot,
original startup root cause, stop-failure ownership, live continuity, fresh-host,
long soak, OCI conformance/security review or other-platform native pass is claimed.
Full OCI Host Service/default cutover remains incomplete. Windows ARM64 current
dependencies unsupported/no native run; support levels unchanged. Six full default
disk-service failures, prior SDK failure and historical unknown-actor seven-RAM
loss remain open. Current Git probes still fail UID501 lookup and GitHub DNS;
baseline branch is already a local-main ancestor, fresh remote merge/push unproved.
No repeated push under unchanged prerequisites, force push or publication.

Next local causal task: failed boot cleanup takes a handler and deletes resources
even if physical stop fails. Existing normal destroy retains the owner; reuse
that invariant and cover its boot/pool callers, without touching real ~/.kbox or
inventing another state authority. Overall continuous GA work remains active.
