# Retained failures and actual corrections

No failed logs, candidate archives or assertions were removed.

- Runtime admission/spec before1: 0 passed / 2 failed. The Mac preflight refused all ulimits; completion retained its unrelated default NOFILE1024:1024. Added bounded nofile/fsize/core mapping through existing OCI process.rlimits, preserving the actual default and no-override bytes.
- Shim before1: 0 passed / 1 failed. Invalid soft>hard was silently forwarded. The shared typed Core parser now refuses malformed/unknown/duplicate/overflow values. Legacy recognized names and valid ordering remain.
- Legacy installed BoxVm final1: captured exec read back nofile64:128 and core0:0 but raised hard NOFILE128→129 successfully, exit51. This assertion remains. Two separately owned diagnostics confirmed actual CapEff/Prm/Bnd=000001ffffffffff,NoNewPrivs0,Seccomp0 and actual kernel hard129; guest /sbin/init matched installed f2b6d327… . Both instances were removed with zero executions/volumes.
- Root cause: CLI execute_captured,exec PTY,attach PTY and health probes routed only OciSdk records through LocalExecutionManager; managed BoxVm used raw Guest sockets without the manager's existing security inheritance or generation fencing. New focused regression before1 executed1 and failed on BoxVm. The minimal fix routes both persisted managed backends through existing sessions; unclaimed historical socket behavior and logs are not promoted. Focused CLI commands after1:691 passed /0 failed; final candidate2 rebuild/native replay pending.
- An initial run-check invocation incorrectly supplied run-context as a command label and failed before child execution because its resulting log parent did not exist. Corrected the invocation; no test ran and no pass is claimed from that error.

Commands/counts in checks/; owned diagnostics and command stdout/stderr under the run-context privateRoot. Candidate1 archive and source inventory remain in artifacts; candidate2 has distinct paths and source identity.

- Final receipt review2 failed before writing its summary: the checker compared an installation directory with an exact argv element containing its bin/kbox child. Review3 requires the exact installed CLI executable (or the exact frozen native test executable) in argv. Native scenarios were already passing and were not rerun; the failed review receipt is retained.
