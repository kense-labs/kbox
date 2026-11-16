# Pool control endpoint ownership

Bounded038 is accepted on this existing macOS26.6.2 arm64 host. Pool startup no
longer deletes a caller file, alias, special object or active control endpoint.
This is a control-socket/data-preservation fix, not full W4/default OCI Host
Service cutover or enterprise GA. Previous037 is committed as
`f5478a4afc31bca7b4260c25b539fdba80cbdbc2`; its installed evidence remains valid.

## Actual failures and shared fix

`initial-file-red1` actually runs1 test,0pass/1fail: initial bind overwrites the
caller regular file. `accept-queue1` adds a real full Unix accept queue and also
fails0/1: Darwin returns ECONNREFUSED61 for a live full queue. A refused connection
alone cannot authorize deleting an existing endpoint. All raw logs are retained.

The existing pool flock adapter is shared by autostart and listener admission.
They use distinct sidecars, so a parent awaiting readiness does not deadlock its
child. Locks use no-follow/nonblocking open, single-link same-UID regular-file
validation, exact opened/visible identity, and owner-controlled/root-sticky
canonical ancestors. Blocking filesystem/flock work stays off async executors.
New locks and sockets use0600. Existing644 autostart contents/modes are preserved.
Canonical endpoint ownership keeps parent alias changes from redirecting cleanup.

The listener lifetime lock contains a bounded32-byte version1 receipt:16-byte
magic plus exact device/inode. It is only initialized for a newly created file
and updated for an already recognized runtime record. Unknown/corrupt/truncated
records, caller aliases and writable shared lock files refuse without mutation.
This is listener ownership, not another execution lifecycle store. Stale recovery
requires a matching receipt, released lifetime lock, refused connection and an
exact identity recheck before unlink. An unproven legacy socket is retained and
explicitly refused, including a full live queue. An old crashed endpoint without
that proof requires the operator to establish its ownership/liveness before
removing it; the new daemon does not guess. Regular lock receipts remain after
normal close. ExecutionManager authority, persisted routes/generations, CLI
options, product configuration, native gitlinks and dependencies stay unchanged.

Implementation failures are retained too: `endpoint-focused2`7pass/4fail found a
17-byte magic copied to a16-byte field; a typed constant fixes it. `focused3`
10pass/1fail found creation/flock publication racing another opener. The creator
now retains publication responsibility; other openers refuse incomplete records.
The original assertion remains exactly1 concurrent winner/all othersWouldBlock.
`focused4`11/0 passes. The final CLI suite includes14 endpoint/close tests.

Candidate1's installed `trailing-path-red1` exposes a newly introduced canonical
filename mapping bug: `callerDirectory/` starts an unintended endpoint inside the
caller directory. Its owned idle daemon is stopped through that exact endpoint,
exit0, no sockets/VM/caller loss. The shared filename boundary now rejects trailing
directory syntax before lock creation, with identity/canary/no-side-effect tests.
Candidate1 and its raw failure remain; it does not qualify final source2.

A preparation helper also fails when comparing mutable image index.json against
its earlier hash after native autostart began. `runner-preparation-failure1.json`
records this helper failure. The original immutable byte map is retained and
compared; catalog mutations are recorded. Original cache metadata was not
captured, so no before/after cache permission claim is made.

## Actual checks

Every test/check wrapper allocates a fresh task-owned K_HOME and short TMPDIR;
commands/environments/exits are in `checks/*.json`, stdout/stderr alongside them.

- Final CLI `cli-full2`:949pass/0fail/0ignored, including all14 endpoint/close tests.
- Final Clippy Runtime/CLI/contracts all-targets passes. Both default verifiers
  pass whitespace/isolation4/support/workspace-all-targets/fmt then exit101 at
  Runtime2322pass/6fail/52ignored. The same six external disk-service failures
  remain; no green default gate is claimed and no assertions are skipped.
- Full contracts:106 top-level tests pass across11 harnesses;12 successful
  filtered subprocess receipts are separate. The historical118 total counted
  those children; `contract-counts.json` and appended036/037 corrections make
  that distinction explicit without deleting history. The >10000 journal replay
  test actually completes. Contract sources are unaffected by the final CLI fix.
- Installer regression8pass. Source freeze/client build is a no-run build, not
  native test execution. Final installed native counts below are checked from
  actual stdout/exit and result files; zero or ignored tests would fail acceptance.

Key commands through the recorded fresh-home `run-check.py`:

```sh
cargo test --locked --offline -p kbox-cli --lib
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli -p kbox-contract --all-targets --no-deps
cargo test --locked --offline -p kbox-contract
python3 scripts/test-install-package.py
bash scripts/verify-commit.sh
bash scripts/package-macos-arm64.sh
```

## Frozen final archive and installed native evidence

Artifact root:
`artifacts/pool-socket-ownership-d41adac0995149faa6cf946513f069fa`.
Private root `/private/tmp/kbox-pool-sock-d41adac09951` has its owned marker.
Final compiled/package source digest:
`cfabba20a920f5a7202d15aa0ff42bce66e1163688a4ec72caf237358da284af`,1726 inputs,
based on mainf5478a4a plus recorded038 changes. Rustc1.97.1;
lock `8497275783877e36efa1dcea71fc76927dcdf42d1994ea522a8500ea86414734`.

Final archive:
`artifacts/pool-socket-ownership-d41adac0995149faa6cf946513f069fa/final-dist-poolsock-2/kbox-3.4.3-macos-arm64.tar.gz`

SHA256 `782cd1ae14a8fe1e2dc83a841c8834b0b05cccf11663f94541eb8d455df87151`;
23422934 compressed bytes /44208128 installed bytes. Fresh prefix is
`installed-poolsock-final-2` under that root. All11 manifest files, source identity,
CLI/shim ARM64 strict ad-hoc signatures, Linux aarch64 guest-init, dylib identities
and embedded OCI Agent match. No signing/notarization/publication promotion.
Candidate1 SHA6f78cf433ac06af93a8dd0ebdcefcf80c5b7958ce1e00701a6a4ed470f780822,
23423513 bytes, and all older archives/checksums remain retained.

The separate unchanged OCI archive remains
`d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79`, original source
`d2b442f175f89b34d6ba9b63e6d2df4a478dc2a6c3493f8ca647a761e9012871`.
Its original owning-repository native patch provenance is retained; it is not
relabeled as038 source. Parent libkrun3c2889c7/libkrunfwfc3ea068 stay clean.
The frozen Runtime client is6f75291a6869e67f3f096e18d11d10ad76f189a691676bd27d2a2a763a299cde;
unchanged Runtime sources explain identical client identity to037.

Actual final native results using installed companions and checked loader maps:

- `endpoint-native2`:13 negative endpoint/lock/path cases plus duplicate-live
  refusal; real saturated queue returns ECONNREFUSED61 without losing the active
  socket. Caller bytes/modes/identities and neighboring objects remain. An owned
  idle/no-VM daemon is deliberately SIGKILLed, then exact receipt-bound recovery
  and normal stop pass. Two concurrent autostart clients run2 real Guest commands,
  proving the separate locks do not deadlock.
- `af-final2`:all5 rows pass:3 Python scenarios for OCI lifecycle, Host RO attacks/
  restart and engine process flow;2 Rust native tests for process prewarm and
  directory grants/fencing, each exactly1pass/0fail/0ignored. Host RO has14 real
  denials on both first and restart probes after privileged RW remount/fresh mount
  and child attempts, with RW control. Identity/restart, streamed process I/O,
  replay/no-late-write and old-generation refusal assertions remain intact.
- `pool-wave-final2`:4 daemons,16 initial authenticated Ready VMs,4 actual Guest
  commands pass; all4 pre-existing hash-named caller template directories retain
  exact dev/inode/bytes. The Mac unsupported snapshot request uses the existing
  explicit same-backend cold optimization; no Linux snapshot claim.

The new endpoint fixture has real Guest/native loading plus deliberately killed
idle daemon controls. It is separate from the3 Python AF scenarios. This phase
does not rerun037's unchanged synthetic stop-refusal fixture or turn its historical
third Rust native pass into a new038 test.

## Owned cleanup and continuing work

`final-cleanup.json` and FD/inventory receipts audit53 private homes, both owned
roots, both verifier short roots,37 observed VM socket directories,154 exact
socket paths,2 engines and727 immutable cache files. Managed executions/volumes,
owned VM/shim/engine/Host processes, mounts/disk images, FD holders and sockets
are empty. Only the mutable image catalog changes. Caller data, candidate1,
previous archives and regular ownership-lock receipts remain. Audit deletes
nothing; fake socket fixtures were removed only by their creating runner.
RSS is unmeasured. Historical default-gate real-home nonmodification remains
unestablished as corrected by035; this phase does not scan/repair the real home.

Pre-admission foreground boot/restore/postboot/health transfer and legacy CLI
boot/compose errors still require actual-owner lifetime work. Initial control
binding is closed only within the above ownership contract. Arbitrary hostile
same-UID namespace mutation/ACL setups, other-platform native, Linux snapshot,
fresh-host/OCI conformance/security review/soak, original15s boot root cause,
six default disk failures/prior SDK failure and earlier unexplained RAM loss
remain open. Full OCI Host Service cutover, live continuity, W4 and GA are not
complete. Windows ARM64 current dependencies remain unsupported/no native run;
platform support levels and default libkrun/BoxVm route stay unchanged.

Git baseline branch already has local ancestry in main; fresh remote equality
and push remain blocked by UID501 lookup/GitHub DNS, with the prior
[prerequisite proof](../20261116-pool-shutdown-ownership/checks/git-prerequisites1.json).
No unchanged push retry, force push, external merge or publication is claimed.
After task-only source/diff/commit closeout, continue the existing local owner gap;
the continuous goal remains active.
