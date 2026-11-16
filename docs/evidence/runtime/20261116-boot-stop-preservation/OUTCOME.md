# Outcome034: retain failed-boot resources after stop refusal

Bounded default-libkrun correction accepted on the existing macOS26.6.2 arm64
host. Full GA remains active. This preserves resources while the VmManager is
retained; outer pool/caller drop-on-error ownership is required next work and
is explicitly not solved here.

## Shared change and actual regression

cleanup_boot_failure took its handler, logged stop errors and then deleted
working data and created anonymous allocations. Normal destroy already refuses
that release. The shared failed-boot path now restores the same handler and
returns early after bounded diagnostics on any stop error, retaining working/
volume/network/socket resources and state. Taking the handler is a separate
statement so its write guard is released before restoring/awaiting. Retrying
successful finalization permits existing cleanup. Even process exit does not
prove backend finalization. No new public API, queue/state/config, dependency,
timeout, route or generation authority. Selected original-owner boot bypass
remains; legacy boot/restore/maintenance and no-launch OCI preparation callers
retain their original typed errors and success behavior.

Actual boot-stop-red1:0pass/3fail, proving removed working data, lost retry owner
and ignored already-exited finalizer refusal. Green3pass; broader VM236pass/1ignored;
final focused3pass/1ignored. Tests reuse existing inner handlers through a small
transparent one-shot stop wrapper and real isolated files/allocation records.
No assertions/default tests weakened or failures skipped.

## Actual source, archive and checks

Frozen source `629e2bac05b4102ad569ae699a45814de438cf873dfa0d70a0c98867acf30a42`,1720 inputs, base
`a85fb534b7bf8cfec580f16ac65c85521d4ca5d4` with scoped dirty change.
Archive `/Users/roylin/code/kense/kbox/artifacts/boot-stop-preservation-3f488eca50004416875e318fc753940a/final-dist-bootstop-1/kbox-3.4.3-macos-arm64.tar.gz`;
SHA256 `4093b9d508588359e25012206deb5767053c41c0fdde3960705d70fd1a796b2e`; 23411039 compressed bytes /
44191744 installed disk bytes. Fresh prefix `/Users/roylin/code/kense/kbox/artifacts/boot-stop-preservation-3f488eca50004416875e318fc753940a/installed-bootstop-final-1`.
All11 manifest companions, CLI/shim/guest-init/native architecture/signatures,
dynamic paths and source/lock binding pass; every identity command exits0.
Frozen owner test client `2663ebf949afedb2a7e8c7c1762ed67f9a2a9773dfda2f00bbbbe5a22a590f90`.
Root lock/KCL/native gitlinks unchanged; clean native sources3c2889c…/fc3ea068…;
no native submodule edited in the parent. Separately qualified native bundle
and unchanged OCI spec031 archive d5f59995… with original source d2b442f… reused.
Prior Box033 archive5b59a546… and both prior checksum archives remain unchanged.
Runtime RSS is unmeasured.

scripts/verify-commit.sh actually exits101: all-target workspace/support/fmt pass;
Runtime2302pass/6fail/51ignored. Same2 RAM/Secret and4 APFS/provider disk-service
failures remain with complete logs. Runtime/CLI Clippy passes (one nonfatal new
clone-in-test suggestion retained); CLI935/0/0, contract library24/0/0, installer8
pass separately. Unchanged full contract118 suites retain preceding033 evidence;
they were not newly executed here. No cross-platform runtime or full-green gate
claim from these results.

## Real native effect, input identity and cleanup

Exact native test vm::tests::boot_stop::native_failed_stop_preserves_live_vm_then_retries_real_finalization
runs1pass/0fail/0ignored. Its owner client invokes actual production code using
archive-installed shim/guest-init/libs; A–F invokes the installed CLI. No development
runtime payload or real user installation borrowed.

A metadata-only VOLUME fixture was built by the prior accepted installed033 CLI
(no RUN). Actual input reference `kbox-stop-preservation-native:proof` binds digest `sha256:12762ced0b57498e6330dad350e35e388d0eab8765e39e38e4a7702a90abe3db`
and config `sha256:4253bbbbf222446071e4e43fa8ec097ceed05b2b1b6bcbdeb698913fe2bd2b27` before/after the run; immutable cache
hashes stay equal. The Linux arm64 image actually declares /scratch and the VM
actually allocates one anonymous volume. Cached qualification-tools digest74dc9800…
remains the unchanged A–F base, avoiding obsolete uncached ownership references.

A real authenticated VM writes caller data and that actual anonymous allocation.
One synthetic stop error is injected transparently around its real handler.
The exact PID/handler, allocation record and directory dev/ino remain; actual
Guest writes continue after refusal and bounded failure logs are retained. The
next actual backend stop succeeds, working/socket/anonymous resources disappear,
and caller bytes stop changing after return. Caller canary remains. This proves
the effect with a real VM, **not a genuine OS/hypervisor stop-error occurrence**.
All live loader mappings, PID/config, before/after allocation proofs and cleanup
are retained in native-fault1.json and native-case/.

Final A–F5 rows allPASS:3 Python scenarios (OCI lifecycle, Host readonly attacks/
restart, primary streams),2 exact Rust1/0/0 tests (prewarm and directory/identity/
fencing). Final totals3 Rust native tests +3 Python scenarios; no zero-test or
ignored default is counted as native success.

Final audit passes7 actual homes, including the image-builder home,
19 observed owned VM socket directories,
78 exact socket paths and2 engine processes.
Zero owned executions/volumes/VM/shim/engine/mount/disk/FD/socket remains.
685 immutable cached inputs retain bytes/UID/GID/mode;
mutable index changes recorded. Caller data/previous archives remain. Audit
deletes nothing. See final-cleanup.json, final-home-inventories.json, native-case/
and af-final1/native-results.json.

## Commands and Explicitly not claimed

Actual argv/status/counts/environment/timing are in checks/*.json/log/stderr.
Main commands:

```sh
cargo test --locked --offline -p kbox-runtime --lib vm::tests::boot_stop
cargo test --locked --offline -p kbox-runtime --lib vm::
bash scripts/verify-commit.sh
cargo clippy --locked --offline -p kbox-runtime -p kbox-cli --all-targets --no-deps
cargo test --locked --offline -p kbox-cli --lib
cargo test --locked --offline -p kbox-contract --lib
python3 scripts/test-install-package.py
bash scripts/package-macos-arm64.sh
```

Packaging uses isolated K_HOME/offline Cargo/K_BUILD_LIBKRUN=1; exact extraction/
installation and native invocations are recorded in helpers/receipts. Current
native result is a synthetic one-shot refusal around a real VM; no actual kernel
stop-error or other-platform native result claimed.

Pool/future/local caller drop-on-error can still lose the retained manager;
that outer lifetime issue is the next required bounded task. Original intermittent
15s startup cause, full W4/OCI Host Service/default cutover/live continuity,
fresh-host/conformance/security/long-soak/platform/GA gates remain open. Windows
ARM64 dependencies unsupported/no native run; support levels unchanged. Six default
disk-service failures, prior SDK failure and historical unknown-actor RAM loss
remain failures. Git UID501 lookup/GitHub DNS prerequisites remain parked from
033; remote merge/push unproved, baseline branch already a local-main ancestor.
No real ~/.kbox, unrelated resources, new workspace, force push or publication.
