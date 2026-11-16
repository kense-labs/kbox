# Plan041

Trace only remove_idle_vms, fill_to_target and existing owner helpers. Poll real
rollback with idle mutex held; queue a membership mutation behind its first lock
and force the second acquisition to expose stale indices. Fake handlers isolate
the hermetic test but actual working-directory deletion/counters are asserted.
Keep an exact target state lock to make scheduling valid after atomic fix too.

Reuse existing VmManager fixtures/retained-owner closure. Lock idle and stats
before selecting exact current IDs, remove under that same lock and perform
no asynchronous operation before existing destroy_vms registers selected actual
handles. Preserve source data, unrelated targets, errors and cancel semantics.
No duplicate execution state. Native uses real independent allocations/PIDs.

Constitution I/V typedfailclosed; II actualresourceauthority; III rawred/evidence;
IV hermetic+installedMac; VI/VII no newworkspace/deps/config; IX/X actualcanaries/
identity/physicaleffects; XII nativegitlinksunchanged; XIV supportunchanged; XV
oldassertions/defaultfailures retained. No waiver. Finish source-bound checks/
native/audit/references/localcommit/bundle, then next required local source gap.
