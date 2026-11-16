# Plan045

First extract existing cold implementation without behavior change into real
boot_cold_vm helper. Existing VmManager/fake actual handler/state-lock fixture
polls then drops it before native/layout work; assert actual handler notdropped,
registered owner/admission/data and typed stop-refusal retry. No assertion cuts.
Then register before template/boot waits; return existing CompletedBoot from
cold/restore producer and adapt boot_new_vm+BootBatch (no double registration).
Safe prelaunch/actual completed cleanup may finalize/remove exact owner while
retaining original error; unclosed failure retains/close admission. A small getter
may expose existing completed_cleanup observation (not BoxState alone/newstate).
Restore uses destroy_one once and propagates close refusal before fallback.

Native wrapper delegates actual VmController.start; oneshot entry gate is BEFORE
physical start, so no returned raw handler is held across a new fixture await.
Caller locks destination after entry; proceeds to actual start; synchronous PID
notification precedes actual return/044 pending store. Only then cancel worker;
actual registered manager persists, release destination, verify Guest writes and
exact created allocation, product drain/noLate/cleanup. Failure keeps typed actual
owner; never intentionally run orphaning baseline with a real VM.
Constitution typedfailclosed/actualauthority/evidence/native/noextra-state/deps/
workspaces/gitlinks/support/noassertionweakening retained, no waiver. Finish
checks/assets/audit/currentdocs/authorized commit/bundle then next local gap.
