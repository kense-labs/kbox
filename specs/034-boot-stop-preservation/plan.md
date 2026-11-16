# Plan034

Reuse normal destroy's fail-closed handler restoration/early return in the shared
failed-boot cleanup. Release the take() lock guard before restoration/awaits.
Retain bounded diagnostics on refusal. Start with small permanent failure,
retry-success and already-exited finalizer tests, covering working data, exact
anonymous allocations and unchanged state/identity. Existing successful cleanup
and terminal-status tests stay. Then actual installed real VM plus transparent
one-shot error injection, rebuild/native regressions/full gate/audit/evidence and
local Git convergence. The outer pool lifetime issue is required next work and
must not be hidden by a successful inner-manager test. External gates parked.
