# Spec039: foreground pool owner handoff

Owning path: Runtime pool/warm_pool.rs, existing CLI callers and tests. Published
foreground idle acquire/release moves the actual VmManager into unregistered
locals across health/stat waits and finalization; failure/cancellation can discard
the handler. Retain actual handles in the existing pool collection before every
async foreground handoff point. Only synchronously transfer the exact owner to
a caller or idle membership after authenticated health and open admission; a
concurrent drain/finalizer prevents transfer. Stop refusal is returned and closes
admission, keeps data/actual owner for exact retry; no ambient-home orphan reap
in these converted foreground paths. Counters count successful operations.

Hermetic real-manager/drop/data/cancel/retry/transfer tests first establish red.
Affected final archive/fresh install/native actual VM refusal/retry/pool/AF and
owned audit must qualify source. ExecutionManager/generation/route/defaults and
private templates/endpoint receipts unchanged; no deps/config/state machine.

Scope is published foreground transfer (including already-returned cold results
waiting for stats); allocation/boot_or_restore/template/batch cancellation and
legacy CLI source lifetimes remain explicit continuing work. No global manager
Drop reaper, because intentional detached ownership exists. Other-host, native
snapshot, actual OS stop-error, fresh-host/full W4/GA/RSS are not claimed.

Cancellation also closes existing admission through a scope guard referencing
only the actual owner weakly; successful transfer/finalization leaves admission
open. Maintenance cleanup selects its own inputs and cannot sweep in-flight
foreground owners; explicit shutdown still drains all retained handles. Actual
continued writes under deliberate cancellation/stop refusal are reported as live,
with no late writes proved only after real backend finalization.
