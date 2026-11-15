# Spec017 tasks

- [x] T001 Read closed016/W4 and owning caller/parser/executor chains,preservework.
- [x] T002 Before regressions for Mac admission,dropped init/session rlimits,legacy invalid omission.
- [x] T003 Shared validated parser/callers,Mac subset and static session inheritance/tests.
- [x] T004 Owning checks,freeze/rebuild/install and real native controls/regressions.
- [x] T005 Verify-commit,owned audit/docs/evidence;continue next local W4 gap.

Context docs/evidence/runtime/20261115-oci-ulimits/run-context.json.
Starting accepted016 source3ac0fbd8… Box52418165… OCIc7db5a96…;18homes/107
absentsockets/0resources/7baselineRAMunchanged; do not repeat completedbaseline.
Actual chain: config.rs ResourceLimits.ulimits VecString; shim/main.rs421
parse_ulimit private15names→guestLinuxIDs,vm_launch150 logsunknownandignores;
oci_production Maculimits blanketreject; oci_hvf_spec processno rlimits;
oci_sessionlaunch_process240 newprocess,Mac capabilitiesalreadyfromstoredconfig,
needsrlimitssameway; Agentplan/init+exec_process/helper alreadyRlimitPlanapply
andkernelreadback. PreserveNone/emptyprocessfield forlegacyexecfingerprints.
No017productedit yet. Next before tests thenminimalsharedimplementation.
Noexternalblock; noearlyfinalwhilelocalworkremains.

Checkpoint: Runtimebefore1actual0/2/0 (11.69s) andShim before1actual0/1/0
(5.80s). ActualOCIbuilderdefault isNOFILE1024:1024,notNone; adaptedplan.
SharedCoreGuestRlimit parser/closed15-nameLinuxID map nowownsstrictnumeric
SOFT:HARD/-1infinity,bounds,duplicatesandsoft<=hard. LegacyShimusesvalidated
valuesandrefusesinvalidinputinstead ofwarning+omitting; existingvalidnames
anddefaultNPROC/NOFILEorderedoverride semantics retained. Testoldassertions
callactualCoreparserthroughlocalformatadapterafterprivatemapmove.
Macboundednofile(16..1048576),fsize,core only;otherprofiles/unqualifiednames
refuse. OCIinit+captured/stream/PTYshareapply_workload_rlimits; nooverride
keepsserializedprocessbytes anddefaults,declaredresourcesreplaceonlymatching
defaults. No operationidentityrewrite; publicSDKcompleteprocessfingerprint
alreadybindsrlimits andstaticrequestconfigimmutable.
Focusedafter2PASSCore2/Runtime5/Shim9. ProbeCcross-build1passedzigstatic
aarch64,butnotnativepass. Probegetrlimit/readback+EMFILE24/EFBIG27/EPERM1/
core0+realTTY/stdin; sources/binaryhashes inchecks/probe-identities1.json.
Nativeulimits1running frozenclient1 viaaccepted016installedassets (CLIoff
early),allVMworkersstrictinstalledloader. NativeRustcoversinit+exec+child+
stdin stream+realPTY*3generations,normal/stopped recovery,replayoneeffect/
changedrequestrefusal,default1024control viaexplicitOCIclient. Currentprofiles
qualifyprocesslimits,nottrustedmanagementFileAPI globalfilesystemquota.
Clippy1running. Nextinspectnativeactualresults/fixturerootcauses,LegacyBoxVm
realinheritance+badinputrefusal,owningchecks/freeze/finalBoxrebuild+install/
unchangedOCIarchivefreshprefixoriginal3ac0fbd8provenance/CLItrue/AF+impacted
regressions/verify/ownedcleanup/docs. Noexternalblock; doNOTfinal.

Checkpoint after frozen1 native execution: source90ab8cec… Box9efb365d…
OCI c7db5a96… original3ac0fbd8… productionunchanged. OCIulimitsfinal2
actual1/0/0 in185.20s,CLItrue,kernelcontrolsallpaths/3generations passed;
AF5passed/rootROBox1passed/verify1exit0in250.13s.
LegacyBoxnofile/core readbacks64:128/0:0 passed,buthardraise128→129
unexpectedlysucceeded (legacy-ulimits-final1-1,exit51),testretained.
Owneddiagnostic legacy-cap-diagnostic-final1-1 confirmsCapEff/Prm/Bnd
000001ffffffffff,NoNewPrivs0,Seccomp0 inexec despite storedcap_dropALL.
Actualkernelhardlimit129afterraise; notshelltestambiguity. Bothowned
instancesremoved,0executions/volumes. Rootcause investigationactive: trace
CLIstreamsession securityinheritance→guestlaunch andactualGuestbinary.
DoNOTweakenhardraiseassert; fixsharedpolicygapandregenerateaffectedassets.
Allfailedlogs retained; noexternalblock; stageNOTaccepted,noearlyfinal.

Confirmedrootcause: CLIbypassedmanagerforpersistedBoxVm captured/PTY and
health; attachPTYalso. NoGuestcapsetbug. AddedfocusedCLIbeforeroutecheck
actual0/1/0; reusedexistingmanagerforbothmanagedroutes andalldirectcallers.
AfterCLIcommandsactual691/0/0 passed; noowningsecurityduplicated.
Source2frozen8c687c06d486c721afcc33605d83ec208cd9b92f3a72d7732529ebd2d7ddd736
1682inputs; Boxcandidate2build+Clippy2+verify2running. Guest/OCI/native
productionunchanged; OCIretains3ac…source/c7db…archive. Nextfinal2install
identitycheck→legacyhardrefusal+PTY/restart/badinput→OCI1+AF5+RootRO1,
ownedfinalaudit/receiptcounts/docs thennextlocalW4gap. Noexternalblock.

Final acceptance checkpoint: source2 8c687c06…/Box2 c61f0a5a…; actual
OCI3 1/0/0 176.49s CLItrue; BoxVm limits1/security2/rootRO1; AF5 passed.
Verify2 exit0 222.33s Box2236/31ignored CLI934 installer8. Audit21homes/
184absentsockets/0resources/0FDholders/7baselineRAMunchanged;15+131source
blobfilesunchanged. Final receipt review3 passed actual counts and paths,
source2 unchanged/native trees clean/indexempty. OUTCOME/FAILURES accepted.
Next bounded local W4 task: managed Secret transport/qualification; start018
from actual allocation→attachment→Host memory backing chain. Noexternal
block for investigation; doNOTfinal while executable local W4 gaps remain.
