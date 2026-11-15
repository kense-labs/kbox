# Spec011 tasks

- [x] Read task/rules/owning source and accepted009/010; preserve dirty source, artifacts and mount baseline.
- [x] T001 Fix current named-volume refusal with a small positive regression; retain actual before failure. Trace shared store/attach/storage-ID/cleanup callers.
- [x] T002 Implement exact named admission/classification/lifetime and negative cases in Box owning paths; preserve existing ordinary bind IDs/routes/generations.
- [x] T003 Test RO/RW/live/shared lifecycle, malformed catalog/paths/aliases and failure cleanup; run focused/owning gates, Clippy/fmt/workspace and verify-commit.
- [x] T004 Freeze/rebuild affected archives, isolate-install and actual named + prior file/rootfs/Host/A–F native matrices; retain counts, identities and failures.
- [x] T005 Audit only owned resources/caller data; update evidence/ledger/docs and immediately continue W4 local work.

Context: docs/evidence/runtime/20261114-oci-named-volumes/run-context.json.
Initial checkpoint (superseded below): HVF profile denies policy.volume_names. Existing VolumeStore
and ExecutionResourceGuard provide attach/detach; existing storage helper is
only unit-tested/unused by production. Need prove its validation is adequate
before wiring. Catalog corruption fallback/paths/driver/quota are entry-boundary
risks to examine; no broadened admission or implementation yet. Anonymous/Secret
remain separate, not marked supported. Spec010 final Box2/OCI7 remains accepted.


Before baselines: named profile0pass/1fail; storage helper0pass/3fail (mutable
name reused as storage ID across recreation, corrupt catalog moved/quarantined,
unenforced driver admitted); VolumeStore0pass/3fail (../caller materialized
outside volume root, poisoned catalog removal deleted only the owned synthetic
canary fixture, corrupt catalog replaced by new metadata). All logs retained;
no user directory/data touched. The native aliases/case current guard test passed,
so no additional platform identity adapter is needed or added.

Implementation in progress: VolumeStore normal get/list/prune/mutations strict
and preserve bad catalog; explicit legacy load() recovery remains for old API
compatibility. Single-component names checked before IO; roots reject aliases;
create rejects unsupported driver and existing allocation aliases; catalog
saves reuse fs_atomic::write_durable. Forty existing/new volume tests passed
first run. Ordinary host scope/volume refs remain existing manager authority.

Named SDK identity is derived from name + allocation created_at + canonical
source, not the mutable name selector; recreation changes it, normal retry is
stable. No new persisted metadata fields/layout or resource state machine.
Existing source identity and generation-bound Host manifests remain the PEP.
The shared storage helper is now wired only for Mac named mounts; ordinary bind
IDs unchanged. Preflight validates declared named mounts/catalog/driver/quota;
legacy rootfs/file/BoxVm remain. Old storage test8pass/1fail asserted raw nameID;
strengthened it to require opaque ID, not mutable name, plus exact retry stability
while retaining access assertions; new recreation regression separately covers
incarnation change. First named profile positive passes. Focused after2running.

Next add/execute owned actual named native lifecycle with installedOCI7 to test
RO/RW/live/restart/shared refs/cleanup and actualCLI. A strict Host-update-after-
Guest-first-read may expose existing directory caching; do not assume it passes.
Only then fix any observed shared cause, run full owning gates, freeze/rebuild
all affected Box/OCI assets, final native/regressions/cleanup/evidence and continue.


Prototype native1 actual failure0/1/0: independent installedOCI7 accepted named
RO/RW, manifest actual identities pinned and HostRO probe14denials+child passed.
Guest first read5bytes; Host grew live file; immediate Guest read returned only
"host " (5bytes), exactly oldcachedsize. named-live-observation.json retains
expected/actual, nativeworkerloader and cleanup[] retained. Both explicitly
created test volumes retained data after runtimecleanup, then test owner removed
only its named fixture allocations; named-fixture-cleanup.json confirms0volumes.

Fixed shared macOS shim cause: caller directory exports now use existing native
uncached API (already selected/pinned for file grants); runtime control share
retains its existing path. No native submodule/FW/kernel change. Need newOCI
Host/shim candidate from current source +actualnative1repeat, no silentAPI fallback.

Identity follow-up reproduced0/1/0: remove+create(previousConfig) reused timestamp
and same storageID. Existing allocation metadata label mechanism now issues a
store-owned UUIDv4 on every new materialization; copied config gets freshUUID.
with_write_lock preserves existingallocationUUID; malformed/duplicate identities
refuse. New SDK IDs bind UUID; preexisting no-label catalogs use a deterministic
validated creation identity without rewriting their metadata. No new persisted
fields/layout/resource state; old label maps retain the UUID on roundtrip.
VolumeStore44 focusedtests nowpass, storagebeforeclone/newNonce tests retained.
Source-specific Aliasafter1passed using existingcanonical guard; no newplatform
filesystem identity adapter/dependency was added. Next newcacheOCI candidate,
current Boxnativeclient+prototype2; actualfinalCLI remainsrequired in newarchives.


Source/implementation checkpoint: normalized trailing separator name/ was
reproducedadmitted0/1/0; single-component guard now explicitly rejects slash as
well as backslash/colon/NUL, retaining Unicode/space names. SharedVStore normal
get/list/prune/changes are strict; explicitlegacy load recovery API is retained.
NonceV4 protected labels are canonical/unique and cannot be overwritten/dropped
through mutation. Copied config on newmaterialization gets newUUID; preexisting
no-label catalogs are delegated stably without metadata rewrite. No new schema
fields/layout/state machine. Named helper filtersoneexactbindperdeclaredvolume;
missing/duplicate/declarationdrift testedbeforemutation. Otherprofilelimits stay.

After5 namedfocused tests passed (actualcountlog authoritative); owning koci-kbox
all-targets pass (75lib/3ignored,18binary/3ignored,3integration; see log), noNative
passclaimedforthoseignoredentries. Newuncached directory native source tested
by real before1 failure; existingruntimecontrolshare unchanged. Built+signedOCI
candidate1 fromcurrent source, systemimage built; source assertionpassed. Candidate
sourceSHA saved checks/oci-named-candidate1-source.json, current source verified.
Native2 running via run-named-prototype2.py (session24363);
verify-commit-named1 and then owningkoci-kboxClippy queued (session76084).
NeedfinalnewBox/OCIarchives +CLIrequirednative replay afterprototypedebugpasses.


Checkpoint: Native2 completed1pass/0fail/0ignored in148.52s using frozen
prototype client0e5c199f and signedcandidate1. Three HostRO runs, live data,
shared allocation/references, normalrestart and stoppedSIGKILL recovery passed.
cliChecked=false: accepted010 CLI lacks this feature; no finalCLI claim yet.
Source b5e73458 candidate retained; exact loader receipts/cleanup[] are saved.

verify-commit-named1 failed101: Box2201pass/4fail/20ignored. Shared contract
volume resolver compared exact label count and rejected new store-issued UUID;
fix recognizes only ALLOCATION_ID_LABEL in addition to exact contractlabels,
not arbitrary metadata. Added positive retry plus unknown/conflictinglabel
regression retaining rawcatalogbytes on refusal. Focusedartifacttests running;
next wholeverify+owningOCIclippy, freeze/rebuildBox/OCI, requireNamedCli=true
installed native+full files/rootfs/failedCreate/A–F, then cleanup/evidence and
next W4. No externalcondition currently blocks this localstage.


Post-caller-fix: contract artifact tests8passed/0failed/0ignored. Finalsource1
07ccdf500f00db25334d538ebed0dd1bcf10dd11e3dfaf417ca45f79cad2ef35;
freeze1 and candidate2 source snapshots match. New OCI final1 archive built,
unpacked into fresh ownprefix, 11files hashchecked, signatures/architectures
checked. SHA5f9578d20cf4db524b7fb83223443f83d0aca257688370b854772d191c478288.
Box final1 build running session56050; verify2/owningClippy session24544.
Nativeclient frozen1 SHA61f08158ca1aa103adec836e0b711744ef3a49aa69c5a772a17c312470cb201b.
Do not edit frozen Rust without rebuilding affected assets and redoing tests.
Next: Box installed + bothassetidentityaudit, named attempt3(cliCheckedrequired),
fullmatrix attempt4, failedCreate5, AFfive; all ownresourcescleared. Then close11
and continue W4; currentartifactCLIparent10 remainsforprototypeonly.


Finalgates: verify-commit-named2 exit0(218.65s), Box2206passed/20ignored,
CLI933passed; all contract/workspace/fmt/Clippy/installer8tests passed, with
actual perbinary summaries in checks/actual-counts-source1.json (do not sum
child duplicate executions). Owningkoci-kboxClippy exit0(3.73s). Box final1
archive d4209a7ff4048690770089126c0026bfc09abe0cc26d5b074992cf8fc83595cc,
compressed23344608bytes, installed44081152bytes; OCI final1 compressed33334546,
installed114851840bytes. Bothsource07cc... and includedAgent verified.

AFfinal1 allfive actualinstallednative scenarios PASS; prewarm/directory each
1/0/0 Rustentry, otherthree oneactualscenario each. Loaderproofs retained.
Named final3 stillrunning(cli+Hostrecovery); fullfinal4running; failedCreate5
started. Pendingcleanup/audit/references/finalOUTCOME+ledger; then nextW4task.


Installednamed final3 completed1/0/0,163.51s; cliChecked=true, three HostRO
repetitions plus child/remount/freshmount, liveHostgrowth, guestwriteback,
sharedrefs/retry/IDs/restart/oldgeneration, reconstructedoff-policyclient and
stoppedHostSIGKILLrecovery allpass. Runtimecleanup kept3namedvolume data/metadata
and removedonlyexecutionrefs. Explicitfixtureowner subsequently removedonlythe
3knownvolumes; finalcleanup0. failedCreate5 completed1/0/0,39.82s; installedCLI
unboundkillguard, exactSDKCreate/delete recovery with0workloadStarts addedpassed.
Fullfinal4 stillrunning firstfile/rootfs matrix (session58550). No failure or
externalblock currently. Await actualfullresult before finalallresourceaudit.


Final checkpoint: fullnative4 passed1/0/0 in377.26s; cliChecked=true. Five
file-probe receipts16Hostdenials each; prior rootfs/private-disk/route/Exec/
resource/fault/Hostrecovery assertions allpassed. Scoped audit PASS:11homes,
98observedsockets allabsent,8immutable sourceblobs unchanged,0executions/volumes/
ownedruntimeprocesses/mounts/disks/descriptors. Initial7RAM/unrelatedmountedimage
preserved, auditdeletednothing. Named runtime keptdata/metadata first; onlyfixture
owner thenremovedits3explicitvolumes. Parentnativeclean/gitlinksunchanged,index
empty; sourcefreeze07cc... intact. OUTCOME and owning/release/ledgerupdated.
Next bounded local W4 item: image-declared anonymous VolumeStore allocations;
continue immediately with spec012 instead of ending at this milestone.
