# Shared intent and existing OCI setrlimit executor

Trace CLIcommon.resource_limits → immutable Box request → preflight/
oci_hvf_spec completion → standard Process.rlimits → publicSDK/Host → existing
Agent RlimitPlan setrlimit/getrlimit. Also launch_process in oci_session builds
a new process and must receive stored limits for captured,stream andPTY. Guest
exec does not inherit limits from its independent Agent parent automatically.
Legacy shim parser maps names to guestLinux IDs; Core already owns ResourceLimits.
Move/reuse its closed name map and validate bounded numbers once. Preservevalid
legacy behavior; invalid input must not be logged then silently omitted.

Actual before test shows OCI ProcessBuilder defaults to NOFILE1024:1024.
No-override calls must not mutate any field (including explicit None); declared
overrides replace only their resource and retain unmentioned existing defaults. Existing public
SDK fingerprints complete Exec request; test nonempty values,replay and changed
request conflicts without introducing a new receipt/state system. Pids cgroup
control unchanged; other Mac resources remain explicitly unqualified.
Constitution I/II/V/X/XV failclosed/oneauthority/effects/unchangedgates,III/IV
retained actualnative evidence,VI/VII no new systems,XII/XIV native boundaries/
no platform promotion. No waiver.

Native legacy regression additionally required all managed CLI process entry
points (captured exec,PTY,attachPTY,health) to use the existing manager. This
fix removes the raw-socket policy bypass without a second policy or route
change. Actual final installed capability masks,spoof refusal,init limits and
PTY hard-limit denial are retained in spec017 evidence.
