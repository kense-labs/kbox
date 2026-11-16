# Plan038

Trace BoundPoolListener::bind, serve, pool_start and ensure_pool_daemon_running.
Fix shared initial binding and existing flock adapter, preserving caller paths
and adding no deps/config/state machine. Use distinct existing-role sidecar
locks, exact no-follow regular single-link same-UID validation, live connection
refusal and same-identity receipt-bound stale unlink; hold daemon lock through final cleanup.
Keep existing owned shutdown cleanup and pre-warm bind ordering. Blocking stale
probe must be bounded/off async executor. Lock replacement is refused and kept.

Constitution: I/V fail-closed io::ErrorKind; II actual listener/VM owners unchanged;
III raw failures plus OUTCOME retained; IV filesystem hermetic plus installed Mac
native; VI/VII one workspace/KCL/no dependency; IX no secrets/caller file mutation;
X actual bytes/identity evidence; XII native gitlinks unchanged; XIV no support
promotion; XV existing assertions/default gate retained. No principle waiver.

Sequence: red filesystem/endpoint test; shared caller fix and adversarial tests;
CLI-full/Clippy/fmt/workspace/default; freeze/rebuild/install affected archive;
real CLI/native regressions and task-only resource audit; release/W4 evidence,
source/diff/commit closeout then continue existing local owner gaps.
