# Plan047

Use existing CLI test_registry fixture with daemon boot limiter0. Poll actual
get_or_create once; its registry must already hold the pool while first fill is
pending. Cancel, retry and assert SAME owner, real key-lock serialization and
truthful drain. No VM/network in baseline. Current Runtime startup locally creates
Self then fill_to_target().await(unit) before returning; only CLI production caller
uses first-ready constructor, CRI calls shown are tests.

Factor existing validation/construction into public synchronous prepare method;
private borrowed initialize reuses replenish_handle mutex through fill and installs
background handle only once. Public borrowed first-ready wrapper returns closed
admission error. Compatibility constructors return their actual owner after the
same fill intent, preserving prior API semantics. CLI retains entry under pools
lock before calling borrowed init; existing entries also complete/retry init under
existing key lock. Registry draining vs publication remains atomic under map lock.

Native Runtime fixture prepares owned Arc BEFORE initializer, holds stats until
actual readyVM completion is registered, cancels fill, preserves exact PID/volume/
Guestwrites then actualdrain/noLate. SharedproductionCLI composition has hermetic
registry proof, not a duplicated startup model. Fullchecks/freeze/archive/install/
AF/pool/owned audit; all authority/typederror/noextrastate/support boundaries hold.
