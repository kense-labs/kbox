# Retained failures and corrections

- nproc-before1: actual0passed/2failed/0ignored; existing Mac admission and
  process completion refuse NPROC. Shared mapping/config/actual-entry guards
  fix it; nproc-after1 actual31passed/0failed/4ignored, expandedafter2 actual32.
- Native source preparation initially matched valid --memory and metadata=
  text as old memory-limit leftovers. No product test source was written before
  those guards failed. The generator now checks full former limit tokens,
  preserving all intended behavior assertions. Preparation is not native proof.

Checkpoint after restored context: final1 native nproc attempt failed 0 passed /
1 failed /0 ignored in45.36s. The exact valid-UID extra exec entered and exited0
while the --hold process(pid450,start sequence17) remained alive; extra pid481
created18/started19/exited20. nproc-full-budget-rejected.txt retains Ok(exit0).
Before failure five actual process paths proved UID1001/GID2002/NNP1/allcaps0,
fork14+EAGAIN11/reap14/reuse and pthread14+EAGAIN/join/reuse; user namespace
entered and mapped, fork13/EAGAIN/reap/reuse. These partial effects do not accept
022. Test cleanup boxes[] and Host/workers reaped; no new022 RAM/VM remains.
verify1 exited0 in221.17s, actual counts still to aggregate. Frozen1 archives,
client and source retained. Root cause: Linux6.12.76 flag_nproc_exceeded checks
new UID count before commit_creds increments; equality at soft limit does not
set PF_NPROC_EXCEEDED, allowing root helper's setuid+exec at full quota. Official
versioned kernel sys.c/fork.c/ucount.c/fs/exec.c inspected. Need shared Agent
entry fix with actual kernel enforcement, preserve exact PID/ownership and all
assertions; do not invent a PID state store or silently substitute cgroup pids.
Then freeze2/rebuild affected OCI+Box/client, focused+owning/default+final
installed native/affected/AF tests, audit/docs and next W4 item.

Shared fix implemented/frozen2 a0edbd9e9b4ac1da0059bbeff8dd5a4dae7c61797ca1833b10a98569d344daac
1696inputs. Agent prepares only one temporary soft NPROC slot(and hard only if
needed), performs an atomic credential-bound fork/reap admission, restores/verifies
exact requested limits before exec. No workload PID or process registry changes.
Two added Agent tests cover finite/infinite preparation and isolated UID-change
zero/one-slot effects+no children; final native Guest runs actual five rlimit tests.
Host advertises nproc-admission v1; Box preflight requires it before preparation.
Explicit Agent manifestv10 binds prior transport schema(v1-v9), auto wraps all
NPROC utility-VM inputs, and preserves prior meaning/omission bytes. HVF and KVM
producers both reached; no KVM native support promotion. Native old Host/shim/
Agent refusal test plus exact no-Start pending cleanup added. Full quota attack
retained, strengthened with concurrent valid execs and valid stream/PTY entries.
Protocol+SDK owning pass: protocol70,SDK227lib+18integration; actual logs retained.
Box nproc-focused7 actual4pass/0fail/2ignored. Build preparation compile errors
(missing native-test SHA import; guessed IsolationRequest method) fixed; fixture
wrong ResourceConfig then absent runtimeRoot fixed to qualified defaults+existing
private empty root. Failures focused3/4/5/6 remain. Native-client-ready3 compiled;
frozen2 clientf949c7a849e2f7d6b9a605023e773228a3c6d87a494755e16f494ebc76a0f663.
Current jobs: OCI owning51723; OCIcandidate2build31699; Boxfinal2build86616;
Agentunitreleasebuild95945; verify287746; OCI+AgentLinuxClippy running. No new
022 native/RAM launched yet. Next finish builds/package/install/identities, actual
Nproc2+Guest5; old3/refusal, affected CPU/memory/oldlimits/Host/full AF5 and memory
environment regressions as needed; count/effect/results checks, strict owned
resource/cache/FD/socket audit, docs then next local W4 gap.

Native2 actual0pass/1fail/0ignored in37.04s; its actual Guest top-level4pass/
1fail: zero budget refused correctly, one-slot fixture failed because libtest
coordinator/thread share the UID budget. Preserve guest stdout/stderr and
Box2/OCI2 archives/installations. No main workload/full-budget retest ran.
Cleanup boxes[]; owned Host/VM reaped. Unit probe now forks a truly single-thread
child before credentials, retaining exact zero/one/hard-restore/no-child assertions;
parent records actual launcher thread count. Formatting first caught missing Rust
unsafe-expression parentheses, corrected before freeze. Two builder launches
preceded completion of the setup cell and found no script; no build/mutation ran.
Frozen3 4f9dbe71f19a9d018e40b0a58e5a62e2dd2cde5d07c9698f7e710c9a8f2a648b
1696inputs differs from2 ONLY in Agent rlimit/nproc_tests.rs; production unchanged.
Still regenerate/repackage Box/OCI and Agent unit ELF to one final source digest.
OCI2 b625cdb4…33,421,177B/114,950,144installed/11hashes; Box2 7b2ac56d…
23,392,254B/44,171,264installed/11hashes, identities2PASS. OCI owning1actual
438pass/0fail/2ignored(448.72s), verify2actualRuntime2246/38ignored,CLI935,
installer8(351.11s), OCI+LinuxAgentClippy passed. Current build3/verify3 and unit
ELF3 jobs active. Native2 failed tests are not counted as acceptance. Next finalize
3 assets/identity then Nproc3+Guest5. Do not start independent next W4 until022
final matrix/cleanup/docs are sealed; do continue local work without a final.

Native3: top-level0pass/1fail/0ignored in233.19s, cleanup boxes[]. Actual Guest
rlimit5pass confirms launcherThreads2 / actual probeThreads1; zero refusal, last
slot and restored hard limit. All five process paths through first/restart/stopped
recovery passed plus full-quota valid exec/concurrent exec/stream/PTY returned
actual ResourceExhausted(errno11), no marker entry and holder/reaping/reuse passed.
Failure is CLI health still unhealthy, not quota enforcement. Diagnostic1 uses
installed3 in new owned root: direct /nproc-probe --health and /bin/sh -c succeed,
bare sh fails Box Host-side executable lookup in empty ext4 placeholder. Cleanup
ps[]/volumes[]/Host reaped. Focused guest-exec-path-before1 red confirms shared
rootcause. Fix delegates only persisted LibkHvf exec PATH lookup to existing Guest
executor, carries effective image/default PATH on omission, preserves explicit
PATH, absolute/relative normalization and PATH-entry escape rejection; non-HVF
Host preflight unchanged. Focused after1actual7pass, after2actual8pass expected
(read command receipt). No health command assertion weakened or shell hardcoded.
OldHost3 actual1pass(preallocation capability refusal); oldShim3 and oldAgent3
actual1pass each, workload marker absent, pending exact Create resumed and force
deleted with zero Starts. Their outer evidence reader failed: oldShim reason is
in retained refusal.txt, not .log; oldAgent stderr is deliberately discarded by
worker stderr(Stdio::null()), bootstrap console does not contain its decoder
reason. Keep these failed outer receipts. Native4 adds a trusted root control
probe of exact old Agent against canonical bytes of the actual current nproc v10
manifest; only its own control share's manifest is temporarily replaced, restored
before teardown even on Rust error. Guest stdout/stderr must prove unknown-field
refusal; exact old-agent SHA links to separate old-image boot refusal. No product
logging expansion, native modification or test weakening. Main nproc4 provides
that proof before old-agent4 outer validation runs. Root control adds SYS_ADMIN
solely for existing old-Agent mount probe; actual nproc workload remains caps0.
AF3 PASS all5(2Rust+3Python),139.38s; Secret ENV3 PASS1Rust+8Guest,240.55s,
owned RAMdisk12 detached and no owned runtime processes. These retained3 results
precede new PATH source changes, so affected final4 archive cases still need run.
Box3 archive7378048c…23,392,269B/44,171,264installed; OCI3 37009193…
33,421,132B/114,950,144installed; all11checksums each/identities3PASS. Source3
4f9dbe71…; unitELF3 749ad05c…; client3f949c7a8…; verify3passed189.63s.
Next freeze4 after focused path/nproc checks, rebuild affected Box4/client4 and
repackage OCI4 with current source/provenance; Agent unit production unchanged
from3 but record exact identity. Run actual Nproc4/old3cases4, CPU/memory/limits/
Host/full AF/ENV affected matrix, final strict resource/cache/source audit and
OUTCOME/release/ledger. Then continue next local W4 gap. No final yet.

Frozen5cc9e1c37a0012ce51169262970c27f90d110ef47648fea8a373952fbcd32d4d0
1697inputs, differs from4 ONLY native_nproc.rs test fixture. Product implementation
unchanged. Box5f190440c72f88159c6c33f40050f4a5dbe556e6e369e3b85f45805e116b34c0a
23,392,791compressed/44,171,264installed/11checksums; OCI37009193… original4f9…
source reused with another fresh5 extraction/11hashes. identities5PASS. Strict
production-equivalence4-to5.json proves every packaged payload identical except
build-info.json; retain actual4 CPU(323.09s),memory(195.94s),oldlimits(194.62s)
passes without gratuitous rerun or relabeling their original archive/source.
Client5c34f39c26fcb24ba1e82f906826f3bfcc6eb2d694f4f638af58d0efea924e958.
Main4 failed ONLY old-Agent in-place-bootstrap probe(0/1/0,84.28s), before CLI.
Its live-file read consistency refusal was valid, not product quota failure; no
consistency checks relaxed. Replaced that test mechanism by immutable pre-boot
SDK-generated manifest and existing installed VM worker. Calibration direct1
refused missing listening socket; direct2 private config mode; direct3 actually
booted but Guest stderr remained invisible. All failures, sources, binaries,
images and reaped workers/socket cleanup receipts retained. Direct4 captured
unknown-field via tiny stderr-only launcher; direct5/6 add native waitpid exit1.
The test-only fixture image embeds the exact old Agentfa5e1d36… unchanged at
/usr/bin/koci-old-agent plus a redirect/wait launcher at the required bootstrap
path. Native firmware/kernel unchanged. Explicit test image hashes/launcher
source+ELF identities/debugfs dumps prevent claiming it as original archive image.
Direct6 PASS actualVM1, old guest Agentexit1/unknown nprocTransportSchema before
token consumption, reaped process and removed exact owned socket directory.
Original-image API refusal is separately tested with no workload marker and
exact no-Start cleanup; NativeOldAgent5 asserts/correlates direct6 agent+original
image SHA, exit1, schema/error and cleanup. NativeMain5 no longer mutates live
control manifests; only Guest5 rlimit unit control remains. All original quota,
health, realTTY, replay, identity and cleanup assertions retained.
Active final5 jobs: main62133, oldHost70386, oldShim21039, oldAgent86422,
HostService63134, files92730, AF37690, ENV21400, verify555683. Wait those actual
results/counts, resolve any failure, then strict owned audit including failed
roots1-4+health-diagnostic1+all direct fixture roots+AF3/AF5 exact build additions.
Direct fixtures are regular files, not Host mounts. Seven initial RAM devices
preserved; ENV3 removed owned RAM12; ENV5 owns only its new proven device.
No OUTCOME acceptance yet. Need final review/docs/release/ledger, then next local
W4 gap. No final while executable local work remains.

Final receipt review first guessed an aggregate18 SDK integration result line;
actual log has six binaries(1+1+3+2+3+8). The reader now sums those exact
successful counters, with zero failed/ignored, rather than fabricating a single
line. No test/product change. Two exploratory reads used non-existent paths
(run-context belongs under evidence; cleanup is cleanup-boxes.json) and were
corrected from actual inventory. No result files were overwritten.
