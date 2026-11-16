# Plan044

Extract the existing legacy provider-start block without behavior change to give
its two callers one actual boundary; reproduce cancellation using real handler
return and held destination mutex, preserving failure/owned fixture cleanup.
Reuse pending_handler, reject repeated acquisition while pending and preserve
both actual owners on destination conflict. Default finalization and boot-failure
cleanup install pending before deleting resources; cache cannot hide pending work.
Maintenance reuses shared start with existing error cleanup. Selected original-
owned acquisition/scope semantics stay intact; from_pid reattachment remains
separate because it has no fresh provider acquisition. No new lifecycle state.

Native controlled return of actual installed-companion VM handler must preserve
PID/allocation/live writes after cancellation, with owned rescue and real final
stop/no late writes. Clarify controlled handoff vs fresh native provider launch.
Full owning/default/Clippy/fmt/source/archive/AF/pool/audit; all typed/authority/
evidence/native/no-deps/gitlinks/support/no-weakening constitution rules retained.
No waiver; continue other local gaps after bounded acceptance.
