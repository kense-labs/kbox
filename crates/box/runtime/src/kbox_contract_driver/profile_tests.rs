use kbox_contract::contract::{IsolationLevel, RuntimeUnitClass};
use kbox_core::{ExecutionBoundary, SeccompMode, SecurityConfig};
use sha2::{Digest, Sha256};

use super::mapping::creation_request;
use super::profile::NATIVE_GUEST_PROFILE_DIGEST;
use super::test_support::runtime_spec;

#[test]
fn native_guest_profile_identity_pins_its_security_semantics() {
    let definition = b"kbox.native-guest.v1\nboundary=microvm\nseccomp=unconfined\ncapabilities=all-guest\nno-new-privileges=false\nresources=explicit\n";
    assert_eq!(
        format!("sha256:{:x}", Sha256::digest(definition)),
        NATIVE_GUEST_PROFILE_DIGEST
    );
}

#[test]
fn ordinary_oci_contract_keeps_its_security_defaults_without_acp() {
    let spec = runtime_spec("ordinary-service", 1, RuntimeUnitClass::Service);
    let request = creation_request(&spec, ExecutionBoundary::Microvm).unwrap();
    assert_eq!(request.config.cap_drop, ["ALL"]);
    assert_eq!(request.config.security_opt, ["no-new-privileges"]);
    assert!(!request.config.privileged);
}

#[test]
fn explicit_native_guest_keeps_resources_and_mounts_without_extra_guest_security_filters() {
    let mut spec = runtime_spec("native-service", 1, RuntimeUnitClass::Service);
    let ordinary = creation_request(&spec, ExecutionBoundary::Microvm).unwrap();
    spec.semantics_profile_digest = Some(NATIVE_GUEST_PROFILE_DIGEST.into());
    let native = creation_request(&spec, ExecutionBoundary::Microvm).unwrap();
    assert_eq!(
        serde_json::to_value(&native.config.resource_limits).unwrap(),
        serde_json::to_value(&ordinary.config.resource_limits).unwrap()
    );
    assert_eq!(native.config.volumes, ordinary.config.volumes);
    assert_eq!(native.config.host_services, ordinary.config.host_services);
    let security = SecurityConfig::from_options(
        &native.config.security_opt,
        &native.config.cap_add,
        &native.config.cap_drop,
        native.config.privileged,
    );
    assert_eq!(security.seccomp, SeccompMode::Unconfined);
    assert_eq!(security.cap_add, ["ALL"]);
    assert!(!security.no_new_privileges);
    spec.isolation = IsolationLevel::Process;
    assert!(creation_request(&spec, ExecutionBoundary::Microvm).is_err());
}
