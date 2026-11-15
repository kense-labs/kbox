//! Box-owned preparation for a portable dedicated-VM OCI bundle.

use std::path::{Path, PathBuf};

use kbox_core::{BoxError, ExecutionBackend, ResolvedExecutionPlan, Result};

use super::VmManager;
use crate::oci_portable_spec::{compile_portable_microvm_oci_spec, MicrovmRuntimeProcess};
use crate::process_path::{
    normalize_linux_guest_path, resolve_runtime_executable_in, ProcessRootfs,
    DEFAULT_CONTAINER_PATH,
};

/// Portable handoff produced without starting a Box-owned VM.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeOwnedMicrovmBundle {
    pub bundle_dir: PathBuf,
    pub console_output: PathBuf,
    pub anonymous_volumes: Vec<String>,
}

impl VmManager {
    /// Resolve the image process, copy the fresh rootfs, and publish one exact handoff bundle.
    pub(crate) async fn prepare_runtime_owned_microvm_bundle(
        &mut self,
        execution_plan: &ResolvedExecutionPlan,
        bundle_directory: &Path,
    ) -> Result<RuntimeOwnedMicrovmBundle> {
        if execution_plan.backend != ExecutionBackend::Kbox {
            return Err(BoxError::BoxBootError {
                message: "portable MicroVM OCI preparation requires the Kbox execution plan"
                    .to_string(),
                hint: None,
            });
        }

        let original_anonymous_volumes = self.anonymous_volumes.clone();
        let box_dir = self.home_dir.join("boxes").join(&self.box_id);
        let layout = match self.prepare_layout().await {
            Ok(layout) => layout,
            Err(error) => {
                self.cleanup_boot_failure().await;
                return Err(error);
            }
        };
        self.image_config = layout.oci_config.clone();

        let prepare = (|| -> Result<RuntimeOwnedMicrovmBundle> {
            let instance_spec = self.build_runtime_owned_instance_spec(&layout)?;
            if self.anonymous_volumes != original_anonymous_volumes {
                return Err(BoxError::ConfigError(
                    "portable MicroVM OCI qualification does not support image-declared volumes"
                        .to_string(),
                ));
            }
            let runtime_process: MicrovmRuntimeProcess = resolve_runtime_owned_process(
                &layout.rootfs_path,
                &instance_spec,
                &self.config.cap_drop,
            )?;
            let hostname = self
                .config
                .hostname
                .clone()
                .unwrap_or_else(|| self.box_id.clone());
            let spec =
                compile_portable_microvm_oci_spec(&self.box_id, &hostname, &runtime_process)?;
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            let spec = {
                let mut spec = spec;
                super::oci_hvf_spec::complete(&mut spec, &self.config, &instance_spec)?;
                spec
            };
            crate::local_execution::oci_portable_rootfs::publish_portable_bundle(
                &layout.rootfs_path,
                &spec,
                bundle_directory,
            )?;

            Ok(RuntimeOwnedMicrovmBundle {
                bundle_dir: bundle_directory.to_path_buf(),
                console_output: instance_spec
                    .console_output
                    .unwrap_or_else(|| box_dir.join("logs").join("console.log")),
                anonymous_volumes: self.anonymous_volumes.clone(),
            })
        })();

        match prepare {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                self.cleanup_boot_failure().await;
                Err(error)
            }
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) async fn prepare_runtime_owned_ext4_bundle(
        &mut self,
        execution_plan: &ResolvedExecutionPlan,
        bundle_directory: &Path,
        caller_files: &std::collections::BTreeMap<PathBuf, koci_sdk::FileSourceIdentity>,
    ) -> Result<RuntimeOwnedMicrovmBundle> {
        if execution_plan.backend != ExecutionBackend::Kbox {
            return Err(BoxError::ConfigError(
                "workload ext4 requires the Kbox execution plan".into(),
            ));
        }
        let reference = self.config.image.clone();
        let auth = self.transient_registry_auth.take();
        let image = self.pull_runtime_image(&reference, auth).await?;
        if image.platform().os != "linux" || image.platform().architecture != "arm64" {
            return Err(BoxError::ConfigError(
                "HVF workload rootfs requires a Linux arm64 image".into(),
            ));
        }
        super::oci_hvf_spec::validate_tmpfs_image_volumes(&self.config, &image.config().volumes)?;
        let box_dir = self.home_dir.join("boxes").join(&self.box_id);
        std::fs::create_dir_all(box_dir.join("logs")).map_err(BoxError::IoError)?;
        // Cancellation may remove the execution directory while a blocking builder is finishing.
        let preparation = tempfile::Builder::new()
            .prefix(".oci-workload-")
            .tempdir_in(&self.home_dir)
            .map_err(BoxError::IoError)?;
        let empty_root = preparation.path().join("rootfs");
        std::fs::create_dir(&empty_root).map_err(BoxError::IoError)?;
        let workspace = if self.config.workspace.as_os_str().is_empty() {
            box_dir.join("workspace")
        } else {
            self.config.workspace.clone()
        };
        std::fs::create_dir_all(&workspace).map_err(BoxError::IoError)?;
        let socket = super::layout::ensure_runtime_socket_dir(&self.home_dir, &self.box_id)?;
        self.image_config = Some(image.config().clone());
        crate::resolved_image::persist_resolved_image_config(&box_dir, image.config())?;
        let layout = super::BoxLayout {
            rootfs_path: empty_root.clone(),
            resumed_rootfs: None,
            exec_socket_path: socket.join("exec.sock"),
            pty_socket_path: socket.join("pty.sock"),
            attest_socket_path: socket.join("attest.sock"),
            port_forward_socket_path: socket.join("portfwd.sock"),
            workspace_path: workspace.canonicalize().map_err(BoxError::IoError)?,
            console_output: Some(box_dir.join("logs/console.log")),
            oci_config: Some(image.config().clone()),
            oci_manifest_digest: Some(image.manifest_digest().to_string()),
            prefer_image_rootfs_metadata: true,
            tee_instance_config: None,
        };
        let spec = self.build_runtime_owned_instance_spec(&layout)?;
        let volume_plans = self.anonymous_disk_plan(image.config())?;
        let anonymous_mounts: Vec<_> = volume_plans
            .iter()
            .map(|(plan, volume)| (PathBuf::from(&volume.mount_point), plan.guest_path.clone()))
            .collect();
        let source_digest = image.manifest_digest().to_string();
        // The blocking job owns its temporary directory even if its awaiting future is cancelled.
        let (preparation, mut rootfs) = tokio::task::spawn_blocking(move || {
            let rootfs = crate::rootfs::OciWorkloadRootfs::load(&image, preparation.path())?;
            Ok::<_, BoxError>((preparation, rootfs))
        })
        .await
        .map_err(|error| BoxError::BuildError(error.to_string()))??;
        let process = resolve_runtime_owned_process_in(&mut rootfs, &spec, &self.config.cap_drop)?;
        let capture_baseline = crate::rootfs::guest_diff_baseline_required(&box_dir)?;
        let hostname = self
            .config
            .hostname
            .clone()
            .unwrap_or_else(|| self.box_id.clone());
        let mut oci = compile_portable_microvm_oci_spec(&self.box_id, &hostname, &process)?;
        let destination = preparation.path().join("artifact");
        let options = crate::rootfs::Ext4ArtifactOptions::from_disk_mib(
            self.config.resources.disk_mb,
            uuid::Uuid::new_v4().into_bytes(),
        )?;
        let disk_mib = self.config.resources.disk_mb;
        let artifact = tokio::task::spawn_blocking(move || {
            use sha2::{Digest, Sha256};
            use std::io::Read;
            let baseline = capture_baseline
                .then(|| rootfs.diff_baseline_bytes())
                .transpose()?;
            let volumes = super::oci_anonymous::stage(
                &mut rootfs,
                &volume_plans,
                disk_mib,
                preparation.path(),
            )?;
            let artifact = rootfs.publish(&destination, options)?;
            let mut input = std::fs::File::open(&artifact.disk).map_err(BoxError::IoError)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = input.read(&mut buffer).map_err(BoxError::IoError)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok::<_, BoxError>((
                preparation,
                artifact,
                format!("sha256:{:x}", hasher.finalize()),
                baseline,
                volumes,
            ))
        })
        .await
        .map_err(|error| BoxError::BuildError(error.to_string()))??;
        super::oci_anonymous::publish(&self.home_dir, &self.box_id, artifact.4)?;
        super::oci_hvf_spec::complete_with_anonymous(
            &mut oci,
            &self.config,
            &spec,
            caller_files,
            &anonymous_mounts,
        )?;
        super::oci_hvf_spec::complete_memory_secret_files(
            &mut oci,
            self.managed_secret_root.as_deref(),
        )?;
        if let Some(baseline) = artifact.3.as_deref() {
            crate::rootfs::install_baseline_bytes_if_absent(&box_dir, baseline)?;
        }
        let descriptor =
            koci_sdk::RootfsImage::new(artifact.2, options.capacity_bytes, source_digest)
                .map_err(|error| BoxError::ConfigError(error.to_string()))?;
        let descriptor = if self.config.read_only {
            descriptor
                .with_readonly_root()
                .map_err(|error| BoxError::ConfigError(error.to_string()))?
        } else {
            descriptor
        };
        oci.annotations_mut().get_or_insert_default().insert(
            koci_sdk::ROOTFS_IMAGE_EXTENSION.into(),
            descriptor
                .annotation_value()
                .map_err(|error| BoxError::ConfigError(error.to_string()))?,
        );
        crate::local_execution::oci_portable_rootfs::publish_portable_bundle_with_disk(
            &empty_root,
            &oci,
            bundle_directory,
            Some(&artifact.1.disk),
        )?;
        Ok(RuntimeOwnedMicrovmBundle {
            bundle_dir: bundle_directory.into(),
            console_output: box_dir.join("logs/console.log"),
            anonymous_volumes: self.anonymous_volumes.clone(),
        })
    }

    /// Remove only Box-owned rootfs and socket preparation after runtime deletion.
    pub(crate) fn cleanup_runtime_owned_microvm_bundle(&self) -> Result<()> {
        let box_dir = self.home_dir.join("boxes").join(&self.box_id);
        self.rootfs_provider.cleanup(&box_dir, false)?;
        match std::fs::remove_dir_all(self.socket_dir()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(BoxError::IoError(error)),
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimePasswdEntry {
    name: String,
    uid: u32,
    gid: u32,
    home: Option<String>,
}

#[derive(Debug, Clone)]
struct RuntimeGroupEntry {
    name: String,
    gid: u32,
    members: Vec<String>,
}

pub(crate) fn resolve_runtime_owned_process(
    mut rootfs: &Path,
    instance_spec: &crate::vmm::InstanceSpec,
    dropped_capabilities: &[String],
) -> Result<MicrovmRuntimeProcess> {
    resolve_runtime_owned_process_in(&mut rootfs, instance_spec, dropped_capabilities)
}

pub(crate) fn resolve_runtime_owned_process_in(
    rootfs: &mut impl ProcessRootfs,
    instance_spec: &crate::vmm::InstanceSpec,
    dropped_capabilities: &[String],
) -> Result<MicrovmRuntimeProcess> {
    let cwd = normalize_linux_guest_path("/", &instance_spec.workdir, "working directory")?;
    rootfs.ensure_directory(&cwd)?;

    let mut environment = instance_spec.entrypoint.env.clone();
    let path = environment
        .iter()
        .rev()
        .find_map(|(key, value)| (key == "PATH").then_some(value.as_str()))
        .unwrap_or(DEFAULT_CONTAINER_PATH);
    let executable =
        resolve_runtime_executable_in(rootfs, &cwd, path, &instance_spec.entrypoint.executable)?;
    let identity = resolve_runtime_identity_in(rootfs, instance_spec.user.as_deref())?;
    if !environment.iter().any(|(key, _)| key == "HOME") {
        if let Some(home) = identity.home.as_ref() {
            environment.push(("HOME".to_string(), home.clone()));
        }
    }
    let mut args = Vec::with_capacity(instance_spec.entrypoint.args.len() + 1);
    args.push(executable);
    args.extend(instance_spec.entrypoint.args.iter().cloned());

    Ok(MicrovmRuntimeProcess {
        args,
        environment,
        cwd: PathBuf::from(cwd),
        uid: identity.uid,
        gid: identity.gid,
        additional_gids: identity.additional_gids,
        dropped_capabilities: dropped_capabilities.to_vec(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeIdentity {
    uid: u32,
    gid: u32,
    additional_gids: Vec<u32>,
    home: Option<String>,
}

#[cfg(test)]
fn resolve_runtime_identity(mut rootfs: &Path, requested: Option<&str>) -> Result<RuntimeIdentity> {
    resolve_runtime_identity_in(&mut rootfs, requested)
}

fn resolve_runtime_identity_in(
    rootfs: &mut impl ProcessRootfs,
    requested: Option<&str>,
) -> Result<RuntimeIdentity> {
    let passwd = runtime_passwd_entries(rootfs)?;
    let groups = runtime_group_entries(rootfs)?;
    let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(RuntimeIdentity {
            uid: 0,
            gid: 0,
            additional_gids: Vec::new(),
            home: None,
        });
    };
    if requested.matches(':').count() > 1 {
        return Err(BoxError::ConfigError(format!(
            "Invalid MicroVM user {requested:?}; expected user[:group]"
        )));
    }
    let (user_part, group_part) = requested
        .split_once(':')
        .map_or((requested, None), |(user, group)| (user, Some(group)));
    if user_part.is_empty() || group_part.is_some_and(str::is_empty) {
        return Err(BoxError::ConfigError(format!(
            "Invalid MicroVM user {requested:?}; user and group must be non-empty"
        )));
    }

    let passwd_entry = if user_part == "root" {
        passwd.iter().find(|entry| entry.uid == 0)
    } else if let Ok(uid) = user_part.parse::<u32>() {
        passwd.iter().find(|entry| entry.uid == uid)
    } else {
        Some(
            passwd
                .iter()
                .find(|entry| entry.name == user_part)
                .ok_or_else(|| {
                    BoxError::ConfigError(format!(
                "MicroVM user {user_part:?} is not present in the prepared rootfs /etc/passwd"
            ))
                })?,
        )
    };
    let uid = if user_part == "root" {
        0
    } else {
        user_part
            .parse::<u32>()
            .ok()
            .or_else(|| passwd_entry.map(|entry| entry.uid))
            .ok_or_else(|| {
                BoxError::ConfigError(format!("MicroVM user {user_part:?} cannot be resolved"))
            })?
    };
    let gid = match group_part {
        Some("root") => 0,
        Some(group) => group
            .parse::<u32>()
            .ok()
            .or_else(|| {
                groups
                    .iter()
                    .find(|entry| entry.name == group)
                    .map(|entry| entry.gid)
            })
            .ok_or_else(|| {
                BoxError::ConfigError(format!(
                    "MicroVM group {group:?} is not present in the prepared rootfs /etc/group"
                ))
            })?,
        None => passwd_entry.map(|entry| entry.gid).unwrap_or(0),
    };
    let username = if user_part.parse::<u32>().is_ok() || user_part == "root" {
        passwd_entry.map(|entry| entry.name.as_str())
    } else {
        Some(user_part)
    };
    let mut additional_gids = groups
        .iter()
        .filter(|entry| {
            entry.gid != gid
                && username
                    .is_some_and(|username| entry.members.iter().any(|member| member == username))
        })
        .map(|entry| entry.gid)
        .collect::<Vec<_>>();
    additional_gids.sort_unstable();
    additional_gids.dedup();

    let home = passwd_entry
        .and_then(|entry| entry.home.as_deref())
        .map(|home| normalize_linux_guest_path("/", home, "user home"))
        .transpose()?;
    Ok(RuntimeIdentity {
        uid,
        gid,
        additional_gids,
        home,
    })
}

fn runtime_passwd_entries(rootfs: &mut impl ProcessRootfs) -> Result<Vec<RuntimePasswdEntry>> {
    let Some(contents) = rootfs.read_text("etc/passwd")? else {
        return Ok(Vec::new());
    };
    Ok(contents
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            if fields.len() < 7 || fields[0].is_empty() {
                return None;
            }
            Some(RuntimePasswdEntry {
                name: fields[0].to_string(),
                uid: fields[2].parse().ok()?,
                gid: fields[3].parse().ok()?,
                home: (!fields[5].is_empty()).then(|| fields[5].to_string()),
            })
        })
        .collect())
}

fn runtime_group_entries(rootfs: &mut impl ProcessRootfs) -> Result<Vec<RuntimeGroupEntry>> {
    let Some(contents) = rootfs.read_text("etc/group")? else {
        return Ok(Vec::new());
    };
    Ok(contents
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            if fields.len() < 4 || fields[0].is_empty() {
                return None;
            }
            Some(RuntimeGroupEntry {
                name: fields[0].to_string(),
                gid: fields[2].parse().ok()?,
                members: fields[3]
                    .split(',')
                    .map(str::trim)
                    .filter(|member| !member.is_empty())
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_path::resolve_runtime_executable;

    #[test]
    fn runtime_owned_process_resolves_path_and_named_identity_from_rootfs() {
        let rootfs = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(rootfs.path().join("usr/local/bin")).unwrap();
        std::fs::create_dir_all(rootfs.path().join("etc")).unwrap();
        let executable = rootfs.path().join("usr/local/bin/example");
        std::fs::write(&executable, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(
            rootfs.path().join("etc/passwd"),
            "root:x:0:0:root:/root:/bin/sh\napp:x:1001:1002:App:/home/app:/bin/sh\n",
        )
        .unwrap();
        std::fs::write(
            rootfs.path().join("etc/group"),
            "app:x:1002:\nmetrics:x:1003:app\n",
        )
        .unwrap();

        assert_eq!(
            resolve_runtime_executable(
                rootfs.path(),
                "/workspace",
                "/usr/local/bin:/bin",
                "example"
            )
            .unwrap(),
            "/usr/local/bin/example"
        );
        assert_eq!(
            resolve_runtime_identity(rootfs.path(), Some("app")).unwrap(),
            RuntimeIdentity {
                uid: 1001,
                gid: 1002,
                additional_gids: vec![1003],
                home: Some("/home/app".to_string()),
            }
        );
    }

    #[test]
    fn runtime_owned_process_rejects_root_escape_and_unknown_named_user() {
        let rootfs = tempfile::tempdir().unwrap();
        assert!(normalize_linux_guest_path("/", "../../host", "test").is_err());
        assert!(resolve_runtime_identity(rootfs.path(), Some("missing")).is_err());
    }
}
