# Spec020 tasks

Current status:020 accepted. Final sourceccc6c32f… /Box57d38d06… /OCI542bfe43…;
8 outer Rust +8 Guest units +3 Python native scenarios, verify2240/934/installer8.
20-home/208-absent-socket audit has zero owned resources/FDs and7initialRAM.
Next021 data/stack/memlock; continuous W4 active, no final reply.

Starting checkpoint:019 accepted, no native019 resources remained. Source03a62c3e… /
Boxb9b77584… /OCI542bfe43… kept. Closure doc-check exit0. Continuous W4 active.
020 root: artifacts/oci-cpu-time-092623a0dfc94f14995861ff217ee4cf;
evidence: docs/evidence/runtime/20261115-oci-cpu-time.

- [x] T001 Read handoff, accepted019/W4 and owning caller/executor chain; retain
  old archives/checks/environment without touching unrelated resources.
- [x] T002 Establish CPU admission red test; implement shared mapping, bounded
  validation and focused tests without changing other limits/defaults.
- [x] T003 Static probe and native all-process/CLI/replay/fencing/recovery test.
- [x] T004 Owning tests/Clippy/fmt/verify; freeze/build/install artifacts; run
  actual native matrix and applicable regressions with counts/result checks.
- [x] T005 Owned process/FD/socket/mount/disk/cache audit, OUTCOME/release/ledger;
  close020 and immediately continue next local W4 item.

Next: focused CPU admission and Process.rlimits assertions must fail on current
helper. Existing Agent production already implements RLIMIT_CPU. Keep positive
finite seconds and infinity, refuse zero explicitly. No external block.

CPUbefore1 actual0/2/0, CPUafter1 actual27/0/4ignored. Core parser unchanged;
shared Mac mapping admits positive/infinity and rejects0 before preparation.
Nativeclient-ready1/2 exit0, static AArch64 musl probe crossbuild1 exit0; not
native acceptance yet. Source frozen1 6615f84bc0724b27fd6494c66a2a7f7b3e1fa26c5744bb8157e6aba230df7005
with1693inputs; only five Box runtime files differ from019; OCI production
source inputs are exactly unchanged and retain03a62c3e… provenance. ReusedOCI
archive542bfe43… extracted fresh,11hashes verified. All baseline cache metadata
and actual dist retained. Next build/install Box1, identities, CPU-native1 and
affected regressions/AF5, verify and exact audit. No external blocker.

Box1 installed6559c4be0d9e1803ead579dbcd7511daf567cf983845def50899dc20db9149b1
23,390,481compressed/44,154,880installed,11hashes; source6615f84b… .
OCI019542bfe43… extractedfresh underoci-reused-installed,original03a62c3e…
provenance retained. Currentidentitycheck3276; verify1active20233.
Next CPU-native1 plus ulimits/wholeHostService/rootRO/tmpfs/ENV/AF5 from exact
installed assets. No native020 has started yet; no owned020 RAM mounted.

Native020first-generationinit/exec/child/stream/realPTYprobeoutputsnowshow
actualSIGXCPU24around1CPUsecondandSIGKILL9around2CPUseconds,hardraiseEPERM;
idle2.2wallsecondsuses<0.001CPU. Directexecsoft/hardmarkersobserved. Notfinal:
replay/restart/recovery/CLIandcount/cleanupstillpending. NativeCPU54808;
processregressionchain90655;service+ENVchain31880;AFretry42558;verify20233.
AFfirstfixturefailedbeforestartinganytest(missingparentdir);failurepreserved,
newrunnerassertsabsoluteownedrootandcreatesitbeforework;freshaf-final2retry.
Nextfinishactualresults,fixanyrootcauses,thenresource/FD/cache/socketaudit.

CPU-native1 actual0/1/0: all15CPUobservations/directSIGexits passed, then active
caller-file alias correctly refused simultaneous control using same probe inode.
Fix fixture with distinct exact-byte caller probe copies per control; keep guard.
cleanup-boxes[]/Hostreaped. This changes test source -> freeze2/rebuild Box2 and
client; OCI production unchanged, retains019provenance. Source1native results
are calibration. AF2all5PASS186.52s, oldulimits1actual1/0/0 207.85s, verify1exit0
289.22s Runtime2240/35ignored CLI934 installer8. CurrentrootRO/ENVchainsmaystill
own resources; let their own cleanup finish, don't cancel/unmount. Nextfmt/
client2/freeze2/buildBox2/verify2 then CPU2+finalnative affectedreplays.

Frozen2ccc6c32ff1d7823cbb6f80aa5e03e799a0f839f170c6844372f5e7341bde77f7,
1693inputs; only CPU native fixture differs from1, all production inputs exact.
Client2eab26b348ef61248e446661f37eb04b3f956b66d4982138190048585975e05f9.
BuildBox2session39953/verify2session12071. IMPORTANT: keepcandidate1run-context
untiloldprocesschain90655finishes; newbuildusesrun-context-final2.json, later
finalnative2explicitlyusesit. HostService1wrappercaughtzero-tests due extra
namespace separator; fixedselectorandadded--list membershipassert beforelaunch.
NoHost/workloadstartedthere;preservefailedreceipt/notcounted. ENV1passed1/0/0
265.16s+Guest8; exactRAMcleanup0/0. RootRO1/tmpfs1stillinprocesschain.
Nextidentities2thenCPU2/finalprocess+service2/AFnewabsolutecontext3; initial
casesremaincalibrationonly. No externalblock/noearlyfinal.

Box2archive57d38d06f5d637b4e2127b1e8967bab37e3a94c83922426e9f616e2e8413d15e
23,390,475compressed/44,154,880installed/11hashes; identities2PASS. OCIunchanged
542bfe43… /03a62c3e… reusedfreshinstalled. FinalnativeCPU2session9887;process
chain2session25528;ENV+HostService2session65758;AF3session98184;verify2active
12071. Alluseimmutable run-context-final2.json; mainrun-context.json still1for
oldprocesschain90655 untilitfinishes. FinalauditpreparedexpectsexacttwoownedAF
newbuilds: calibrationBox1AF2 andfinalBox2AF3, eachwithownCLIhash+resultdigest.
DoNOTrerunoldsource1globalassertionsnow. Afterfinalcounts:auditallcurrentand
failed/preparedhomes/RAM/FD/socket/cache, finalreview/OUTCOME/docs, close020then
nextW4localmemoryprocess-limitgap. No externalblockingcurrent020.

Initialprocesschain1hasfinishedPASS(all3),so mainrun-context.json promotedto
final2; immutablecandidate1copyretained. Finalsource2verifyexit0 237.00s
Runtime2240/35ignored CLI934 installer8 (parseactualcountsnext). AF3all5passed,
ulimits2actual1/0/0passed. CPU2threewaveobservations15written; fullCLI/control/
cleanupstillpending. ENV2andwholeHostService2/processrootRO2/tmpfs2active.
Auditreadsimmutablefinal2context. Nextactualresultreviewthenallhome/socket/FD/
RAM/cacheaudit (includingfailuresandbothAFbuilds),closureandnextlocalW4.

CPU-native2final actual1/0/0 PASS330.69s, all15measuredprocessobservations,
threegenerations1/2/3, realPTY3, directsoft152/hard137, replayoneeffect, stale
fence/refusal, CLIrestart, omitted+infinity2.5CPU-secondpositivecontrols and
independentexecution1:2unchangedpassed. cleanup[]/ownedHostexited. Source2
verify2Runtime2240/35ignored,CLI934/installer8; AF3all5PASS175.70s; ENV2PASS
260.83s+actualGuest8andRAMumount0/detach0. StillrootRO2/tmpfs2/wholeHostService2
active; do not claimglobal020acceptanceorcleanupyet. Nextfinishthose/audit/review.

Allfinal2nativechainsPASS:CPU1test;ulimits/rootRO/tmpfs3tests;ENV1test+Guest8;
wholeHostService1test;AF3fivecases(2Rust+3Python). Acceptedcountpendingaudit:
8outerRust+8LinuxGuestunits+3Python; verify2exit0. Nativechainprocess2exit0
668.40s/service2exit0 648.35s; allownedworkersstoppedbycasecleanup. Planned
20existing/preparedhomesauditincludesfailedCPU1+zeroHostService1andbothAF2/3.
Nextresource/FD/socket/RAM/cache/runtime-diskauditthenfinalreceiptidentityreview.

Final owned audit PASS1.94s and final receipt review PASS1.48s; all20homes/
208sockets/23native+255AFbaselinefiles checked, no residual runtime disks orFDs.
Both native repositories clean, index unstaged. OUTCOME is authoritative; do not
rerun020globalfrozen2assertions after021sourcechanges. No active020resource/job.
Next immediately021 memory process-limit gap; no external blocker for local work.
