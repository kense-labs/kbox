# Spec014 Guest masked and readonly paths

Status: accepted bounded Guest guard fix on this existing macOS arm64 host.
Final installed assets, actual Guest protocol effects and owned cleanup passed.
The continuous W4 task remains active; full cutover/default switch and GA are open.

## Implemented behavior

The existing shared Box Guest Init guard now returns typed I/O errors. Both
command preparation and PTY propagate failure before spawn/openpty/fork, using
existing failure output/Error frames. Only actual NotFound is skipped. Invalid,
excessive or unsafe paths refuse; symlink components are explicitly unsupported.
Targets are pinned component by component, and actual mount IDs and inode
identity are checked before readonly attribute changes and again afterward.
Readonly uses the existing Linux fd mount API recursively, covering inner mounts.

A different st_dev is no longer evidence of masking. Directory masks require
this executor's kernel mount source plus actual tmpfs/readonly/empty readback.
This prevents an unrelated mount, or caller RO tmpfs with a live RW alias, from
being mistaken for a valid mask. File masks reuse the existing OCI private-empty-
source pattern: a private Guest tmpfs, configured size=4k, empty mode000 file,
readonly before attachment, exact FD/identity readback and temporary-builder
teardown. This is not a measured memory value. Existing source bytes/descriptors
and sibling data remain unchanged. Repeated application does not stack mounts
or leak FDs or staging directories. No dependency/workspace/config/wire schema,
resource store, native source or route/default change.

The high-level manager still filters caller K_SEC_* variables and inherits the
durable boot security policy. It was not weakened to make the fixture pass.
Trusted Guest protocol callers provide the existing CRI path controls; lifecycle,
restart and generation fencing remain with ExecutionManager.

## Actual failures and corrections

- Before Guest1 ran0/1/0: Linux parents0/2/3 proved exec/PTY continued after real
  mount failure; separate mounted-target0/1 proved a canary stayed visible under
  an unrelated mount. All failure outputs and owned cleanup remain.
- Expanded Guest3 proved empty caller RO tmpfs could later expose data through
  its RW alias. Kernel-source identity plus actual mask readback fixed it.
- Guest3–7 retained an initial file-mask ENOENT. Diagnostics proved the OCI
  supplied /dev/null was an unlinked node: pin and open_tree succeeded, attachment
  failed. Pathname attachment also failed. The private empty readonly source
  remains linked until attachment, then its builder detaches. No weaker fallback,
  device permission change or workload-specific exception was introduced.
- Linux Clippy3 refused non-octal mode(0) and found an unused staging baseline.
  mode0o000 and the missing actual comparison fixed both; assertions were added.
- Binary1 used missing /bin/sha256sum; the fixture now uses existing BusyBox.
  Binary2 incorrectly treated firmware PID1 as the executing Guest Init. Real
  PID1 is /init.krun; the actual /sbin/init server parent hash is now verified.
- Binary3/4's **managed exec** marker interpretation was incorrect: manager
  request K_SEC_* values are intentionally filtered. Candidate1Binary5 already
  refused PTY correctly. [Caller correction](checks/binary-exec-layer-correction.md)
  preserves the evidence and policy; the fixture now uses existing ExecClient on
  the owned current socket. Old13raw Binary7 then actually created both forbidden
  markers, while new14raw Binary6 passed effects/refusal/restart. Generic binary
  image labels were also corrected; it uses sameUidImage, not a foreign-UID claim.
- Candidate1/source24a remains intact. Final2 native build/archive succeeded,
  but a diagnostic copy helper reused the candidate1 directory name. mkdir
  refused overwrite. [Failure](checks/package-final2-helper-failure.json) retains
  the original helper. Resume verified the already-built source2 archive, then
  copied/installed into fresh final2 paths; no unnecessary rebuild. Premature
  identities2 correctly refused the old prefix and wrote no passing result.

Other prototype results, quote-diagnostic failure and source/receipt corrections
remain in checks/tasks. No failed assertion, package or history was removed.

## Final source and assets

HEAD405c8b89f151e9e2879438d9e457dd60bfd18b90 remains dirty and unstaged. Source2
SHA256 `8f562b1b69caeee726b22c068a8b3909cd515fa30c31b4e0759aa34408055f55`
covers1669inputs,8changed from accepted013. [Component comparison](checks/source2-component-equivalence.json)
proves only the native caller fixture changed after source1; Guest and production
inputs stayed identical. Exact paths and original provenance: [context](run-context.json).

| Final archive | SHA256 | Compressed bytes | Installed disk bytes |
| --- | --- | ---: | ---: |
| Box3.4.3 final2 | `c6eb7199786662c6119a731f70811be1154f082f9ccfe7f88af4c49fa2af0285` | 23365380 | 44118016 |
| Unchanged OCI0.3.6 accepted012, fresh extraction | `df4b882c40f607b56e098b9f971552ad879f8a156fdc35703cc79fd8faf71a2a` | 33345582 | 114847744 |

Old global/accepted013 archives and sidecars were saved before packaging; candidate1
and prior originals remain. OCI production inputs did not change; its original
source remains dada2886…, not relabeled source2. Both final prefixes passed11file
hashes, arm64 CLI/shim/native libraries, static aarch64 Guest/Agent, signatures,
dylib links and embedded-Agent equality ([identities](checks/final-identities-2.json)).
Workers loaded only these installed libraries and executables or exact owned
copies. The actual Guest server executable SHA256 is
`302300eaeeeca31577a90806c131fa19f3d1530f7682f7f7524b0fa524675cb1`.
Native client SHA256 is
`329c0a5c7d9a2822c29018300087b5bac88b5b2bc017bc5dc92f2a5de7b4b615`.

OCI native bundle remains009 d7a398ec… (lib7dcaf940…/firmware841bc9d5…/
kernelb1180b50…) with original sidecar provenance; BoxVm retains its independent
baseline libraries. Parent native gitlinks/working trees are unchanged.

## Commands, counts and effects

Existing macOS26.6.2(25G83), M5Max arm64, euid501, Rust1.97.1. All homes/caches
are isolated; no real ~/.kbox. Exact argv/exits/durations/stdout/stderr and frozen
helpers are in checks and artifacts/path-guards-90a50b1c75b5416bb6741f5c12536c05.

- Guest host all-target tests:156/0/0,8.87s; Linux-only cases excluded here.
  Host and Linux-musl Clippy passed; Linux uses existing cargo-zigbuild wrappers.
  Fmt passed. Final cross test build2 exit0,0.45s, fresh artifacts; not a runtime pass.
- verify-commit2: exit0,249.49s; Runtime2222/27ignored, CLI933, installer8;
  all contract suites, workspace all-target check, fmt, scoped Clippy/support
  checks pass. verify1's earlier241.82s pass remains separately recorded.
- FinalGuest11: actual1/0/0,40.62s on real HVF Linux Guest. Filter reports4/0/6,
  plus four child1/0/0 and four explicitly launched1/0/0. These include real
  missing-authority refusals, unsafe/symlink containment, unrelated mounts,
  caller RO/RW alias attack, empty-file/dir masking, actual EROFS including
  nested mounts,32repetitions, unchanged mountinfo/FDs/staging and source bytes.
- Final installed binary8: actual1/0/0,6.24s. Real raw exec and PTY refuse unsafe
  controls and leave both markers absent in generations1/2. Actual masked data
  is hidden, ordinary readonly file/directory writes return EROFS, sibling writes
  work. Two observed worker PIDs use one execution9344e63e-aed3-47a2-a99f-28dd16bcadaa;
  restart advances1→2 and managed stale generation returns Conflict. Each actual
  server parent hashes to the installed Guest Init, not firmware PID1 or a dev binary.
- Existing rootRO regression:1completed installed CLI scenario,3.41s; root EROFS,
  init/exec/real PTY, writable tmpfs/caller bind, restart tmpfs reset/data retention.
- A–F final2: all5passed,143.72s: OCI lifecycle, Host RO attacks/restart,
  primary streams/engine, prewarm and directory grants. Last two each actual1/0/0;
  other drivers each validate1complete scenario result. [Actual result files](checks/af-final2/native-results.json).

The [final receipt review](checks/final-receipt-review2.json) asserts actual counts,
marker contents, server hashes, worker identity, generations and component source.

## Cleanup and continuation

[Final audit](checks/final-cleanup.json):24homes,0managed executions/volumes,
owned processes/mounts/workload disks/temporary builders,118observed socket paths
absent. No owned descriptor holder remains. Seven prior RAM mounts/unrelated
mounts are unchanged;15main,15binary and101AF immutable source blob files match
expected digests. Audit deleted nothing. Caller canaries/inputs survive in the
owned evidence roots; only canonical run-owned resources were removed.

Next local W4 slice is queued OCI tmpfs intent through the existing Guest mount
executor. Current source is frozen only for this accepted phase; continued
implementation will intentionally produce a new source identity.

## Explicitly not claimed

These are newly booted Box Guest path restrictions, not Host inode-wide RO,
revocation of existing descriptors, or resistance to explicitly granted Guest
SYS_ADMIN. Symlink components/unsupported Guest mount APIs refuse. Existing
running old Guest servers require restart with updated installed companions.
No full OCI read_only/tmpfs/Secret parity, cutover/default migration, live HVF
session continuity, complete security review/conformance/soak, fresh-host or
cross-platform qualification, support/readiness promotion or GA. Windows ARM64
remains dependency-unsupported and lacks native validation. No runtime RSS measured.
No commit/push/publication; both previously missing historical evidence files
remain missing, not fabricated.
