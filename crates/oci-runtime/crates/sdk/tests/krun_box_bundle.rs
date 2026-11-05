use std::path::PathBuf;

use krun_oci_sdk::{OciBundle, OciSemanticPhase};

const KRUN_BOX_CONFIG: &str = include_str!("../../../fixtures/krun-box/config.json");
const KRUN_BOX_ROOTFS: &str = "/var/lib/krun/boxes/box-123/rootfs";

#[test]
fn loads_the_exact_krun_box_compiler_output() {
    let bundle_directory = std::env::current_dir()
        .expect("current directory")
        .join("krun-box-fixture");
    let bundle = OciBundle::from_json(bundle_directory, KRUN_BOX_CONFIG)
        .expect("Krun Box compiler output must remain a valid OCI bundle");
    bundle
        .validate_for_phase(OciSemanticPhase::Start)
        .expect("Krun Box bundle must remain start-valid");

    assert_eq!(
        bundle.spec().root().as_ref().expect("root").path(),
        &PathBuf::from(KRUN_BOX_ROOTFS)
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
        .any(|entry| entry == "KRUN_EXEC_LISTENER_FD=3"));

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
        "krun-box/box-123"
    );
    assert!(linux.resources().is_some());
    assert!(linux.seccomp().is_some());
    assert_eq!(linux.devices().as_deref().expect("devices").len(), 6);
}
