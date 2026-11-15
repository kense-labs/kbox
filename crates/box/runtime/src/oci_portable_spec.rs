//! Portable MicroVM OCI bundle compilation for dedicated-VM OCI Runtime handoff.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kbox_core::error::{BoxError, Result};
use koci_sdk::{
    PORTABLE_ROOTFS_METADATA_ANNOTATION, PORTABLE_ROOTFS_METADATA_SCHEMA_V1,
    RUNTIME_BUNDLE_HANDOFF_EXTENSION, RUNTIME_BUNDLE_HANDOFF_MOVE_V1,
};
use oci_spec::runtime::{
    Capabilities, Capability, LinuxBuilder, LinuxCapabilitiesBuilder, LinuxNamespaceBuilder,
    LinuxNamespaceType, Mount, MountBuilder, ProcessBuilder, RootBuilder, Spec, SpecBuilder,
    UserBuilder,
};

/// Qualification schema for a portable one-VM-per-container bundle.
pub const PORTABLE_MICROVM_BUNDLE_SCHEMA: &str = "kbox.portable-microvm-bundle.v1";

/// Container process compiled for the long-lived Kbox OCI Runtime owner.
///
/// The executable, user and working directory are resolved against the
/// prepared rootfs before this value reaches the compiler. Bootstrap listener
/// descriptors deliberately have no representation here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrovmRuntimeProcess {
    pub args: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub uid: u32,
    pub gid: u32,
    pub additional_gids: Vec<u32>,
    pub dropped_capabilities: Vec<String>,
}

const RESERVED_BOOTSTRAP_ENVIRONMENT: &[&str] = &[
    "K_BOOTSTRAP_MODE",
    "K_EXEC_LISTENER_FD",
    "K_PTY_LISTENER_FD",
    "KRUN_INIT_LOG_FD",
    "K_SANDBOX_SHARE_HOST_NETWORK",
];

/// Compile the bounded portable profile consumed by OCI Runtime's dedicated-VM driver.
pub fn compile_portable_microvm_oci_spec(
    box_id: &str,
    hostname: &str,
    runtime_process: &MicrovmRuntimeProcess,
) -> Result<Spec> {
    validate_box_id(box_id)?;
    validate_hostname(hostname)?;
    validate_runtime_process_shape(runtime_process)?;

    let user = UserBuilder::default()
        .uid(runtime_process.uid)
        .gid(runtime_process.gid)
        .additional_gids(runtime_process.additional_gids.clone())
        .build()
        .map_err(oci_error)?;
    let process = ProcessBuilder::default()
        .terminal(false)
        .user(user)
        .args(runtime_process.args.clone())
        .env(compile_runtime_environment(&runtime_process.environment)?)
        .cwd(runtime_process.cwd.clone())
        .capabilities(compile_workload_capabilities(
            &[],
            &runtime_process.dropped_capabilities,
        )?)
        .no_new_privileges(true)
        .build()
        .map_err(oci_error)?;
    let linux = LinuxBuilder::default()
        .namespaces(compile_portable_microvm_namespaces()?)
        .cgroups_path(PathBuf::from(format!("kbox/{box_id}")))
        .build()
        .map_err(oci_error)?;
    let mut annotations = HashMap::new();
    annotations.insert(
        "com.kbox.microvm.schema".to_string(),
        PORTABLE_MICROVM_BUNDLE_SCHEMA.to_string(),
    );
    annotations.insert(
        "com.kbox.isolation-class".to_string(),
        "hardware-vm".to_string(),
    );
    annotations.insert(
        "com.kbox.process-owner".to_string(),
        "koci-runtime".to_string(),
    );
    annotations.insert(
        RUNTIME_BUNDLE_HANDOFF_EXTENSION.to_string(),
        RUNTIME_BUNDLE_HANDOFF_MOVE_V1.to_string(),
    );
    // Windows virtio-fs cannot store Linux UID/GID; the guest must replay the
    // portable metadata contract. Linux/macOS same-uid virtio-fs retains the
    // Host share-root UIDs and refuses guest chown by design, so ownership
    // replay would fail closed with EPERM.
    if cfg!(windows) {
        annotations.insert(
            PORTABLE_ROOTFS_METADATA_ANNOTATION.to_string(),
            PORTABLE_ROOTFS_METADATA_SCHEMA_V1.to_string(),
        );
    }

    SpecBuilder::default()
        .version("1.3.0".to_string())
        .root(
            RootBuilder::default()
                .path(PathBuf::from("rootfs"))
                .readonly(false)
                .build()
                .map_err(oci_error)?,
        )
        .mounts(vec![mount(
            "/proc",
            "proc",
            "proc",
            &["nosuid", "noexec", "nodev"],
        )?])
        .process(process)
        .hostname(hostname.to_string())
        .annotations(annotations)
        .linux(linux)
        .build()
        .map_err(oci_error)
}

fn validated_environment(
    environment: &[(String, String)],
) -> Result<std::collections::BTreeMap<String, String>> {
    let mut values = std::collections::BTreeMap::new();
    for (key, value) in environment {
        if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
            return Err(BoxError::ConfigError(format!(
                "Invalid MicroVM process environment key {key:?}"
            )));
        }
        values.insert(key.clone(), value.clone());
    }
    values.entry("PATH".to_string()).or_insert_with(|| {
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()
    });
    Ok(values)
}

fn compile_runtime_environment(environment: &[(String, String)]) -> Result<Vec<String>> {
    let mut values = validated_environment(environment)?;
    for reserved in RESERVED_BOOTSTRAP_ENVIRONMENT {
        values.remove(*reserved);
    }
    Ok(values
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect())
}

fn validate_runtime_process_shape(process: &MicrovmRuntimeProcess) -> Result<()> {
    let executable = process.args.first().ok_or_else(|| {
        BoxError::ConfigError("MicroVM runtime-owned process requires an executable".to_string())
    })?;
    validate_linux_absolute_normalized(Path::new(executable), "process executable")?;
    if process.args.iter().any(|argument| argument.contains('\0')) {
        return Err(BoxError::ConfigError(
            "MicroVM process arguments must not contain NUL".to_string(),
        ));
    }
    validate_linux_absolute_normalized(&process.cwd, "process working directory")?;
    Ok(())
}

pub(crate) fn compile_workload_capabilities(
    added: &[String],
    dropped: &[String],
) -> Result<oci_spec::runtime::LinuxCapabilities> {
    let mut names: std::collections::BTreeSet<String> = [
        "CHOWN",
        "DAC_OVERRIDE",
        "FOWNER",
        "FSETID",
        "KILL",
        "NET_BIND_SERVICE",
        "SETGID",
        "SETPCAP",
        "SETUID",
        "SYS_CHROOT",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect();

    for capability in added {
        let normalized = normalize_capability_name(capability);
        parse_allowed_capability(&normalized)?;
        names.insert(normalized);
    }
    if dropped
        .iter()
        .any(|capability| normalize_capability_name(capability) == "ALL")
    {
        names.clear();
    } else {
        for capability in dropped {
            let normalized = normalize_capability_name(capability);
            parse_allowed_capability(&normalized)?;
            names.remove(&normalized);
        }
    }

    let capabilities = names
        .iter()
        .map(|capability| parse_allowed_capability(capability))
        .collect::<Result<Capabilities>>()?;
    LinuxCapabilitiesBuilder::default()
        .bounding(capabilities.clone())
        .effective(capabilities.clone())
        .permitted(capabilities)
        .inheritable(HashSet::<Capability>::new())
        .ambient(HashSet::<Capability>::new())
        .build()
        .map_err(oci_error)
}

pub(crate) fn normalize_capability_name(value: &str) -> String {
    let normalized = value.trim().to_ascii_uppercase();
    normalized
        .strip_prefix("CAP_")
        .unwrap_or(&normalized)
        .to_string()
}

fn parse_allowed_capability(value: &str) -> Result<Capability> {
    let normalized = normalize_capability_name(value);
    let capability = match normalized.as_str() {
        "AUDIT_WRITE" => Capability::AuditWrite,
        "CHOWN" => Capability::Chown,
        "DAC_OVERRIDE" => Capability::DacOverride,
        "FOWNER" => Capability::Fowner,
        "FSETID" => Capability::Fsetid,
        "KILL" => Capability::Kill,
        "MKNOD" => Capability::Mknod,
        "NET_BIND_SERVICE" => Capability::NetBindService,
        "SETFCAP" => Capability::Setfcap,
        "SETGID" => Capability::Setgid,
        "SETPCAP" => Capability::Setpcap,
        "SETUID" => Capability::Setuid,
        "SYS_CHROOT" => Capability::SysChroot,
        _ => {
            return Err(BoxError::ConfigError(format!(
                "MicroVM capability {value:?} is outside the allowlist"
            )))
        }
    };
    Ok(capability)
}

fn compile_portable_microvm_namespaces() -> Result<Vec<oci_spec::runtime::LinuxNamespace>> {
    [
        LinuxNamespaceType::Uts,
        LinuxNamespaceType::Mount,
        LinuxNamespaceType::Ipc,
        LinuxNamespaceType::Network,
        LinuxNamespaceType::Cgroup,
        LinuxNamespaceType::Pid,
    ]
    .into_iter()
    .map(|typ| {
        LinuxNamespaceBuilder::default()
            .typ(typ)
            .build()
            .map_err(oci_error)
    })
    .collect()
}

fn mount(destination: &str, typ: &str, source: &str, options: &[&str]) -> Result<Mount> {
    MountBuilder::default()
        .destination(PathBuf::from(destination))
        .typ(typ.to_string())
        .source(PathBuf::from(source))
        .options(
            options
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        )
        .build()
        .map_err(oci_error)
}

fn validate_box_id(box_id: &str) -> Result<()> {
    if box_id.is_empty()
        || box_id.len() > 128
        || !box_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(BoxError::ConfigError(format!(
            "Invalid MicroVM box ID {box_id:?}"
        )));
    }
    Ok(())
}

fn validate_hostname(hostname: &str) -> Result<()> {
    if hostname.is_empty()
        || hostname.len() > 253
        || hostname.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(BoxError::ConfigError(format!(
            "Invalid MicroVM hostname {hostname:?}"
        )));
    }
    Ok(())
}

fn validate_linux_absolute_normalized(path: &Path, label: &str) -> Result<()> {
    let Some(path) = path.to_str() else {
        return Err(BoxError::ConfigError(format!(
            "MicroVM {label} must be a UTF-8 Linux path"
        )));
    };
    if !path.starts_with('/')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| matches!(component, "." | ".."))
    {
        return Err(BoxError::ConfigError(format!(
            "MicroVM {label} must be an absolute normalized Linux path: {path}"
        )));
    }
    Ok(())
}

fn oci_error(error: impl std::fmt::Display) -> BoxError {
    BoxError::ConfigError(format!(
        "Failed to compile portable MicroVM OCI spec: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn as_json(spec: &Spec) -> Value {
        serde_json::to_value(spec).unwrap()
    }

    fn sample_runtime_process() -> MicrovmRuntimeProcess {
        MicrovmRuntimeProcess {
            args: vec!["/usr/bin/example".to_string(), "--serve".to_string()],
            environment: vec![
                ("APP_MODE".to_string(), "production".to_string()),
                ("K_EXEC_LISTENER_FD".to_string(), "99".to_string()),
                ("K_PTY_LISTENER_FD".to_string(), "98".to_string()),
                ("KRUN_INIT_LOG_FD".to_string(), "97".to_string()),
                ("K_BOOTSTRAP_MODE".to_string(), "image-value".to_string()),
            ],
            cwd: PathBuf::from("/workspace"),
            uid: 123,
            gid: 456,
            additional_gids: vec![789],
            dropped_capabilities: vec!["SETUID".to_string()],
        }
    }

    #[test]
    fn portable_microvm_compiler_uses_relative_root_and_no_user_namespace() {
        let value = as_json(
            &compile_portable_microvm_oci_spec("box-123", "box-123", &sample_runtime_process())
                .unwrap(),
        );

        assert_eq!(value["ociVersion"], "1.3.0");
        assert_eq!(value["root"]["path"], "rootfs");
        assert_eq!(value["root"]["readonly"], false);
        let annotations = value["annotations"].as_object().unwrap();
        if cfg!(windows) {
            assert_eq!(
                annotations[PORTABLE_ROOTFS_METADATA_ANNOTATION],
                PORTABLE_ROOTFS_METADATA_SCHEMA_V1
            );
        } else {
            assert!(!annotations.contains_key(PORTABLE_ROOTFS_METADATA_ANNOTATION));
        }
        assert_eq!(
            value["annotations"][RUNTIME_BUNDLE_HANDOFF_EXTENSION],
            RUNTIME_BUNDLE_HANDOFF_MOVE_V1
        );
        assert_eq!(
            value["annotations"]["com.kbox.isolation-class"],
            "hardware-vm"
        );
        let namespaces = value["linux"]["namespaces"].as_array().unwrap();
        assert!(namespaces
            .iter()
            .any(|namespace| namespace["type"] == "mount"));
        assert!(!namespaces
            .iter()
            .any(|namespace| namespace["type"] == "user"));
        assert_eq!(value["mounts"][0]["destination"], "/proc");
    }
}
