# Spec013 tasks

- [x] Read accepted012/source/W4; preserve dirty workspace and prior assets.
- [x] T001 Trace init/exec/PTY exact setup and establish before regression.
- [x] T002 Shared fallible readonly guard and all callers, typed error/verification tests.
- [x] T003 Native Guest effect/no-launch tests and impacted lifecycle regressions.
- [x] T004 Owning/workspace/fmt/Clippy/verify; frozen rebuilt installed assets and counts.
- [x] T005 Owned cleanup, final evidence/docs/W4 and continue next local task.

Context: docs/evidence/runtime/20261115-rootfs-readonly/run-context.json.
012 accepted source dada2886…/Box6e2e961d…/OCIdf4b882c… retained. Its final
23homes/117sockets/0ownedresources and allnative matrices already passed.
Current013: only handoff/spec/root inventory created; no product source edited.
Confirmed linux/mounts.rs remount_rootfs_readonly returnsOk after both fail;
exec_server.rs helper returnsvoid on failures; exec_server prepare and PTY ignore.
main.rs propagates wrapper result but wrapper currently lies. Rootfs readonly
stat check also uses unwrap_or(false). Need callable before and all callers.
This safety gap takes priority over planned tmpfs control. Masked/readonly path
guards also use warn/continue; record next scope, do not claim fixed here.


Before checkpoint: added only test/harness code, no productfix yet.
LinuxGuestlib test readonly_rootfs_mount_failure_refuses_exec_preparation calls
real build_command in a self-exec child dropping SYS_ADMIN via existing helper,
requires error before returning a launchable command and a writable temp root
with no marker. Existing prepare currently ignores a void remount failure.
Crossbuild guest-tests-before-build1exit0 (notruntimepass), frozenGuesttestbinary
R13/guest-tests-before1. Nativeclientready1exit0. Actual MacHVFGuest run
run-guest.py ...run-context.json1 running session94534. MustreadactualGueststdout,
error,counts/cleanup beforefix; current_exe sourcegrantmayneedfixturecorrection.
Allnativeworkers use accepted012installedBox/OCIassets. No native codechange.
Next confirmbeforeerrorisadmittedreadonlyroot (notmissingfixture); thenfallible
sharedexistinglibhelper + init/exec/PTY propagation/poststatverification. NeedPTY
framedError/no child test, actualnormalBoxVm readonly/init and cancellation/
retaineddata. Generic masked/readonly pathguards stillwarn/continue recorded;
tmpfsqueuedafterthishigher-prioritygap. Noexternalblock.


Actualbefore1 complete: nativeGuest1 ran0/1/0,34.00s; LinuxGuestparent andchild
failed at "explicit readonly root was admitted after failed mount enforcement".
This is the intended production build_command failure, not a fixture/path error.
Nativecleanup-boxes[] andHostsocketabsent. Allworkers used012installedassets.
Added second realPTY frame regression (noSourcefixyet); before-build2running87361.
Needfreeze newGuestlibtestbinary fromJSON/libtarget andrun-guest attempt2; require
PTYErrorframewithread-only reason beforefork, notgenericTTYsetup failure/EOF.
Then publicfallible existingexecrootguard; mainwrapperreuse; exec typedfailure
output; PTYframedrefusal. Poststatverify andnormal/idempotent readonly case.


Before2actualnative0/1/0,33.93s. Both realLinuxGuest exec/PTY parentsfailed;
PTY child specifically reached the forbidden continuation after un-enforced
readonly root, not a missingdevice/fixture error. Bothownedrunscleaned[]/socketgone.
Productfixnowreuses one fallible exec_server::remount_rootfs_readonly for init,
exec andPTY; invalid/missing/nondirectory targets error; stat failures propagate;
bind/remount errors propagate; poststat mustconfirmST_RDONLY. Initcalls? before
spawn; exec returns failureOutput; PTY writes Errorframe thenreturnsbeforeopenpty.
Nochanges to default/backend/HostROpolicies. linux/mounts wrapper lost divergent
best-effort implementation. Positive/idempotent/fullinitnative stillpending.

Afterbuild2crosscompiled; actualNativeGuest3passed1/0/0,34.73s. Guestparents3/0/2
plus two explicitlyinvokedchild1/0/0: realexec/PTYrefusal andpathvalidationpassed.
Ownhosttests Mac156/0/0 (Linux-onlycases excluded), hostClippy1exit0. LinuxClippy1
failed missingaarch64-linux-musl-gcc; retainedlog. Reusinginstalledcargo-zigbuild
CC/CXX/ARwrappers (cache0.23.4) forLinuxClippy2, session86792 wasoldfailed;current
newsession recordedtool. No externalblock andno newdependency installed.
Next closeinit-specificfailure proof/normalreadonly-idempotent-writablesubmount
nativecases, thenowning/verify/finalBoxassetbuild+installednative/A–F/cleanup.
OCIproductioncode unchanged from012; may reuse immutableacceptedOCIarchivewith
explicitprior sourceprovenance and fresh isolatedextract rather than fabricate
newsourcebinding. No13finalasset oracceptanceclaimyet. W4tmpfsstillqueued.

Checkpoint resumed: NativeGuest4 passed1/0/0 (37.75s), Guest3/0/4 plus two child1/0/0 and three explicit1/0/0 (alreadyRO/enforcement/init). Verify1 exit0 250.51s; LinuxClippy3 exit0. Source frozen1 87966c0e77c51d463f8a1e31c7847002f3ef38a63ce1993257ad85472a948a4b. Preserved global and accepted012 archives+checksums. OCI production inputs unchanged; reuse accepted012 immutable archive with original provenance, freshly extracted under R13 (11 checksums). Next rebuild Box, frozen source-current Guest/client binaries, normal installed BoxVm readonly/init/exec/PTY/restart and affected A–F; owned cleanup. No external blocker.

Finalcheckpoint: Boxfinal1 ee2c103c56675350b28db18c6c0f7627a417095bbd1d281ac1ab63d824e0a9d5 built/installed11hashes. Normal BoxVm readonly native2 passed1scenario (3.15s), init/exec/PTY rootEROFS, innerRW, restart ID preserved/gen1→2/tmpfs cleared/caller retained. Attempt1 diagnostic backend field was wrong, actual runtime_route asserted in2; product untouched; attempt1 cleaned. FinalGuest5 uses frozen source1 binaries/newBox+fresh OCI12, actual1/0/0 in39.29s; LinuxGuest3/0/4 plus2child1/0/0 and3 explicit1/0/0. A–Ffinal1 running session4174 (OCI/HostRO passed; primary running); no13acceptance beforeall5 and finalowned audit. Source unchanged87966c0…; no externalblock.

CLOSED013: final Boxee2c103c…/source87966c0…/unchangedOCI12df4b882c… accepted. FinalGuest5 1/0/0, normalBoxVm2 1scenario, A–F5/5, verify0. Final12homes/82sockets/0resources+descriptors,7priorRAM/15main+91AFblobfiles unchanged. OUTCOME.md has exact results and no claims beyond slice. Continuous task NOTfinished. Proceed014 confirmed path-restriction fail-open before queuedtmpfs; no external blocker.
