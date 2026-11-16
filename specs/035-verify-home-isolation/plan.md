# Plan035

Establish real script subprocess red with inherited and unset K_HOME using fake
tools confined to a test TempDir. Add a minimal private-root allocation/export at
script entry and preserve paths/results for audit; include the isolation check
itself in the gate. Do not disable existing checks or allow a skip flag.
Then real default gate with explicit recorded owned home, exact counts/failures,
source/archive equality and known fixture-path audit. Append correction of prior
unqualified default-test home claims, update docs/tasks and commit normally.
No real user-home modification, including attempted cleanup of past side effects.
