//! Backend-neutral execution isolation resolution.

use serde::{Deserialize, Serialize};

use crate::config::{BoxConfig, ExecutionIsolation};
use crate::error::{BoxError, Result};

/// Concrete backend selected for an execution request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionBackend {
    /// libkbacked MicroVM execution.
    Kbox,
}

/// Security boundary provided by the resolved backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationClass {
    /// A hardware-backed virtual-machine boundary.
    HardwareVm,
}

/// Deterministic result of resolving one execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedExecutionPlan {
    /// Isolation requested by the caller or selected by the implicit default.
    pub requested_isolation: ExecutionIsolation,
    /// Concrete runtime backend.
    pub backend: ExecutionBackend,
    /// Effective security-boundary class.
    pub isolation_class: IsolationClass,
    /// Controls that the selected backend must prove before launch.
    pub required_controls: Vec<String>,
}

/// Resolve a box configuration without probing or mutating the host.
///
/// Host capabilities are checked separately immediately before preparation.
/// Keeping this function pure makes unsupported feature combinations fail
/// before image pulls, rootfs mounts, state changes, or runtime processes.
pub fn resolve_execution(config: &BoxConfig) -> Result<ResolvedExecutionPlan> {
    crate::validate_host_service_sockets(
        &config.host_services,
        config.sidecar.as_ref().map(|sidecar| sidecar.vsock_port),
    )?;
    if !config.host_services.is_empty()
        && (config.snapshot_mem_file.is_some()
            || config.snapshot_sock.is_some()
            || config.restore_from.is_some())
    {
        return Err(BoxError::ConfigError(
            "Host service sockets are not supported by snapshot boot".into(),
        ));
    }
    match config.isolation {
        ExecutionIsolation::Microvm => {
            validate_microvm_compatibility(config)?;
            Ok(ResolvedExecutionPlan {
                requested_isolation: ExecutionIsolation::Microvm,
                backend: ExecutionBackend::Kbox,
                isolation_class: IsolationClass::HardwareVm,
                required_controls: Vec::new(),
            })
        }
    }
}

/// Validate features that cannot be represented safely by the MicroVM backend.
pub fn validate_microvm_compatibility(config: &BoxConfig) -> Result<()> {
    if config.isolation != ExecutionIsolation::Microvm {
        return Ok(());
    }

    validate_security_options(config)
}

fn validate_security_options(config: &BoxConfig) -> Result<()> {
    for raw_option in &config.security_opt {
        let option = raw_option.trim();
        if option.is_empty() {
            return Err(BoxError::ConfigError(
                "microVM isolation does not accept an empty security option".to_string(),
            ));
        }

        if option.eq_ignore_ascii_case("no-new-privileges") {
            continue;
        }

        let Some((key, value)) = option.split_once('=') else {
            return Err(unsupported_security_option(option));
        };
        let key = key.trim();
        let value = value.trim();

        if key.eq_ignore_ascii_case("seccomp") {
            if value.eq_ignore_ascii_case("default") || value.eq_ignore_ascii_case("unconfined") {
                continue;
            }
            if value.is_empty() {
                return Err(BoxError::ConfigError(
                    "microVM isolation requires a seccomp mode".to_string(),
                ));
            }
            return Err(BoxError::ConfigError(format!(
                "microVM isolation does not support custom seccomp profile '{value}'"
            )));
        }

        if key.eq_ignore_ascii_case("no-new-privileges") {
            if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
                continue;
            }
            return Err(BoxError::ConfigError(format!(
                "microVM isolation requires no-new-privileges to be true or false, got '{value}'"
            )));
        }

        if key.eq_ignore_ascii_case("apparmor") {
            return Err(BoxError::ConfigError(format!(
                "microVM isolation does not support AppArmor security option '{option}'"
            )));
        }

        if key.eq_ignore_ascii_case("label") {
            return Err(BoxError::ConfigError(format!(
                "microVM isolation does not support SELinux label security option '{option}'"
            )));
        }

        return Err(unsupported_security_option(option));
    }

    Ok(())
}

fn unsupported_security_option(option: &str) -> BoxError {
    BoxError::ConfigError(format!(
        "microVM isolation does not support security option '{option}'"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resolves_only_to_khardware_vm() {
        let plan = resolve_execution(&BoxConfig::default()).unwrap();
        assert_eq!(plan.backend, ExecutionBackend::Kbox);
        assert_eq!(plan.isolation_class, IsolationClass::HardwareVm);
        assert!(plan.required_controls.is_empty());
    }

    #[test]
    fn microvm_rejects_host_kernel_and_custom_security_profiles() {
        for (option, expected) in [
            ("apparmor=runtime/default", "AppArmor"),
            ("label=type:container_t", "SELinux"),
            ("seccomp=/profiles/restricted.json", "custom seccomp"),
            ("systempaths=unconfined", "security option"),
        ] {
            let config = BoxConfig {
                security_opt: vec![option.to_string()],
                ..Default::default()
            };
            let error = resolve_execution(&config).unwrap_err().to_string();
            assert!(
                error.contains(expected),
                "expected {option:?} rejection to mention {expected:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn microvm_accepts_guest_enforceable_security_options() {
        let config = BoxConfig {
            security_opt: vec![
                " SECCOMP=DEFAULT ".to_string(),
                "seccomp=unconfined".to_string(),
                "no-new-privileges".to_string(),
                "no-new-privileges=false".to_string(),
            ],
            cap_add: vec!["NET_ADMIN".to_string()],
            cap_drop: vec!["NET_RAW".to_string()],
            privileged: true,
            ..Default::default()
        };

        assert!(resolve_execution(&config).is_ok());
    }
}
