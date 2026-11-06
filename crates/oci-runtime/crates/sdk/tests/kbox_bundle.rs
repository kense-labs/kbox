use std::path::PathBuf;

use koci_sdk::{OciBundle, OciSemanticPhase};

const K_BOX_CONFIG: &str = include_str!("../../../fixtures/kbox/config.json");
const K_BOX_ROOTFS: &str = "/var/lib/kbox/boxes/box-123/rootfs";

#[test]
fn loads_the_exact_kbox_compiler_output() {
    let bundle_directory = std::env::current_dir()
        .expect("current directory")
        .join("kbox-fixture");
    let bundle = OciBundle::from_json(bundle_directory, K_BOX_CONFIG)
        .expect("Kbox Box compiler output must remain a valid OCI bundle");
    bundle
        .validate_for_phase(OciSemanticPhase::Start)
        .expect("Kbox Box bundle must remain start-valid");

    assert_eq!(
        bundle.spec().root().as_ref().expect("root").path(),
        &PathBuf::from(K_BOX_ROOTFS)
    );
    let process = bundle.spec().process().as_ref().expect("process");
    assert_eq!(
        process.args().as_deref(),
        Some(["/sbin/init".to_string()].as_slice())
    );
    assert!(process
        .env()
        .as_deref()
        .expect("environment")
        .iter()
        .any(|entry| entry == "K_EXEC_LISTENER_FD=3"));

    let linux = bundle.spec().linux().as_ref().expect("Linux configuration");
    assert_eq!(
        linux.uid_mappings().as_deref().expect("UID mappings").len(),
        1
    );
    assert_eq!(
        linux.gid_mappings().as_deref().expect("GID mappings").len(),
        1
    );
    assert_eq!(
        linux
            .cgroups_path()
            .as_deref()
            .expect("cgroups path")
            .to_string_lossy(),
        "kbox/box-123"
    );
    assert!(linux.resources().is_some());
    assert!(linux.seccomp().is_some());
    assert_eq!(linux.devices().as_deref().expect("devices").len(), 6);
}
