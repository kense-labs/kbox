# Spec046: closed boot admission after queued template wait

Published cold helper currently ignores shutdown after queued template-resource
wait, and restore may start/fallback after template preparation despite closure.
Check existing watch at entry and after template wait, BEFORE a new template
consumer or VM boot. Release/finalize only exact never-started prepared manager;
preserve typed refusal/actual owner and avoid fallback after closure. No change
to ExecutionManager boot_failed_before_provider_start semantics, state/config/
deps/workspace/identity/routes/generation/native sources or support levels.

Acceptance: alreadyclosed+queued-shutdown realhelper regressions red then green;
unchanged admission/typedcleanup assertions. Full source/frozen archive/installed
functional realVM control plus separate native prelaunch negative(0 actualVMs,
not a successful VM run), AF/pool/owned audit/current evidence/authorizedcommit/
bundle. Already-admitted in-progress boot, unpublished startup ownership, provider
internal/from_pid/legacy/capacity/fullW4/defaultOCIcutover/GA remain open.
