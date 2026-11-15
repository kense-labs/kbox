# Reuse existing resource contracts and executor

Core guest_rlimit already parses both types and maps Linux14/15. Mac Box
vm/oci_hvf_rlimits is the missing admission/completion mapping. OCI Agent
executor/rlimit already applies and reads back both before actual init/exec
workload entry. Reuse that single path for capture, stream and PTY; no new CLI
user default, per-UID budget, scheduler service, schema or state is needed.
Scope: Box helper, its existing spec/production test siblings, native module
registration and one native sibling, static C syscall/CPU probe and CLI driver.

Retain primary pinned Linux6.12.76 scheduler/RT/CPU-timer sources and actual
kernel configuration. Establish red tests before two standard mappings. Verify
exact values/defaults/idempotence and side-effect-free security/platform refusal.
Prove actual priority, errno, signals/CPU and controls in an isolated Guest.

Freeze current sources/client, retain old archives, rebuild Box and freshly
install it and unchanged OCI022 into isolated prefixes. Check all companions,
architectures/signatures/native/Agent identities and actual loaded paths. Run
affected native matrix/Host/files/A–F plus default verify. Keep unrelated024/025
results under their original identities when their code/assets are unchanged;
record exact source delta instead of relabelling historical tests.

Audit only owned homes/processes/VMs/mounts/disks/FDs/sockets/cache metadata and
baseline RAM. Retain every failed probe/candidate. Close OUTCOME and release/
ledger references only after actual counters/results/cleanup/source checks.

Constitution Check: one ExecutionManager authority and persisted route/fencing;
closed typed input/KCL/one workspace, actual PEP evidence/fail-closed boundaries,
clean native submodules, no support/default promotion. No waiver or new deps.

Final:actual5Rustnative+3Python frominstalled2, no new Guest unit run;
verify2259/42ignored CLI935installer8 and12-home/93-socket strictcleanupPASS.
C1 encountered installed musl scheduler stubs; C2 calls actual Linux syscalls
with identical effects/errno/assertions. Readiness uses present shell builtin,
not nonexistent/bin/test; that Rustfixture change prompted source/client/Box2
and verify2. Productpayloads1→2 match exceptbuild-info. All failures retained.
See final OUTCOME; immediately continue the existing Secret/raw-volume gap.
