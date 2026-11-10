use std::path::Path;

use kbox_contract::contract::RuntimeObservation;
use kbox_core::rootfs_metadata::RUNTIME_ENV_PATH;
use kbox_core::secret::SECRET_ENVIRONMENT_MANIFEST;
use kbox_core::{ExecutionBackend, IsolationClass};

use super::fixture::{BoxRuntimeConformanceFixture, SECRET_ENV_VALUE, SECRET_FILE_VALUE};
use super::{require, Result};

/// Box has exactly one isolation boundary: a hardware-backed MicroVM. There is
/// no shared-kernel security profile, no host-side sandbox OCI bundle, and no
/// `isolation/sandbox` provider build identity to verify.
pub(super) fn verify_provider_least_privilege(
    fixture: &BoxRuntimeConformanceFixture,
    record: &crate::BoxRecord,
    observation: &RuntimeObservation,
) -> Result<()> {
    let metadata = record
        .managed_execution
        .as_ref()
        .ok_or_else(|| super::protocol("security record lost managed metadata"))?;
    let config = &metadata.request.config;
    let expected_plan = kbox_core::resolve_execution(config)
        .map_err(|error| super::external("resolve persisted security execution plan", error))?;
    require(
        metadata.plan == expected_plan,
        "security execution persisted a plan that drifted from its creation intent",
    )?;
    require(
        metadata.plan.backend == ExecutionBackend::Kbox
            && metadata.plan.isolation_class == IsolationClass::HardwareVm
            && metadata.plan.required_controls.is_empty(),
        "security execution did not retain its exact MicroVM backend and boundary",
    )?;
    require(
        !config.privileged
            && config.cap_add.is_empty()
            && exactly_one(&config.cap_drop, "ALL")
            && exactly_one(&config.security_opt, "no-new-privileges")
            && !record.privileged
            && record.cap_add.is_empty()
            && exactly_one(&record.cap_drop, "ALL")
            && exactly_one(&record.security_opt, "no-new-privileges"),
        "security execution did not persist the exact least-privilege policy",
    )?;

    let provider_build = observation
        .provider_build
        .as_deref()
        .ok_or_else(|| super::protocol("security observation omitted provider build identity"))?;
    require(
        microvm_hypervisor_identity(provider_build).is_some(),
        format!("MicroVM provider build identity is incomplete: {provider_build:?}"),
    )?;
    verify_microvm_process_identity(fixture, record)
}

pub(super) fn verify_secret_persistence(
    record: &crate::BoxRecord,
    creation_intent: &[u8],
    boxes: &[u8],
) -> Result<()> {
    verify_no_plaintext("managed creation intent", creation_intent)?;
    verify_no_plaintext("boxes.json", boxes)?;
    verify_microvm_secret_persistence(record)
}

fn exactly_one(values: &[String], expected: &str) -> bool {
    matches!(values, [value] if value == expected)
}

fn verify_microvm_process_identity(
    fixture: &BoxRuntimeConformanceFixture,
    record: &crate::BoxRecord,
) -> Result<()> {
    let pid = record
        .pid
        .ok_or_else(|| super::protocol("running MicroVM record has no shim PID"))?;
    let start_time = record
        .pid_start_time
        .ok_or_else(|| super::protocol("running MicroVM record has no shim start identity"))?;
    require(
        crate::process::is_process_alive_with_identity(pid, Some(start_time)),
        format!("MicroVM shim identity {pid}/{start_time} is not alive"),
    )?;
    verify_microvm_shim_binary(fixture, pid)
}

#[cfg(target_os = "linux")]
fn verify_microvm_shim_binary(fixture: &BoxRuntimeConformanceFixture, pid: u32) -> Result<()> {
    let expected = fixture
        .home_dir
        .join("bin/kbox-shim")
        .canonicalize()
        .map_err(|error| super::external("canonicalize packaged MicroVM shim", error))?;
    let actual = std::fs::read_link(Path::new("/proc").join(pid.to_string()).join("exe"))
        .map_err(|error| super::external("read MicroVM shim executable identity", error))?
        .canonicalize()
        .map_err(|error| super::external("canonicalize live MicroVM shim", error))?;
    require(
        actual == expected,
        format!(
            "MicroVM process did not launch the packaged shim: actual={} expected={}",
            actual.display(),
            expected.display()
        ),
    )
}

#[cfg(not(target_os = "linux"))]
fn verify_microvm_shim_binary(_fixture: &BoxRuntimeConformanceFixture, _pid: u32) -> Result<()> {
    Ok(())
}

fn verify_microvm_secret_persistence(record: &crate::BoxRecord) -> Result<()> {
    let resolved_image = read_regular_file(
        &record.box_dir.join(crate::RESOLVED_IMAGE_CONFIG_FILE),
        "Secret MicroVM resolved image configuration",
    )?;
    let staged_environment = read_microvm_staged_environment(record)?;
    verify_no_plaintext("MicroVM resolved image configuration", &resolved_image)?;
    verify_no_plaintext("MicroVM staged environment", &staged_environment)?;
    verify_environment_manifest("MicroVM", &staged_environment)
}

/// Hypervisor token from a provider build string.
///
/// The product appends `tee/sev-snp-simulated` or `tee/sev-snp-hardware` after
/// the hypervisor name. That suffix is part of the identity, not a second
/// hypervisor, and must not make the security profile reject the build.
pub(super) fn microvm_hypervisor_identity(provider_build: &str) -> Option<&str> {
    let (_, rest) = provider_build
        .split_once(" isolation/microvm hypervisor/")
        .filter(|(prefix, rest)| prefix.starts_with("kbox/") && !rest.is_empty())?;
    let mut parts = rest.split_whitespace();
    let hypervisor = parts.next()?;
    match parts.next() {
        None => Some(hypervisor),
        Some(tee)
            if tee.starts_with("tee/") && tee.len() > "tee/".len() && parts.next().is_none() =>
        {
            Some(hypervisor)
        }
        _ => None,
    }
}

fn read_microvm_staged_environment(record: &crate::BoxRecord) -> Result<Vec<u8>> {
    let relative = RUNTIME_ENV_PATH.trim_start_matches('/');
    let candidates = [
        record
            .box_dir
            .join("runtime-control")
            .join(crate::vm::MICROVM_STAGED_ENVIRONMENT_FILE),
        record.box_dir.join("merged").join(relative),
        record.box_dir.join("rootfs/.krootfs").join(relative),
        record.box_dir.join("rootfs").join(relative),
        record.box_dir.join("upper").join(relative),
    ];
    for path in &candidates {
        match std::fs::symlink_metadata(path) {
            Ok(_) => return read_regular_file(path, "Secret MicroVM staged environment"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(super::external(
                    "inspect Secret MicroVM staged environment",
                    error,
                ))
            }
        }
    }
    Err(super::protocol(format!(
        "Secret MicroVM staged environment is missing below {}",
        record.box_dir.display()
    )))
}

fn read_regular_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| super::external(&format!("inspect {label}"), error))?;
    require(
        metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
        format!("{label} is not a regular file: {}", path.display()),
    )?;
    std::fs::read(path).map_err(|error| super::external(&format!("read {label}"), error))
}

fn verify_no_plaintext(label: &str, bytes: &[u8]) -> Result<()> {
    let value = String::from_utf8_lossy(bytes);
    require(
        !value.contains(SECRET_ENV_VALUE) && !value.contains(SECRET_FILE_VALUE),
        format!("{label} persisted Secret plaintext"),
    )
}

fn verify_environment_manifest(provider: &str, staged_environment: &[u8]) -> Result<()> {
    require(
        String::from_utf8_lossy(staged_environment)
            .lines()
            .any(|line| line.starts_with(&format!("{SECRET_ENVIRONMENT_MANIFEST}="))),
        format!(
            "{provider} staged environment omitted the non-secret environment binding manifest"
        ),
    )
}

#[cfg(test)]
mod identity_tests {
    use super::microvm_hypervisor_identity;

    #[test]
    fn accepts_hypervisor_with_optional_tee_suffix() {
        assert_eq!(
            microvm_hypervisor_identity("kbox/3.2.7 isolation/microvm hypervisor/KVM"),
            Some("KVM")
        );
        assert_eq!(
            microvm_hypervisor_identity(
                "kbox/3.2.7 isolation/microvm hypervisor/KVM tee/sev-snp-simulated"
            ),
            Some("KVM")
        );
        assert_eq!(
            microvm_hypervisor_identity(
                "kbox/3.2.7 isolation/microvm hypervisor/KVM tee/sev-snp-hardware"
            ),
            Some("KVM")
        );
    }

    #[test]
    fn rejects_incomplete_or_extra_identity_tokens() {
        assert_eq!(
            microvm_hypervisor_identity("not-kbox isolation/microvm hypervisor/KVM"),
            None
        );
        assert_eq!(
            microvm_hypervisor_identity("kbox/3.2.7 isolation/microvm hypervisor/"),
            None
        );
        assert_eq!(
            microvm_hypervisor_identity("kbox/3.2.7 isolation/microvm hypervisor/KVM other"),
            None
        );
        assert_eq!(
            microvm_hypervisor_identity(
                "kbox/3.2.7 isolation/microvm hypervisor/KVM tee/sev-snp-simulated extra"
            ),
            None
        );
        assert_eq!(
            microvm_hypervisor_identity(
                "kbox/3.2.7 isolation/microvm hypervisor/KVM tee/"
            ),
            None
        );
    }
}
