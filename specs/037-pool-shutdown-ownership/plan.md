# Plan037

Trace bulk drain/source closure, registry leases and request accounting/Stop ACK.
Establish swallowed-stop-error/owner-loss red. Reuse VmManager as resource
authority; collections hold actual handles, no duplicate lifecycle metadata.
Serialize each manager's finalization, retain it before asynchronous teardown,
and remove only after success. Propagate cleanup failures to the existing Stop
response; exclude Stop itself from the in-flight barrier, close admission before
waiting, unlink listener only after successful owned drain. Keep retry available.
Verify positive close and refusal/cancellation controls, rebuild affected frozen
assets, install into fresh prefix and perform actual Mac native validation/audit.
Commit only accepted task files; Git network/account failure is parked, continue
remaining actual local owner gaps after this phase.
