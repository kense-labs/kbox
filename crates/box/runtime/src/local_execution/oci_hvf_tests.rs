use super::*;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "oci_hvf_native.rs"]
mod native;

#[test]
fn hvf_explicit_endpoint_and_creation_policy_are_separate() {
    let temporary = tempfile::tempdir().unwrap();
    let service = temporary.path().canonicalize().unwrap().join("host");
    let endpoint = service.join("runtime.sock");
    let active = parse(None, None, Some(endpoint.clone().into_os_string()))
        .unwrap()
        .unwrap();
    assert_eq!(active.creation_policy, OciMigrationPolicy::MicrovmViaOci);
    assert_eq!(active.runtime_root(), service.join("runtime"));
    let off = parse(Some("off".into()), None, Some(endpoint.into_os_string()))
        .unwrap()
        .unwrap();
    assert_eq!(off.creation_policy, OciMigrationPolicy::LegacyOnly);
    assert_eq!(off.endpoint(), active.endpoint());
    assert!(parse(None, None, None).unwrap().is_none());
    assert!(parse(Some("off".into()), None, None).unwrap().is_none());
}

#[test]
fn hvf_missing_or_wrong_endpoint_and_unknown_modes_refuse() {
    for mode in ["microvm", "all", "sandbox", "typo"] {
        assert!(parse(Some(mode.into()), None, None).is_err(), "{mode}");
    }
    let temporary = tempfile::tempdir().unwrap();
    let service = temporary.path().canonicalize().unwrap().join("host");
    let runtime = service.join("runtime");
    let endpoint = service.join("runtime.sock");
    for (root, endpoint) in [
        (std::path::PathBuf::from("relative"), endpoint.clone()),
        (service.join("other/runtime"), endpoint.clone()),
        (
            runtime.clone(),
            std::path::PathBuf::from("relative/runtime.sock"),
        ),
        (runtime.clone(), service.join("../host/runtime.sock")),
        (runtime, service.join("other.sock")),
    ] {
        assert!(MacosHvfOciMigrationConfig::new(root, endpoint).is_err());
    }
}
