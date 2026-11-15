# Spec015 tasks

- [x] T001 Read accepted014/W4/owning paths; preserve source/archive/evidence.
- [x] T002 Fix before regression for dropped tmpfs spec and current rejection.
- [x] T003 Shared parser/platform adaptation + preflight/spec/callers/override plan/tests.
- [x] T004 Owning/fmt/Clippy/verify and actual final installed native acceptance.
- [x] T005 Exact owned cleanup/docs/evidence; continue next local W4 item.

Context docs/evidence/runtime/20261115-oci-tmpfs/run-context.json.
Starting accepted014 source8f562b…/Boxc6eb7199…/OCIdf4b882c…;24homes/118sockets/
0resources andallfinalmatrices already closed, do not redo. Known chain: preflight
oci_production.rs735 rejects alltmpfs; vm/oci_hvf_spec::complete doesn'tmap it;
Agent executor/mount.rs already supports it. Core guest_exec exists; Guest parser
in linux/mounts.rs629 handles ro/rw separately but other genericflags are passed
as kernel data. Need preserveexistinglegacy options and actualflag semantics.
No15productedit yet. Next addfocused failing spec/preflight, thenimplement.
Noexternalblock. Neverfinalwhilelocalworkremains.

Checkpoint: focused-after3 actual Core2 + Runtime16 passed. before-admission1
actual2failed; spec-before1 ran0tests andisnotbeforeproof; before2actual1failed.
Shared Core GuestTmpfsMount parses bounded path/options andgenericflagpairs,
legacyGuest mapsflagsviaexistingnix andpreservesadvancedkerneldata. Macadapter
closeddata(size/mode/UID/GID), target overlapandbounds; OCI specappends standard
tmpfsafterindexedstorage; planner honors normalizedtmpfsoverrides. Addedactual
Host nativeprobe (flags/EROFS/ENOSPC/noexec/numericowner/isolation/restart/recovery/
CLI/noanonymous/caller+namedretention), nowrunningtmpfs-attempt1 withfrozenclient1
andaccepted014assets forcalibration. Hostclippy1running. Earlycompilefailures
privatehelper/OciImageConfigDefault/testConflictpattern correctedwithlogsretained.
Next inspectactualprobe,fixrootcauses, native Guest/BoxVmflagcalibration, owning
checks,sourcefreeze/build/install/AF+storage+HostROregressions/cleanup. Noexternalblock.

Calibration tmpfs-attempt1 actual0/1/0 (71.69s), cleanupempty: realtmpfs/RO
EROFS/quotaENOSPC/noexecEACCES/uidgid/isolation passed first. Failureat
retainedwrite: cap_dropALL Guestroot0cannotwriteHost501-owned0755directories.
Useactualcaller501:20 ownerforretainedwrite, preservingfixturepermissions,
notweakenassertion/product. Beforefixtureeffectlogs retained. LinuxClippy1
foundtwoaccidentalwide-log replacements inexistingvolumemounter; restoredonly
those(nonbehavior), rerunrequired. HostClippy1passed23.48s. nativeclient3building.

Checkpoint: native tmpfs-attempt2 actual1/0/0,138.89s; gen1→2→3/
Host SIGKILL stopped recovery all passed; per-execution isolation, empty tmpfs
aftereachnewVM, caller/namedretention,nohiddenimageallocations. CLI intentionally
false inthiscalibration(old014CLI hasno015mapping); finalinstallwillrequiretrue.
Core482+8integration, Guesthost156 passed. LinuxClippy2passed2.46s; hostfmtpassed.
Sourcefrozen1 31aeb63a32b8b388fe5526eddf9f3172eb3c577656fd8e1b6342309a17fbcb16
1674inputs; Boxpackagefinal1running. OldBoxc6eb7199andchecksumspreserved.
OCI12df4b882c archive freshlyextractedand11hashesverifiedwithoriginaldada...
provenance(retainunchangedOCIproductioninputs, nofalsefreshsourceclaim).
Next finalBoxinstall/identities; immutablefinalnativeclient; requireCLI native
tmpfs3, actualBoxVminitflag+rootRO, AF5/storage/fileHostRO, verify-commit, owned
audit andreceiptreview. acceptedfalse; continuousW4stillactive.

Final archive1 installed 1f81ad37106ed31275027bd5c135a1b9f72bcfd755d5af252d58e75dac9e1f30
23,370,310 compressed/44,122,112 installed,11hashes. Finalidentities1passed;
GuestInitf2b6d327…; finalclient919b2bbf…. Finaltmpfs3 actual1/0/0,163.07s,
CLIcheckedtrue, actualsameIDgen1→2→3. AF5allpassed(total175.67s), installed
BoxVmrootRO+tmpfsflags/noexec/PTY/restart passed1scenario6.79s.
verify-commit1passed261.49s Box2228/28ignored CLI933 installer8, contracts/
workspace/fmt/scopedClippypassed. ActualLinuxGuestbin tmpfsunit2/0/0 passed
via finalinstalledBoxVm (guest-linux-final1-1,1.33s), no crosscompile-as-native.
FinalfileHostPEP1 actual1/0/0 42.07s; named1 actual1/0/0 162.01s inclCLI/
threeHostROwaves/restart/recovery; anonymous1runningbroadownedlifecycle.
Next waitactualanonymouscount/result, inventoryallownedhomes/socket/process/
mount/disk/FD, final-resource-audit1 andreceipt-review1; updateOUTCOME/release/
W4thencontinue016 boundedOCIrootreadOnlyintent. NoRustchangedafterfreeze.

CLOSED: anonymous1 actual1/0/0,421.59s withCLItrue; finalresourceaudit1
PASS13homes/106absentsockets/0resources/7baselineRAMpreserved/15main+111AF
immutableblobfilesunchanged. finalreceiptreview1PASS counts,actualresults,all
installedorigins,3mainworkerssameIDgen1→2→3,sourcefrozenunchanged,nativetrees
clean,indexempty,0FDholders. Noauditdeletion. OUTCOMEaccepted; allfailuresretained.
Continue016OCIrootread_onlyintent usingexistingGuestexecutor; noexternalblock.
Do NOT final: otherlocalW4workremains. Historical015proofmustnotberevalidated
againstlaterchanged016source.

016evidenceindexrepair: reviewJSONnamecollidedwithrun-checkreceipt; original
stdoutPASS/count/sourceobjectrecoveredasfinal-receipt-review1-summary.json.
No test/sourcecheckrerunagainstlatercode; script-hashfieldnotfabricated.
Originalreceipt/logskept;accepted015behaviorproofunchanged.
