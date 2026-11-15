# Spec018 tasks

Current status: bounded phase accepted. Final sourcee1aa0d07…; installed Box3
9ad22c54… /OCI3 3331cbc9…;14 native Rust tests +3 Python scenarios passed,
verify3 passed2238/934/installer8;16-home/134-socket audit passed.
Authoritative outcome: docs/evidence/runtime/20261115-oci-secrets/OUTCOME.md.
Next active phase019 managed Secret environment; continuous task continues.

- [x] T001 Read accepted017/W4 and actual Secret/file/RAM chain; preserve work.
- [x] T002 Before regressions and required bounded memory-file contract.
- [x] T003 Shared RAM backing validation,SDK/Host/shim/capability and Box callers.
- [x] T004 Focused/owning checks; freeze/rebuild/install/native matrix and counts.
- [x] T005 Verify,owned audit/docs/evidence; continue next executable W4 gap.

Accepted017 source8c687c06… Boxc61f0a5a… OCI016c7db5a96… original3ac0fbd8…;
21homes/184absentsockets/0ownedresources/7baselineRAMunchanged. Do not redo017.
Currentpreflightoci_production707rejectsmanaged_secret_root. hvf_files hardlinks
only to runtime_share.with_extension(files),crossFS returnsUnsupported withcopy
forbidden. SDKFileGrantsv1 autoCaller/DetachOnly storage; mark_secret_mount would
conflict. Reuse existing attach_secret_extension for a new required memory-file
mechanism,retaining exactFileGrantsv1 bytes/semantics. Agent file protocol stays
unchanged unless actual tests prove otherwise. Existing Mac RAM validation in
kbox_contract_driver/secret.rs uses mount+hdiutil and a PID-named plist file;
move/reuse into actual Host adapter and avoid concurrent tempfile collision.
Source mode/owner preserved; native runs actual owner for0400 files. Existing
Secret environment manifest is not consumed by OCI Agent; keep explicit refusal
and separate next slice. No external block; no early final.

Implementation checkpoint: SDKMemorySecretFiles required extension v1 binds
ordered unique indices to existing exact RO FileGrantsv1 and Secret extension
classification; malformed/unsupported/old Host before tests actual0/2/0.
SDKafter1 actual2pass; platform shared RAM checker after1 actual3pass (one
existing profile memory test); Box/OCI all-target check1 exit0. Moved actual
RAM provenance intoKociKbox,Boxreusesit; unique stdin plutil replaces PID-named
plist race. Existing mount-line assertions remain via shared helper alias.
Shim validates source+projection RAM before Seatbelt; existing pinned FD and
device/inode reverify retains backing after confinement (cannot spawn mount/
hdiutil subprocess inside Seatbelt). No new Agent mounter/manifest.
Host advertises required memory extension; Boxcapabilitypreflight+RAMroot/RO
file admission and completion annotations implemented; environment manifest
explicitly refused. Native and final assets NOT YET run/built for018.
Focusedafter2 Box1pass/1fail; diagnostic1 actual0/1 confirms fixture was0755
and failed correct privacy gate before memory check. Fixture now supplies an
explicit0700 child root,retaining assertions and original logs. Needrerun.
Next: finishfocused/Clippy,addnativeownedRAM Secret file test (actualowner/
privilegedHostRO controls,restart/stopped/compat/cleanup),buildallaffected
Box+Host+Shim+Agent/systemimage archives; isolatedinstall/actual native. Full
017stillaccepted; sourceevolvesonlyfor018. No external block,no earlyfinal.

Critical compatibility correction: old shim validates known file descriptors but
does NOT recompute attachment classification digest. Required Host extension
alone cannot make a mixed new-Host/old-shim safe. Protocolbefore1 actual0/1/0
proved memory manifest stillv6; implemented explicit newv8,validated iffmemory
annotation present; ordinaryv6 retained. Newv8 uses same file descriptors and
existing Agent mounter; old shim/Agent parser rejectsunknownv8. Raw images plus
MemorySecretFiles refuse; Boxplans anonymouscombination refusal beforeallocation.
Focusedafter3 passed Runtime2/SDK3 (zero-test targets notnative). Nativepositive
Rust test and genericnegativeoldHost/oldShim/diskprojectiontest added; initial
nativeclientready1 compiled beforev8,notfinal. Needrebuildclient. Clippy1 exit0
beforelatestnative/protocol edits; runagainbeforefreeze. No018nativeexecution
orRAMmountyet. Nextprotocolowningresults,finalfocused/clippy,freeze1/buildBox+
Host+shim+Agent+systemimage,archiveinstall and realRAMcases. DoNOTfinal.

Frozen1 6b0a7a2…1688inputs; Box1c4e00e57…23,384,844compressed/44,154,880
installed; OCI1e1dc2e04…33,364,378/114,892,800,all11hashes and source/signature/
architecture/embeddedAgentidentitychecks passed. Nativeclient1 13c56514… .
Actualpositive1 0/1/0: synthetic identity omitted requiredsha256: prefix;
corrected to actualdigest_component contract. Layerinventory proves sha256sum
/usr/bin/sha256sum (not/bin); stat/bin/stat. Nativefixturepaths corrected.
Allprodinputs unchanged; candidate1 mustremain historical andfinalsource2
archives mustbe regenerated aftercalibration passes. Positive1 exactRAM
/dev/disk13 created512MiB,Hostreaped,0ownedprocesses,umount0/detach0; noleak.
Activejobs: files1 session89927,ulimits1 session20243 usingcandidate1 assets;
owning-tests1 session26730 andverify1 session93682. Candidate1oldprovenance
checks retained; do not call final acceptance on repaired source. Nextrebuild
nativeclient3→positive2calibrationwithsameproductionarchives,thennegative
oldHost/oldShim/diskprojectioncases. No external block; continue.

Calibrationpositive2 actual0/1/0: HFS512MiB RAMcannot create4GiB logical ext4
(ftruncate ENOSPC),allownedresourcesreaped/umount0/detach0. Thisisnotan
externalblock. Actualowned APFSRAMprobe: physical/dev/disk13→synthdisk14→
volume/dev/disk14s1,512MiB backing;4GiBlogical sparsefileallocated0bytes.
Probeumount/detachsuccessful,allrecordschecks/apfs-probe1. ExtendsharedRAM
validatorusingactualdiskutilAPFSvolume→allPhysicalStores,eachmustexactly
matchhdiutilram:// device; rejectempty,mixedphysical/RAM andwrongvolume.
ExistingHFSRAMsemanticskeepdirectcheck. APFSwrapperseparatepreservesHFS
failures. ProductKbox/BoxRAMvalidationchanged,sofreeze2/rebuildbotharchives
andrelatednativechecksrequired. Nativev8/SDKordinarycontractsunchanged.
Candidate1files1actual1/0/0 46.59s,ulimits1actual1/0/0 187.11s;owning1
SDK220/protocol67/Runtime438+2ignored/Kbox82+4ignoredexit0in497.43s.
Verify1exit0 453.59s Box2238+33ignored CLI934 installer8. Product/source1
receiptsretained,notfinalnewsourceevidence. Nextplatformafter2/nativeclient4,
freeze2/buildBox2+OCI2/APFSpositive3; negativeoldHost/oldShim/diskprojection.
NoactiveownedRAMafterprobe. DoNOTfinalwhilelocalworkremains.

SharedplatformhelpermovedtoSDK (stdlib+existingserde_json only),Kboxreexports
forBoxcompatibility. HostRuntimeusesthesameprobeoffasync thread; publishes
MemorySecretFilescapabilityonlyifactualruntime share isRAM-backed and records
explicitcapabilityevidenceotherwise. Thisavoidsfalsephysical-rootadvertisement
and Runtime→native-Kboxdependency. AllactualShimchecksremainbeforeSeatbelt,
pinnedidentitychecksafter. APFSexactvolume→allRAMstoreslogicunit3passed in
SDK; missingKboxmoduledeclarationfrommove causedready4/Clippy3compilefailure,
fixedactualSDKreexportandfmt(noassertionchange,noNativepassclaimed).
Currentjobs ready5 session20955,Clippy4 session84763. NoactiveRAM. Sourcehas
changedafterfrozen1; doNOTreusefrozen1snapshotassertions orclaimcandidate1final.
Nextfreeze2andbuildBox2/OCI2(allHost+Shim+Agent/systemimage),APFSnative3 then
oldHost/oldShim/diskrootcapabilitynegativecases and fullaffectedmatrices.

Source2frozen8b2df684b2864f52227acf172aa9b370d6383cbed3d7e4918446b0dfc24d9106
1688inputs; frozenclient2 7dee441dca6650b1fe7dde8109c13ef8ba95ad59da1ddb7ec8e56bb5f18e4468.
Ready5 exit0/Clippy4 exit0. SDKmovedhelperunit3passed; formerKboxModule
removedonlyforournewtaskfiles andsnapshottedinartifacts/shared-memory-before-move.
HostRuntimeprobesRAMoffasync threadbeforepublishingMemorycap;evidenceexplicit
available/unavailable. APFSnativewrappercreated butnotrunyet. NoownedactiveRAM.
ActivebuildOCIcandidate2 session58308; Box2/verify2/owning2 justlaunched (see
checks/*.log and tool session status). NextwhenbuildcompletepackageOCI2 then
check-final-identities2,run-memory-native-apfs.py context3positive,thennegative
cases andcurrentartifactAF/file/named/anonymous/rootRO/tmpfs/ulimitsregressions.
Keepcandidate1failures/archives andallScope17 acceptance unchanged; nofinal.

NewOCI2archived/installed bd3ac1a02d31ae89dcdaccf902cbc9ebf0ed9037117452e8cfdd983cee185b25
33,372,506compressed/114,892,800installed/11hashes,source2 8b2df684… .
The combined identity check was invoked tooearly whileBox2stillbuilding and
correctlyrefusedBox1source6b…; noNativequalificationclaimed. WaitforBox2
installationreceipt beforecheck-final-identities2. OCI2packing succeeded and
itsreceiptretained; rerunonlytheidentitycheckafterbothdependenciesready.
Currentlongjobs Box2 session22118,verify2 session4875,owning2 session36167.
R18af-final2cacheisolatedandhashed; AFcontextstillmustbecreatedfromreadyBox2.
NoownedRAMcurrently. ContinueAPFSpositive3frombothinstalled2archivesafter
identitycheck,negativecases,allaffectedregressions,cleanup/evidence.

Newnative3APFSactual0/1/0: owner501initpassed,capturedexec defaultroot0
correctlyfailedreading400 file dueALLdrop. ExistingNone→rootlegacycontract
retained; nativeownerreads nowexplicitSomeownerUIDallpaths(notpermission
widening). Positive3activecleanup0/umount0/detach0; nativecleanup-boxes[].
OldHost1actual1/0/0 1.83s.Source/markerpreserved andexactRAMdevice12cleaned.
OldShim1correctlyv8rejected(vm_enteredfalse),buttestgenericcleanupfailedon
protectedunboundKilling; historicalrecordarchived,ephemeralRAMdevice14
reclaimedafterHostexit,noactiveRuntimeprocess. Notalabeledpass. Nativefixture
nowreusesSpec009pending::drainwithnewcompatibleassets,nopolicyweakening,
zeroaddedStartreceiptsbeforecleanup. AddedtestonlyHost::start_with_assets.
Source2productionunchanged,testsourceschanged; finalsource3archivesmustbe
regeneratedaftercalibrationpasses. Verify2exit0 387.21s;owning2exit0 466.71s
SDK223/protocol67/Runtime438+2ignored/Kbox80+4ignored. AFcontextfinal2ready
butAFnotlaunched. NocurrentownedRAM. Nextrebuildnativeclient6/calibrate
positive4+oldShim2+oldAgent/diskRoot; onlythensealarchivesandfullregressions.

Currentcalibration3 source583429c8ca40c250ba67b184f4248e7904807440c75c3d536ff9101deee229c2;clientbc7bebcaf73145d725aaa63e76c9ee24252e233f15e0cb25094fff4fd53e7ead.
Onlytestcaller/Hostfixtureedits afterfrozen2; prod2 source8b2df684… intact.
Runningpositive4 session53207,oldShim2 session97784; disk-rootcapability and
old-Agentcasesjustlaunched viaAPFSwrapper. ActiveownedRAM belongs to these
exactcases; donotdetachuntiltheirrunnercleanup. Physical+syntheticdevice/UUID
identitiesareeach checks/secret-*/ram-ownership.json. Compatibilityrecovery
usesnewmanifest+newshim viaK_BOX_TEST_HVF_RECOVERY_* andexistingpendingdrain;
negativeexactmarkerassertions andprotectedunboundkill/removeassertionsremain.
AFcache/contextreadybutnoAFcaseexecutedyet; remainingfullnative rootRO/tmpfs/
named/anonymous/files/ulimits finalmatrixnotstartedwithlatestprod2assets.
Beforefinalacceptance freeze/sealcurrenttestsourcesandregenerateaffected
archives(actualsource+companionhashes),nativeacceptanceandcleanup/evidence.
No externalblock,noearlyfinal; continueW4after018closure.

Recovery checkpoint (current): positive4 actual1/0/0 218.91s with prod2/clientbc7…
passed allthree generations/CLI/Hostdenials and exactRAMumount0/detach0.
OldShim2 Rust1/0/0 passed,outerPythonloader failed on legitimate restored new
shim; failedreceipt retained. Post-child audit proved emptyBox/pendingcleanup
zeroStart/zeroFDholders/noownedprocesses,then exactAPFS UUID+physicalRAM
identityverified andumount/detach0 (post-child-cleanup.json). Strictloader now
binds to workerexecutable path; newrecoveryprefixonly allowed afterhost-2.pid.
OldShim3 completewrapperPASS 36.51s with actual1/0/0 andRAMcleanup0.
Diskroot1 actual1/0/0 3.55s; OldAgent1 Rust1/0/0 40.42s but outerproofFAIL: old
Gueststderr is notavailable inconsole (worker usesstderr null). No exactv8
oldAgentclaimfromgenerictransportfailure. Added test-only directretainedoldELF
probe in actualcurrentnativeGuest reading actualv8manifest; mustobserve exact
schema rejection andremoveonlyits newGuestmount. Ready7 compile active10546;
then positive5 calibration,retainallsource2archives; freeze/rebuildfinal3 after
calibration settles. AF andfullaffectednative matricesstillpending. No external
block,noearlyfinal; close018andcontinuetoSecret-envnextW4slice.

Frozen3 sourcee1aa0d07bea7f853f81ff91e7f9cad778035a132c765cfab17bb3a4adca15585
1688inputs; clientbad58865dc903703ad11372c7e4e86b5bc7b6d32a080d47b6f95613b7e72ad65.
Onlytwo native testfiles changed sinceprod2; no productionbehaviorchange.
Positive6 actualoldELFprobe succeeded: oldAgent3f3f16a7… rejected actualv8
manifest with explicitunsupported-schema stderr/exit1 inrealHVF Linuxarm64
Guest; temporaryGuestmountremoved. Fullpositive6stillrunning76142. Building
finalBox3/OCI3 andverify3 next; then installidentity/positive7+compatnegatives,
files/named/anonymous/tmpfs/rootRO/ulimits andAF5/finalownedresourceaudit.

Final3 archives installed andidentitypassed (checks/final-identities-3.json):
Box9ad22c544c91a835ba78ee88b197ae90b2a512ba651985f0d1f1f3dba6a0925c,
23,384,813compressed/44,154,880installed; OCI3331cbc98ba351465226fe42d960015fcb90e1013069fbb32532a21ef99a0bdf,
33,372,500/114,892,800; bothsourcee1aa… and11companionhashes. FirstBox3
wrapperfailedon stale dist literalafterbuildsuccess,beforecopy; fixedfresh3b
wrapper/receipts preserveallcandidate2assetsandfailedwrapper. Positive6
calibrationactual1/0/0 218.30s including oldELFv8explicitrejection,allRAMclean.
Verify3exit0 210.76s: Box2238/33ignored,CLI934,installer8. AFfinal3allfive
passed151.80s withactualprewarm/directory1/0/0each. Currentpositive7session3292,
storagematrix61775 (files3/named3passed,anonymousactive),processmatrix48676
(rootROactive),compatibility99566. Alluseactualinstalled3assets. Cachebaseline
23native/213AFfiles includesmutableindex: exactolderindexbytes recoveredfrom
unchangedR17cachewithbaselinehashverification. Observedonlylast_used/path
changes foroldrefs,plusoneactualAFbuiltimage(recorddigest/resultmatches).
Keepallotherbaselinefilesstrictlyunchanged. FinalauditpreparedbutNOTexecuted;
receipt review/OUTCOME/docs/closurepending,thenimmediately019Secret-env.

Latest checkpoint: finalpositive7 actual1/0/0 228.53s, all4compatnegativecases
actual1/0/0 (oldHost1.96s/oldShim37.79s/oldAgent41.27s/diskroot2.44s), allRAM
umount0/detach0. AFall5PASS,files/named/rootRO/tmpfs passed; anonymous/rawRO/
ulimits remain running in matrix sessions61775/48676. Verify3passed2238+934+
installer8. WorkingOUTCOMEexplicitlyOPEN; finalaudit/review scripts prepared,
not run untilallmatrixchildrenfinish. Sharedmemorychecker callers verified
canonical at BoxSecretroot, PreparedUtilityVmLayout and pinned MacosFile
attachment; fileidentity/normalization guards remain (no additional feature
or hypothetical policy edit). Next finalnativecounts→audit/review→close018
docs→immediately019. No externalblock.

Final closure: all7affectednativecases1/0/0 each, all5Secretcases1/0/0, AF5
passed (2Rust1/0/0 +3Python); finalreview3 PASS checks actualsource/archives/
counts/results/clean native repos/indexempty. Audit2 extendsfirst13homeaudit
to16exactpreparedhomes (three never created boxes.json), retainsaudit1,
134observedsocketsabsent/0ownedresourcesorFDs/7initialRAMunchanged, all23+213
cachefileschecked, indexonlyknownmetadata+exactAFbuildentry; no blobs changed.
OUTCOME/spec/W4/owningdocs/releaseupdated. No externalblock for019; beginit
fromexisting Agent executor secret_env consumer, file metadata and actualcallers.
