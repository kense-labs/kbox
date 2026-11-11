use kbox_contract::contract::{IsolationLevel, RuntimeUnitSpec};
use kbox_contract::{RuntimeError, RuntimeResult};

/// Explicit native Linux guest policy. It grants no additional host mounts or services.
pub const NATIVE_GUEST_PROFILE_DIGEST: &str =
    "sha256:daf902bf6663b433b3d9a97dffefc5044661decf3b872916209348a7e439baeb";

pub(super) fn native_guest_requested(spec: &RuntimeUnitSpec) -> bool {
    spec.semantics_profile_digest.as_deref() == Some(NATIVE_GUEST_PROFILE_DIGEST)
}

pub(super) fn validate_profile(spec: &RuntimeUnitSpec) -> RuntimeResult<()> {
    if native_guest_requested(spec) && spec.isolation != IsolationLevel::Sandbox {
        return Err(RuntimeError::UnsupportedCapabilities(vec![
            "profile:native-guest requires an independent-kernel Sandbox".into(),
        ]));
    }
    Ok(())
}
