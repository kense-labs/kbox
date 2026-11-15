# Reuse standard OCI process limits

Trace: CLI resource_limits -> stored Box request -> Mac preflight
oci_hvf_spec::validate -> rlimits::limits -> completion/apply_workload_rlimits
for init and oci_session::launch_process for exec/stream/PTY -> public SDK ->
independent HVF Host -> Agent RlimitPlan -> setrlimit/getrlimit before credentials.
The Agent already maps RLIMIT_CPU; only Box admission omits this resource.

First add a focused actual failing CPU admission/completion test. Add the one
resource mapping and explicit zero refusal in the existing Mac helper. Keep
all other unqualified limits refused. Use the existing native limits harness
and a bounded static Linux probe whose parent observes wait4 signals/rusage;
do not substitute process timeouts or readback for enforcement.

Freeze source after focused checks. Rebuild/isolate-install Box. OCI artifacts
may be reused only with an unchanged source-input proof, their original019
provenance, archive/hash retention and a fresh isolated extraction. Native
runtime binaries/libraries must come from those installed archives. Verify
actual executed counts and results, then audit owned resources and close the
bounded stage; continue the next executable W4 item.
