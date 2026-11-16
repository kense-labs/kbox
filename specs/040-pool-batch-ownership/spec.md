# Spec040: completed pool batch ownership

Owning path: Runtime pool/warm_pool.rs and direct CLI/native consumers. Published
boot-batch producers return successful VmManager values into JoinSet/results
before idle publication. Consumer cancellation can discard those actual owners.
Retain completed managers in the existing pool collection at producer completion
and carry the existing cancellation/admission guard with queued results. Publish
only through exact synchronous transfer after required idle/stat/admission waits;
close failures preserve owners and select only actual pending inputs. No new
execution state/config/dependency/workspace or global VM Drop reaper.

Establish a real native baseline: actual authenticated cold VM completes the
real batch but lacks parent registration. Before deliberately dropping an
unregistered VM, fail the invariant and ask its still-owning worker to finalize
it. After fix, cancel the consumer holding completed results, prove exact actual
VM/identity/live data retained, then real drain/no late writes/owned cleanup.
Hermetic lifetime/refusal/selection controls and installed CLI/AF/pool/native
regressions must accompany current frozen source/archive.

Scope is completed workers/results for an existing published pool. Internal
allocation/boot_or_restore/template cancellation before completion, unpublished
pool lifetime and legacy CLI source ownership remain required continuing work.
No native Linux snapshot/other-host/fresh-host/OS stopfailure/RSS/full W4/GA claim.
