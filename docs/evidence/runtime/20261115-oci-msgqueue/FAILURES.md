# Failures retained

Before1: admission/completion0passed/2failed/0ignored, CLI0passed/1failed.
Actual missing mapping and per-UID CLI match, not zero tests or compilation.
After mapping/admission/CLI change:15passed/1failed/1ignored, CLI1passed.
The only after1 failure is shared test helper extraction retaining hardcoded
expected RlimitSigpending despite an explicit independent Msgqueue argument.
Fix that missed expectation to the caller-supplied enum; both original Sigpending
and Msgqueue test wrappers still assert exact independently specified types and
every original UID/capability/NNP/default/idempotence boundary. No product
behavior/assertion weakened. Initial attempted replacement asserted no match and
wrote nothing; actual multiline source was then read and patched.

No024 native execution yet. Static C build1passed (cross-compile only). Pinned
kernel config POSIX_MQUEUE/IPC_NS/USER_NS enabled; actual support still needs
installed native effect verification. All candidates/logs retained.

Native1actual0/1/0,37.27s wrapper: init probe exits80 before ready file;
FileDownload correctly refuses stopped filesystem. Cleanupboxes[]. Quarantine
record retains actualinitexit80. NeedactualCstderr, not generic file lifecycle
error as root cause. Run a separate installed-archive CLI diagnostic with sleeping
init and captured exec to preserve exact probe stderr/limits/identity, then fix
proven cause. No unsupported/native success claim yet; no test weakened.

Diagnostic1 actualinstalledCLIexec exits80 with probe line45 expectedMQ
EMFILE but errno75(EOVERFLOW); namespace child then reports short pipe. Real
UID1001:2002,capsets0,NNP1 andRLIMIT_MSGQUEUE2048 confirmed independently.
Probe's new user namespace maps UID only; it leaves realGID2002 unmapped before
new IPC inode creation, unlike the prior signal/fork probes with no new inode.
Probe2 maps its own GID after setgroups=deny and asserts getgid0 before IPC;
keeps exact EMFILE24/ordinary-FD-success and every original quota/lifetime/message
assertion. Add gidMapped observation. C1source/ELF/diagnosticremain immutable;
only C2source/ELF regenerate, product source/client/archive/verify unchanged.
Needactualnative2 result to confirm this mapping cause. Diagnostic1 removed
exactexecution,volumes[],hostSIGINTexit0; included in final ownedaudit.

MQ2first/restart/recovery all15observations nowPASS with UID+GIDmapped and
newIPCnamespace actuallyentered; MQcreateEMFILE24 whileordinaryFDopens. This
confirms the original75 came from incomplete namespace credentials, not a
weakened quota expectation. FullpeerUID/lifetime/messagesassertionspassed.
No product source/client/archive/verify change; remainingmain2CLI/zero and
independentregressionmatrixstillmustfinish beforeacceptance.
