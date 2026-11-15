//! Complete the portable bundle using explicit Box storage and resource intent.

use std::path::PathBuf;

use kbox_core::{BoxConfig, BoxError, Result};
use oci_spec::runtime::{
    Capability, LinuxCpuBuilder, LinuxMemoryBuilder, LinuxPidsBuilder, LinuxResourcesBuilder,
    MountBuilder, Spec,
};

use super::VmManager;
use kbox_core::vmm::InstanceSpec;

#[path = "oci_hvf_tmpfs.rs"]
mod tmpfs;

#[path = "oci_hvf_rlimits.rs"]
mod rlimits;

fn error(value: impl std::fmt::Display) -> BoxError {
    BoxError::ConfigError(format!("HVF OCI configuration: {value}"))
}

fn sys_admin(value: &str) -> bool {
    crate::oci_portable_spec::normalize_capability_name(value) == "SYS_ADMIN"
}

pub(crate) fn validate(config: &BoxConfig) -> Result<koci_sdk::VmConfig> {
    let configuration = koci_sdk::VmConfig::new(
        u8::try_from(config.resources.vcpus).map_err(error)?,
        config.resources.memory_mb,
    )
    .map_err(error)?;
    crate::rootfs::Ext4ArtifactOptions::from_disk_mib(config.resources.disk_mb, [0; 16])?;
    tmpfs::mounts(config)?;
    rlimits::limits(config)?;
    workload_capabilities(config)?;
    crate::host_fs::oci::OciResources::from_box_config(config)?;
    if !config.workspace.as_os_str().is_empty() {
        let path = &config.workspace;
        let metadata = std::fs::symlink_metadata(path).map_err(error)?;
        if !path.is_absolute() || !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(error("workspace must be a plain absolute directory"));
        }
    }
    for value in &config.volumes {
        let path = VmManager::parse_volume_spec(value)?.host_path;
        let metadata = std::fs::symlink_metadata(&path).map_err(error)?;
        if metadata.is_file() {
            crate::local_execution::oci_file_grants::observe(&path, false)?;
        } else if !path.is_absolute() || !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(error(
                "caller source must be a plain absolute directory or regular file",
            ));
        }
    }
    Ok(configuration)
}

pub(crate) fn workload_capabilities(
    config: &BoxConfig,
) -> Result<oci_spec::runtime::LinuxCapabilities> {
    let additions: Vec<_> = config
        .cap_add
        .iter()
        .filter(|name| !sys_admin(name))
        .cloned()
        .collect();
    let drops: Vec<_> = config
        .cap_drop
        .iter()
        .filter(|name| !sys_admin(name))
        .cloned()
        .collect();
    let mut capabilities =
        crate::oci_portable_spec::compile_workload_capabilities(&additions, &drops)?;
    if config.cap_add.iter().any(|name| sys_admin(name))
        && !config.cap_drop.iter().any(|name| {
            sys_admin(name) || crate::oci_portable_spec::normalize_capability_name(name) == "ALL"
        })
    {
        let mut granted = capabilities.bounding().clone().unwrap_or_default();
        granted.insert(Capability::SysAdmin);
        capabilities.set_bounding(Some(granted.clone()));
        capabilities.set_effective(Some(granted.clone()));
        capabilities.set_permitted(Some(granted));
    }
    Ok(capabilities)
}

pub(super) fn complete(spec: &mut Spec, config: &BoxConfig, instance: &InstanceSpec) -> Result<()> {
    complete_with_files(spec, config, instance, &std::collections::BTreeMap::new())
}

pub(super) fn complete_with_files(
    spec: &mut Spec,
    config: &BoxConfig,
    instance: &InstanceSpec,
    files: &std::collections::BTreeMap<PathBuf, koci_sdk::FileSourceIdentity>,
) -> Result<()> {
    complete_with_anonymous(spec, config, instance, files, &[])
}

pub(super) fn complete_with_anonymous(
    spec: &mut Spec,
    config: &BoxConfig,
    instance: &InstanceSpec,
    files: &std::collections::BTreeMap<PathBuf, koci_sdk::FileSourceIdentity>,
    anonymous: &[(PathBuf, String)],
) -> Result<()> {
    let configuration = validate(config)?;
    complete_vm_resources(spec, configuration)?;
    spec.root_mut()
        .as_mut()
        .ok_or_else(|| error("missing portable root config"))?
        .set_readonly(Some(config.read_only));
    VmManager::validate_macos_directory_exports(instance, files)?;
    spec.annotations_mut().get_or_insert_default().insert(
        koci_sdk::RUNTIME_BUNDLE_HANDOFF_EXTENSION.into(),
        koci_sdk::RUNTIME_BUNDLE_HANDOFF_MOVE_V2.into(),
    );
    let mut destinations = vec![PathBuf::from("/workspace")];
    for volume in &config.volumes {
        destinations.push(PathBuf::from(
            VmManager::parse_volume_spec(volume)?.guest_path,
        ));
    }
    destinations.extend(anonymous.iter().map(|(_, target)| PathBuf::from(target)));
    if destinations.len() != instance.fs_mounts.len() {
        return Err(error(
            "unclassified filesystem export differs from the exact planned mounts",
        ));
    }
    let mounts = spec.mounts_mut().get_or_insert_default();
    mounts.push(
        MountBuilder::default()
            .destination("/sys/fs/cgroup")
            .typ("cgroup")
            .source("cgroup")
            .options(vec![
                "nosuid".into(),
                "nodev".into(),
                "noexec".into(),
                "ro".into(),
            ])
            .build()
            .map_err(error)?,
    );
    let mut file_grants = Vec::new();
    for (index, (mount, destination)) in instance.fs_mounts.iter().zip(destinations).enumerate() {
        if let Some((source, target)) = index
            .checked_sub(1 + config.volumes.len())
            .and_then(|offset| anonymous.get(offset))
        {
            VmManager::validate_guest_mount_path(target)?;
            if mount.host_path != *source || mount.read_only {
                return Err(error(
                    "anonymous raw disk source or access differs from its planned allocation",
                ));
            }
            let disk = crate::volume::anonymous_ext4_disk_path(source);
            let metadata = std::fs::symlink_metadata(&disk).map_err(error)?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || disk.canonicalize().map_err(error)? != disk
            {
                return Err(error("anonymous raw disk must be a plain canonical image"));
            }
            mounts.push(
                MountBuilder::default()
                    .destination(destination)
                    .typ("ext4")
                    .source(disk)
                    .options(vec!["rw".into(), "nosuid".into(), "nodev".into()])
                    .build()
                    .map_err(error)?,
            );
            continue;
        }
        let metadata = std::fs::symlink_metadata(&mount.host_path).map_err(error)?;
        let is_file = metadata.is_file() || files.contains_key(&mount.host_path);
        if is_file {
            let actual = crate::local_execution::oci_file_grants::observe(&mount.host_path, false)?;
            if files.get(&mount.host_path) != Some(&actual) {
                return Err(error("file bind lacks its reserved source incarnation"));
            }
            if instance.fs_mounts.iter().any(|other| {
                other.host_path != mount.host_path && mount.host_path.starts_with(&other.host_path)
            }) {
                return Err(error("file source overlaps a directory grant"));
            }
            file_grants.push(
                koci_sdk::FileGrant::new(
                    mounts.len(),
                    koci_sdk::StorageAttachmentId::new(format!("kbox.hvf.mount.{}", mounts.len()))
                        .map_err(error)?,
                    actual,
                    if mount.read_only {
                        koci_sdk::StorageAccessMode::ReadOnly
                    } else {
                        koci_sdk::StorageAccessMode::ReadWrite
                    },
                )
                .map_err(error)?,
            );
        }
        mounts.push(
            MountBuilder::default()
                .destination(destination)
                .typ("bind")
                .source(mount.host_path.clone())
                .options(vec![
                    if is_file { "bind" } else { "rbind" }.into(),
                    if mount.read_only { "ro" } else { "rw" }.into(),
                    "nosuid".into(),
                    "nodev".into(),
                ])
                .build()
                .map_err(error)?,
        );
    }
    if file_grants.len() != files.len() {
        return Err(error("unclassified reserved caller file"));
    }
    if !file_grants.is_empty() {
        let grants = koci_sdk::FileGrants::new(file_grants).map_err(error)?;
        spec.annotations_mut().get_or_insert_default().insert(
            koci_sdk::FILE_GRANTS_EXTENSION.into(),
            grants.annotation_value().map_err(error)?,
        );
    }
    if !anonymous.is_empty() {
        spec.annotations_mut().get_or_insert_default().insert(
            koci_sdk::RAW_STORAGE_IMAGES_EXTENSION.into(),
            koci_sdk::RAW_STORAGE_IMAGES_VALUE.into(),
        );
    }
    let tmpfs = tmpfs::mounts(config)?;
    for mount in &tmpfs {
        if anonymous
            .iter()
            .any(|(_, path)| tmpfs::overlaps(&mount.destination().to_string_lossy(), path))
        {
            return Err(error("tmpfs overlaps an image volume"));
        }
    }
    spec.mounts_mut().get_or_insert_default().extend(tmpfs);
    let process = spec
        .process_mut()
        .as_mut()
        .ok_or_else(|| error("missing portable process"))?;
    apply_workload_capabilities(config, process)?;
    apply_workload_rlimits(config, process)?;
    let limits = crate::host_fs::oci::OciResources::from_box_config(config)?;
    let mut cpu = LinuxCpuBuilder::default()
        .quota(limits.cpu_quota)
        .period(limits.cpu_period);
    if let Some(shares) = limits.cpu_shares {
        cpu = cpu.shares(shares);
    }
    if let Some(cpus) = limits.cpuset_cpus {
        cpu = cpu.cpus(cpus);
    }
    let mut memory = LinuxMemoryBuilder::default().limit(limits.memory_limit);
    if let Some(value) = limits.memory_reservation {
        memory = memory.reservation(value);
    }
    if let Some(value) = limits.memory_swap {
        memory = memory.swap(value);
    }
    let resources = LinuxResourcesBuilder::default()
        .cpu(cpu.build().map_err(error)?)
        .memory(memory.build().map_err(error)?)
        .pids(
            LinuxPidsBuilder::default()
                .limit(limits.pids_limit)
                .build()
                .map_err(error)?,
        )
        .build()
        .map_err(error)?;
    spec.linux_mut()
        .as_mut()
        .ok_or_else(|| error("missing portable Linux config"))?
        .set_resources(Some(resources));
    Ok(())
}

fn complete_vm_resources(spec: &mut Spec, configuration: koci_sdk::VmConfig) -> Result<()> {
    if spec
        .annotations()
        .as_ref()
        .is_some_and(|annotations| annotations.contains_key(koci_sdk::VM_RESOURCES_EXTENSION))
    {
        return Err(error(
            "VM resource annotations must originate from the Box resource request",
        ));
    }
    if configuration != koci_sdk::VmConfig::default() {
        spec.annotations_mut().get_or_insert_default().insert(
            koci_sdk::VM_RESOURCES_EXTENSION.into(),
            configuration.annotation_value().map_err(error)?,
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "oci_hvf_vm_config_tests.rs"]
mod vm_config_tests;

pub(crate) fn apply_workload_capabilities(
    config: &BoxConfig,
    process: &mut oci_spec::runtime::Process,
) -> Result<()> {
    let mut capabilities = workload_capabilities(config)?;
    if process.user().uid() != 0 {
        // Nonroot exec retains ambient values; never carry default capabilities implicitly.
        let explicit: std::collections::HashSet<_> = capabilities
            .permitted()
            .iter()
            .flatten()
            .filter(|capability| {
                config.cap_add.iter().any(|name| {
                    crate::oci_portable_spec::normalize_capability_name(name)
                        == crate::oci_portable_spec::normalize_capability_name(
                            &capability.to_string(),
                        )
                })
            })
            .copied()
            .collect();
        capabilities.set_inheritable(Some(explicit.clone()));
        capabilities.set_ambient(Some(explicit));
    }
    process.set_capabilities(Some(capabilities));
    Ok(())
}

pub(crate) fn apply_workload_rlimits(
    config: &BoxConfig,
    process: &mut oci_spec::runtime::Process,
) -> Result<()> {
    rlimits::apply(config, process)
}

pub(super) fn validate_tmpfs_image_volumes(
    config: &BoxConfig,
    image_volumes: &[String],
) -> Result<()> {
    for mount in tmpfs::mounts(config)? {
        let target = mount.destination().to_string_lossy();
        for path in image_volumes {
            let path = VmManager::normalize_guest_mount_path(path)?;
            if target != path && tmpfs::overlaps(&target, &path) {
                return Err(error("tmpfs overlaps an image volume subtree"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "oci_hvf_spec_tests.rs"]
mod tests;

pub(crate) fn complete_memory_secret_files(
    spec: &mut Spec,
    root: Option<&std::path::Path>,
) -> Result<()> {
    let Some(root) = root else { return Ok(()) };
    crate::BoxTransientSecretStore::new(root)
        .require_ready_sync()
        .map_err(error)?;
    let indices = spec
        .mounts()
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(index, mount)| {
            mount
                .source()
                .as_ref()
                .filter(|path| path.starts_with(root))
                .map(|_| index)
        })
        .collect();
    let environment = spec
        .process()
        .as_ref()
        .and_then(|process| process.env().as_deref())
        .unwrap_or_default();
    let bindings = koci_sdk::MemorySecretEnvironmentBinding::from_environment(environment)
        .map_err(error)?
        .is_some();
    let raw_storage = spec.annotations().as_ref().is_some_and(|annotations| {
        annotations.contains_key(koci_sdk::RAW_STORAGE_IMAGES_EXTENSION)
    });
    let memory = if raw_storage {
        koci_sdk::MemorySecretFiles::new_for_raw_storage(indices)
    } else if bindings {
        koci_sdk::MemorySecretFiles::new_for_environment(indices)
    } else {
        koci_sdk::MemorySecretFiles::new(indices)
    }
    .map_err(error)?;
    spec.annotations_mut().get_or_insert_default().insert(
        koci_sdk::MEMORY_SECRET_FILES_EXTENSION.into(),
        memory.annotation_value().map_err(error)?,
    );
    Ok(())
}
