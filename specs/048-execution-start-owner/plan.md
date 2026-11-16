# Plan048

Trace ExecutionManager start callers and driver's bounded startup; manager start
currently drops its same lifecycle guard and backend.start future on cancellation,
whereas inspect already clones its manager into a committed Tokio operation after
acquiring that guard. Keep start pre-admission cancellable and move only admitted
read/validation/ensure_started into the same existing pattern. Typed JoinError;
no timeout-triggered kill of a different original request, no PID scan fallback.
Reuse inspection_cancellation_tests backend/gates for deterministic early replay
and explicit stop; test current source red before shared API repair. Keep original
cold timeout and failed047 homes/artifacts, not relabeled fixed. Native wrapper
may delay return from real backend start, never supply fake VM/OS failure; same
registered handler remains authority and product stop must succeed on PASS.
