use kbox_core::config::ExecutionIsolation;
use kbox_runtime::hardware_vm::{
    open_vm_only, select_isolation, stored_profile_bytes, ContractError, IsolationSelection,
    VmOnlyFeature,
};

use super::resolve_isolation;

#[test]
fn omitted_isolation_selects_hardware_vm_and_legacy_selectors_are_rejected() {
    assert_eq!(resolve_isolation(None), ExecutionIsolation::Microvm);

    let live = std::env::var("K_BOX_OCI_MIGRATION").ok();
    let baseline = stored_profile_bytes(None);
    assert_eq!(stored_profile_bytes(live.as_deref()), baseline);
    for migration in [None, Some("off"), Some("microvm"), Some("all")] {
        assert_eq!(stored_profile_bytes(migration), baseline);
        assert_eq!(
            select_isolation(None, migration),
            Ok(IsolationSelection::HardwareVm)
        );
        assert_eq!(
            select_isolation(Some("microvm"), migration),
            Err(ContractError::MicrovmSpellingRejected)
        );
        assert_eq!(
            select_isolation(Some("sandbox"), migration),
            Err(ContractError::UnknownIsolation)
        );
        for feature in [
            VmOnlyFeature::Profile,
            VmOnlyFeature::Snapshot,
            VmOnlyFeature::ControlChannel,
        ] {
            assert_eq!(
                open_vm_only(IsolationSelection::HardwareVm, feature),
                Ok(())
            );
        }
    }
}
