# Extend the existing per-UID mapping

Core already parses msgqueue/Linux12; Agent already implements RLIMIT_MSGQUEUE.
Pinned Linux6.12.76 aarch64 config enables POSIX_MQUEUE, IPC_NS and USER_NS. The
primary ipc/mqueue.c charges capacity+kernel structures to real-UID ucounts at
create and releases it on inode eviction, not on message receive or unlink
alone. Native checks must distinguish byte-budget EMFILE from RLIMIT_NOFILE
and per-queue EAGAIN. mq syscalls use the IPC namespace's internal filesystem;
no host or workload mqueue mount is required by the probe.

Callers: vm/oci_hvf_rlimits.rs (static validation/completion and every session
entry), existing CLI configured_process_limit_user (capture/health/PTY/attach).
Reuse that guard and map standard OCI enum; nproc-only compatibility capability
and manifestv10 remain unchanged. Tests reuse existing owning siblings and one
native-harness sibling with bounded static C syscall probe. Configuration,
exports, SDK/Host/Agent transport and dependency sets remain unchanged.

First capture admission/completion/CLI red counts. Then implement shared guard
extension and caller match; focused owning checks/fmt/Clippy/verify. Freeze Box
source/client/archive, fresh isolated install, unchanged OCI022 extraction with
original provenance/source proof and signatures/architectures/loader identities.
Run actual mq effects/all paths/recovery/controls/CLI then affected per-UID/
limits/Host/file/Secret/A–F regressions. Every loop has a bound; children reaped,
every created queue unlinked and descriptor closed, only owned resources cleaned.
Retain failures, exact counts/results, source/dependency hashes, sizes and audit.

Constitution Check: same ExecutionManager authority/fencing, closed typed input,
fail closed, KCL only/one workspace/local APIs, no native source edits/new state,
actual native PEP effects and retained evidence. No exception needed. Default
routes/support/readiness/live continuity and external platform gates unchanged.

Final: source1/Box1/freshunchangedOCI022 accepted. MQ2 and allaffectednative
regressions/AF/Secret/Guestcounts plusverify1 and strict15homeauditpass. First
namespaceMQprobe lackedGIDmapping and gotEOVERFLOW75; C2mapsGIDaftersetgroups
deny, assertsgetgid0 thennewIPC; all15actuallyenterednamespaceobservations
retainexactEMFILE24. C1/source/diagnosticremain. NoRust/productassetchangefrom
thisexternalfixturefix; seeOUTCOMEforactualcounts,identitiesandcleanup.
