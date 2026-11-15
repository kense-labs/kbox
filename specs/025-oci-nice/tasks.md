# Spec025 tasks

## Final accepted checkpoint

025 ACCEPTED. Final source2 7aa7feebab7594c99f4d9866d23050e164bf6e6551ed4c7378f5ecfd19c22661
(1700 inputs), Box2 15baefe0145e030046f16b1cdb4c2af72434e5b3bd581488b620f1fe2db76dfd;
OCI022 37009193… freshly extracted with original4f9dbe71… provenance.
Actual final5 Rust native +3 Python scenarios pass; no new Guest unit run.
Nice2 1/0/0,220.74s native/225.09s wrapper,15 observations across three generations,
root/nonroot/child/thread/userNS/zero/infinity and2 real CLI PTY cases.
Host2/files2 each1/0/0; A–F all5 pass. Verify2 exits0 in200.12s: Runtime2257/0/41ignored,
CLI935,installer8. Audit10 homes/95 absent sockets/0 owned resources or FD holders;
all7 baseline RAM devices and23 native/353 A–F cache blobs preserved.
Native1 failed at root userNS uid_map EPERM because empty capabilities correctly
exclude parent CAP_SETFCAP. C2 asserts that refusal and still tests the priority
boundary, plus requires successful mapped nonroot control. Candidate1/failure/
diagnostic retained; source2/client2/archive2 regenerated and actually retested.
[OUTCOME](../../docs/evidence/runtime/20261115-oci-nice/OUTCOME.md).
No external025 blocker. Next existing local W4 gap: rtprio/rttime; continue now.
The following checkpoints remain historical, not current job status.

024 ACCEPTED; all jobs/owned resources closed. Sourceb91eb406… /Box16b0e49a9…
/OCI37009193…original4f9dbe71…;11Rustnative+13Guest+3Python, verify2255/40ignored
CLI935installer8;15homes/129absentsockets/0ownedresources/7baselineRAM.
See accepted024 OUTCOME, not old checkpoints. Don't rerun its global frozen
assertions after025 changes or count historical evidence as a new run.

Root: artifacts/oci-nice-581649d8a124446983e5432838103584.
Private: /private/tmp/kbox-nice-581649d8a124.
Evidence: docs/evidence/runtime/20261115-oci-nice.

- [x] T001 Read resource/callers/primary scheduling contract, retain old
  archives+SHA and initial native process/mount/RAM/environment identities.
- [x] T002 Red admission/completion tests in existing siblings, map Nice;
  check exact values/defaults/capability/profile/refusal without CLI drift.
- [x] T003 Bounded C+native sibling: actual priority floor/EACCES/reuse,
  inherited child/namespace/hardraise, root+nonroot/zero/infinity controls,
  all entries/CLI/health/realTTY/attach/replay/three generations.
- [x] T004 Focused/owning/fmt/Clippy/verify, frozen Box rebuild/fresh isolated
  installs/unchanged OCI provenance; actual affected/A–F counters/results.
- [x] T005 Exact owned home/process/mount/disk/FD/socket/RAM/cache cleanup,
  OUTCOME/release/ledger and next existing local W4 gap.

No025 managed native resources yet. No external blocker. CLI/per-UID defaults
stay unchanged because nice is per-process, unlike the three accepted counters.

Red1actual0passed/2failed/0ignored confirms missing Box mapping. Production
adds only standard Nice enum mapping + qualification error text; no CLI/UID
default change. Primary Linux6.12.76 kernel/sched/syscalls.c can_nice compares
nice_to_rlimit with per-process RLIMIT_NICE or initial-userns CAP_SYS_NICE;
kernel/sys.c returns EACCES for denied setpriority. Actual native proof pending.
Caches/preexistingarchive/SHA/initial7RAM retained; cachecloneactive39426.
After1/fmt currentlyrunning. Next C/native/CLI probes then freeze/build/identity
and installednative plus appropriate affectedmatrix/verify/audit/docs.

After1actual18/0/1ignored. Client1/fmt/nativefixturecompiled0. Frozen1
42bfc31f213dd2e1b1113a35401b8ee777ba5d777c4444d4517cc1966965c469
1700inputs/5Boxruntime changes; CLI/OCI/Agent/native sourcesunchanged. Reverse
patch(onlyNiceenumcase+errorqualifier)hashmatchesaccepted024rlimitssource
exactly, recordedminimal-production-delta.json. Build1active64190,verify1active
35185. Plannednative: actualNice5paths/3gen/root+nonroot/child+thread/namespace/
zero/infinity/CLIhealth/PTY/attach/replay/recovery, thenHost/filePEP/A-F. Other
unchangedlimits/Secret/Guest checksretainactual024 identities, not gratuitously
rerunorrelabel; sourceproofandfullnewbaselineverificationrequired.
Nextinstalledarchiveidentity andNice1beforeaffectednativechecks.

Box1b0243a60d4b6733bd0c85f49fe1b02b30f06054f7a14242af541b79a3bf56899,
23,393,050compressed/44,171,264installed/11hashes,identity1PASS. Main1failed
initbeforefirstready/stoppedfilesystem; do notguessCcause. Diagnostic1script
preparedfrompriorcurrent-installedCLIrecipe, root0/emptycaps/nice25 config,
sleepinginit + capturedexec actualstderr/limits/identity. Native1active39997
finished(obtainreceipt), verify1active35185. Noaffectedmatrixstarted. NeedC
rootcauseandfixthenmainpassbeforeacceptance. Noexternalblocker.

Verify1PASSED227.50s,Runtime2257/41ignored CLI935installer8. Diagnostic1
frominstalledBox1 exits80 ONLY uid_map0->parent0EPERM; actualNice25anddirect
priority/child/threadchecksreachedbeforenamespace. Kernelverify_root_map
requiresparentCAP_SETFCAP(emptyworkloadcapscorrectlydeny). C2rootbranch
requiresexactmapEPERM,thenstillassertsNiceEACCES13insideenteredunmappedns;
explicitnonrootcontrolmustmapandassertsamepriorityboundary. NoCAPgrantor
fakeMappedflag; alloriginalNiceeffectchecksretained, plusmappingErrno.
Rustfixturechanged;source2 7aa7feebab7594c99f4d9866d23050e164bf6e6551ed4c7378f5ecfd19c22661
1700inputs/deltaonlynative_nice.rs. Client2compiled/frozen. Build2active11230,
verify2active96183. C1/candidate1/failure/diagnosticcleanupretained.
NeedBox2identity/payload-equivalence thenNativeNice2/Host2/files2/AF2(allcurrent
installed2); otherunchangedresources/Secret/Gueststayhistorical024withsourceproof.
Noactive025nativejobs now; allfailed1/diagnosticresourcesclosed. Noexternalblocker.
