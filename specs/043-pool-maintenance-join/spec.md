# Spec043: cancelled maintenance join remains owned

Both drain methods currently take the actual maintenance JoinHandle before
awaiting it. Cancelling that await detaches the unfinished worker and loses
the join owner; retry can return before actual maintenance cleanup. Preserve
the existing handle in its mutex until completion, shared by both public callers.
No new lifecycle/config/dependency/workspace/OS killer or route/generation change.

Acceptance: controlled real task/production-drain cancellation red then green
for both public variants, retry remains pending until task exits, existing
assertions retained. Actual installed VM with registered maintenance owner/PID/
allocation keeps writing while cancelled/retrying join, then physical worker
cleanup and drained result/noLatewrites. Fullchecks/rebuiltfrozenarchive/AF/pool/
audit/currentevidence, authorized localcommit/verifiedbundle, continue localgap.
Internalboot/unpublished/legacy/fullOCIcutover/W4/GA/externalplatforms/RSS open.
