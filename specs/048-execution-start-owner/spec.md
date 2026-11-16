# Spec048: original starts survive request cancellation

047 native first-start timeouts left Starting/PIDNone; automatic recovery safely
refused missing original identity. Original delay cause remains unknown. Fix the
proven shared boundary: a start already admitted under its existing lifecycle
lock must retain its same actual operation until durable completion, even when
its caller stops awaiting. Duplicate start/inspect/stop must wait for that owner;
cancellation before admission starts nothing. Reuse existing inspect's committed
operation pattern; no new registry/state/config/permissions/dependencies.

Preserve generation fencing, exact physical handler and resource authority.
Completing an admitted start is not an explicit workload cancel/stop; stop still
serializes and physically terminates it. Runtime/process shutdown mid-operation,
provider-inner acquisitions and original cold-start delay are NOT solved by
merely retaining a Tokio task; keep those separate and unclaimed.

Acceptance: actual shared API red/green with pending original start, cancelled
caller/waiter/pre-admission, replay/inspect/stop serialization and stale generation.
Owning/default/CLI/Clippy, current source-bound archive/client, native same actual
VM/identity/no second boot/durable handle/stop/remove/noLate, AF/pool/owned cleanup.
Default disk/other platform/Git blocks recorded, no green GA/whole W4 claim.
