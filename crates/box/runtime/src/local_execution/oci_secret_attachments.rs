//! Classify Box-managed Secret bind mounts in `CreateAttachments`.
//!
//! Secret names, values, authorization, and materialization stay in Box. The
//! runtime only receives mount-index classifications via `mark_secret_mount`.

use std::path::{Path, PathBuf};

use kbox_core::{ExecutionManagerError, ExecutionManagerResult};
use koci_sdk::{CreateAttachments, OciBundle};

/// Mark every OCI bind whose source is under the managed Secret root.
///
/// When `managed_secret_root` is absent, attachments are unchanged. Overlap
/// with storage classifications fails closed inside the SDK.
pub(super) fn attach_box_managed_secret_mounts(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    managed_secret_root: Option<&Path>,
) -> ExecutionManagerResult<CreateAttachments> {
    let Some(root) = managed_secret_root else {
        return Ok(attachments);
    };
    let Ok(root) = root.canonicalize() else {
        return Err(ExecutionManagerError::Unavailable(format!(
            "managed Secret root {} is not reachable for attachment classification",
            root.display()
        )));
    };

    let mounts = oci_bind_mount_sources(bundle)?;
    let mut attachments = attachments;
    let mut marked = 0usize;
    for (index, source) in mounts.iter().enumerate() {
        let Some(source) = source else {
            continue;
        };
        let Ok(canonical) = source.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(&root) {
            continue;
        }
        attachments = attachments.mark_secret_mount(index).map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!(
                "failed to classify managed Secret mount at OCI index {index}: {error}"
            ))
        })?;
        marked += 1;
    }
    if marked == 0 {
        return Err(ExecutionManagerError::Unavailable(format!(
            "managed Secret root {} was configured but no OCI bind mounts were classified as secrets",
            root.display()
        )));
    }
    Ok(attachments)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(super) fn validate_hvf_managed_secret_files(
    record: &crate::BoxRecord,
) -> ExecutionManagerResult<()> {
    let metadata = record.managed_execution.as_ref().ok_or_else(|| {
        ExecutionManagerError::Internal(
            "managed Secret validation has no execution metadata".into(),
        )
    })?;
    let config = &metadata.request.config;
    let environment: Vec<_> = config
        .extra_env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    let bindings = koci_sdk::MemorySecretEnvironmentBinding::from_environment(&environment)
        .map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!("Secret environment projection: {error}"))
        })?;
    let Some(root) = &metadata.request.policy.managed_secret_root else {
        if bindings.is_some() {
            return Err(ExecutionManagerError::InvalidRequest(
                "Secret environment projection requires a managed memory root".into(),
            ));
        }
        return Ok(());
    };
    if let Some(bindings) = &bindings {
        for binding in bindings {
            let mut matching = 0;
            for spec in &config.volumes {
                let volume = crate::vm::VmManager::parse_volume_spec(spec)
                    .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
                if volume.guest_path == binding.path() {
                    if !volume.read_only || !volume.host_path.starts_with(root) {
                        return Err(ExecutionManagerError::InvalidRequest(
                            "Secret environment binding lacks its exact managed readonly file"
                                .into(),
                        ));
                    }
                    matching += 1;
                }
            }
            if matching != 1 {
                return Err(ExecutionManagerError::InvalidRequest(
                    "Secret environment binding has missing or ambiguous file authority".into(),
                ));
            }
        }
    }
    crate::BoxTransientSecretStore::new(root)
        .require_ready_sync()
        .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
    let mut count = 0;
    for spec in &config.volumes {
        let volume = crate::vm::VmManager::parse_volume_spec(spec)
            .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
        if !volume.host_path.starts_with(root) {
            continue;
        }
        let file = std::fs::symlink_metadata(&volume.host_path)
            .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
        if bindings.as_ref().is_some_and(|bindings| {
            bindings
                .iter()
                .any(|binding| binding.path() == volume.guest_path)
        }) {
            use std::os::unix::fs::PermissionsExt;
            if file.permissions().mode() & 0o777 != 0o400 {
                return Err(ExecutionManagerError::InvalidRequest(
                    "Secret environment file must retain exact mode400".into(),
                ));
            }
        }
        if !volume.read_only
            || !file.is_file()
            || file.file_type().is_symlink()
            || file.len() == 0
            || file.len() > 1024 * 1024
        {
            return Err(ExecutionManagerError::InvalidRequest(
                "managed Secret must be a bounded nonempty read-only regular file".into(),
            ));
        }
        count += 1;
    }
    if count == 0 {
        return Err(ExecutionManagerError::InvalidRequest(
            "managed Secret root has no matching file grants".into(),
        ));
    }
    Ok(())
}

fn oci_bind_mount_sources(bundle: &OciBundle) -> ExecutionManagerResult<Vec<Option<PathBuf>>> {
    let configuration: serde_json::Value =
        serde_json::from_str(bundle.config_json()).map_err(|error| {
            ExecutionManagerError::Internal(format!(
                "failed to decode OCI config for secret attachments: {error}"
            ))
        })?;
    let Some(mounts) = configuration
        .get("mounts")
        .and_then(|value| value.as_array())
    else {
        return Ok(Vec::new());
    };
    Ok(mounts
        .iter()
        .map(|mount| {
            let typ = mount.get("type").and_then(|value| value.as_str());
            if typ != Some("bind") {
                return None;
            }
            mount
                .get("source")
                .and_then(|value| value.as_str())
                .map(PathBuf::from)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use koci_sdk::ProcessIo;
    use serde_json::json;

    fn write_bundle(home: &Path, config: serde_json::Value) -> OciBundle {
        let bundle_dir = home.join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        OciBundle::from_json(&bundle_dir, serde_json::to_string(&config).unwrap()).unwrap()
    }

    fn secret_count(attachments: &CreateAttachments) -> usize {
        serde_json::to_value(attachments)
            .unwrap()
            .get("secrets")
            .and_then(|value| value.as_array())
            .map(|value| value.len())
            .unwrap_or(0)
    }

    #[test]
    fn leaves_attachments_unchanged_without_secret_root() {
        let home = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let source = external.path().canonicalize().unwrap();
        let bundle = write_bundle(
            home.path(),
            json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/data",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "ro"]
                }]
            }),
        );
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached = attach_box_managed_secret_mounts(&bundle, attachments, None).unwrap();
        assert_eq!(secret_count(&attached), 0);
    }

    #[test]
    fn marks_bind_under_managed_secret_root() {
        let home = tempfile::tempdir().unwrap();
        let secret_root = home.path().join("runtime-secrets");
        std::fs::create_dir_all(&secret_root).unwrap();
        let secret_file = secret_root.join("token");
        std::fs::write(&secret_file, b"redacted").unwrap();
        let source = secret_file.canonicalize().unwrap();
        let bundle = write_bundle(
            home.path(),
            json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [
                    {
                        "destination": "/proc",
                        "type": "proc",
                        "source": "proc",
                        "options": ["nosuid", "noexec", "nodev"]
                    },
                    {
                        "destination": "/run/secrets/token",
                        "type": "bind",
                        "source": source,
                        "options": ["rbind", "ro"]
                    }
                ]
            }),
        );
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        assert_eq!(secret_count(&attachments), 0);
        let attached =
            attach_box_managed_secret_mounts(&bundle, attachments, Some(&secret_root)).unwrap();
        assert_eq!(secret_count(&attached), 1);
        attached.validate(&bundle).unwrap();
    }

    #[test]
    fn ignores_binds_outside_secret_root_and_fails_if_none_match() {
        let home = tempfile::tempdir().unwrap();
        let secret_root = home.path().join("runtime-secrets");
        std::fs::create_dir_all(&secret_root).unwrap();
        let external = tempfile::tempdir().unwrap();
        let source = external.path().canonicalize().unwrap();
        let bundle = write_bundle(
            home.path(),
            json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/data",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "rw"]
                }]
            }),
        );
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let error =
            attach_box_managed_secret_mounts(&bundle, attachments, Some(&secret_root)).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("no OCI bind mounts were classified as secrets"),
            "{error}"
        );
    }
}
