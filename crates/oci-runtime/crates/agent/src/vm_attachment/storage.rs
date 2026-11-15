#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::collections::BTreeSet;

use koci_agent_protocol::{AgentVmAttachmentManifest, AGENT_VM_DIRECTORY_RELATIVE_ROOT};
use koci_sdk::{Error, ErrorCode, Result, StorageAccessMode};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
const MAX_GUEST_BLOCK_DEVICES: usize = 1_024;
const MAX_ENCODED_STORAGE_SOURCES: usize = 64 * 1024;

/// One manifest-authorized OCI mount source rewritten to a verified Guest disk.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UtilityVmStorageSource {
    mount_index: usize,
    configured_source: String,
    guest_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    environment_binding: Option<koci_sdk::MemorySecretEnvironmentBinding>,
}

impl UtilityVmStorageSource {
    pub(crate) const fn mount_index(&self) -> usize {
        self.mount_index
    }

    pub(crate) fn configured_source(&self) -> &str {
        &self.configured_source
    }

    pub(crate) fn guest_source(&self) -> &str {
        &self.guest_source
    }

    pub(crate) fn is_bind(&self) -> bool {
        self.guest_source
            .starts_with(&format!("{AGENT_VM_DIRECTORY_RELATIVE_ROOT}/"))
            || self.guest_source.starts_with(&format!(
                "{}/",
                koci_agent_protocol::AGENT_VM_FILES_RELATIVE_ROOT
            ))
    }

    fn validate(&self) -> Result<()> {
        if let Some(binding) = &self.environment_binding {
            binding.validate()?;
        }
        if self.configured_source.len() < 2
            || !self.configured_source.starts_with('/')
            || self.configured_source.contains('\0')
        {
            return Err(storage_error(
                "VM storage source has an invalid configured Host path",
            ));
        }
        let block = self.guest_source.strip_prefix("/dev/").is_some_and(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
        let directory = self
            .guest_source
            .strip_prefix(&format!("{AGENT_VM_DIRECTORY_RELATIVE_ROOT}/kdir-"))
            .is_some_and(|tag| {
                tag.len() == 24
                    && tag
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            });
        let file = self
            .guest_source
            .strip_prefix(&format!(
                "{}/kfile-",
                koci_agent_protocol::AGENT_VM_FILES_RELATIVE_ROOT
            ))
            .and_then(|tail| tail.strip_suffix("/file"))
            .is_some_and(|tag| {
                tag.len() == 24
                    && tag
                        .bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            });
        if !block && !directory && !file {
            return Err(storage_error(
                "VM storage source has an invalid Guest device or directory path",
            ));
        }
        Ok(())
    }
}

/// Non-sensitive metadata for opening an authorized file before entering the workload root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MemorySecretEnvironmentSource {
    binding: koci_sdk::MemorySecretEnvironmentBinding,
    protected_source: std::path::PathBuf,
}
impl MemorySecretEnvironmentSource {
    pub(crate) fn binding(&self) -> &koci_sdk::MemorySecretEnvironmentBinding {
        &self.binding
    }
    pub(crate) fn protected_source(&self) -> &std::path::Path {
        &self.protected_source
    }
}

/// Canonical exact-source rewrites inherited by one prepared container init.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct UtilityVmStorageSources(Vec<UtilityVmStorageSource>);

impl UtilityVmStorageSources {
    pub(crate) fn from_json(encoded: &str) -> Result<Self> {
        if encoded.len() > MAX_ENCODED_STORAGE_SOURCES {
            return Err(storage_error(
                "encoded VM storage sources exceed their bounded size",
            ));
        }
        let sources: Self = serde_json::from_str(encoded).map_err(|error| {
            storage_error(format!("failed to decode VM storage sources: {error}"))
        })?;
        sources.validate()?;
        Ok(sources)
    }

    pub(crate) fn to_json(&self) -> Result<String> {
        self.validate()?;
        let encoded = serde_json::to_string(self).map_err(|error| {
            storage_error(format!("failed to encode VM storage sources: {error}"))
        })?;
        if encoded.len() > MAX_ENCODED_STORAGE_SOURCES {
            return Err(storage_error(
                "encoded VM storage sources exceed their bounded size",
            ));
        }
        Ok(encoded)
    }

    pub(crate) fn as_slice(&self) -> &[UtilityVmStorageSource] {
        &self.0
    }

    pub(crate) fn memory_secret_environment_sources(
        &self,
        bundle: &std::path::Path,
    ) -> Vec<MemorySecretEnvironmentSource> {
        self.0
            .iter()
            .filter_map(|source| {
                source
                    .environment_binding
                    .as_ref()
                    .map(|binding| MemorySecretEnvironmentSource {
                        binding: binding.clone(),
                        protected_source: bundle.join(&source.guest_source),
                    })
            })
            .collect()
    }

    fn validate(&self) -> Result<()> {
        if self.0.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(storage_error(
                "VM storage source rewrites must be unique and canonically ordered",
            ));
        }
        let mut mount_indices = BTreeSet::new();
        let mut guest_sources = BTreeSet::new();
        for source in &self.0 {
            source.validate()?;
            if !mount_indices.insert(source.mount_index) {
                return Err(storage_error(format!(
                    "OCI mount index {} has more than one VM storage source",
                    source.mount_index
                )));
            }
            if !guest_sources.insert(source.guest_source.as_str()) {
                return Err(storage_error(format!(
                    "Guest storage source {} is assigned more than once",
                    source.guest_source
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestBlockDevice {
    name: String,
    serial: String,
    size: u64,
    read_only: bool,
}

#[cfg(target_os = "linux")]
pub(super) fn configure_guest_storage(
    manifest: &AgentVmAttachmentManifest,
    bundle: &koci_sdk::OciBundle,
) -> Result<UtilityVmStorageSources> {
    let mut sources = if manifest.storage().is_empty() {
        UtilityVmStorageSources::default()
    } else {
        sources_from_inventory(manifest, &inventory_guest_block_devices()?)?
    };
    let bindings = koci_sdk::MemorySecretFiles::from_bundle(bundle)?
        .map(|memory| memory.environment_bindings(bundle))
        .transpose()?
        .unwrap_or_default();
    for directory in manifest.directories() {
        sources.0.push(UtilityVmStorageSource {
            mount_index: directory.mount_index()?,
            configured_source: directory.host_source().to_string(),
            guest_source: directory.guest_mount_relative_path(),
            environment_binding: None,
        });
    }
    for file in manifest.files() {
        sources.0.push(UtilityVmStorageSource {
            mount_index: file.mount_index()?,
            configured_source: file.host_source().to_string(),
            guest_source: file.guest_file_relative_path(),
            environment_binding: bindings
                .iter()
                .find(|binding| {
                    bundle
                        .spec()
                        .mounts()
                        .as_ref()
                        .and_then(|mounts| mounts.get(file.mount_index().ok()?))
                        .and_then(|mount| mount.destination().to_str())
                        == Some(binding.path())
                })
                .cloned(),
        });
    }
    sources.0.sort();
    sources.validate()?;
    Ok(sources)
}

fn sources_from_inventory(
    manifest: &AgentVmAttachmentManifest,
    inventory: &[GuestBlockDevice],
) -> Result<UtilityVmStorageSources> {
    let mut sources = Vec::with_capacity(manifest.storage().len());
    for attachment in manifest.storage() {
        let device = device_from_inventory(
            inventory,
            attachment.source_identity(),
            attachment.access_mode() == StorageAccessMode::ReadOnly,
        )?;
        sources.push(UtilityVmStorageSource {
            mount_index: attachment.mount_index()?,
            configured_source: attachment.host_source().to_string(),
            guest_source: format!("/dev/{}", device.name),
            environment_binding: None,
        });
    }
    sources.sort();
    let sources = UtilityVmStorageSources(sources);
    sources.validate()?;
    Ok(sources)
}

fn device_from_inventory(
    inventory: &[GuestBlockDevice],
    identity: koci_agent_protocol::AgentVmStorageSourceIdentity,
    read_only: bool,
) -> Result<&GuestBlockDevice> {
    let serial = identity.virtio_serial();
    let mut matches = inventory.iter().filter(|device| device.serial == serial);
    let device = matches
        .next()
        .ok_or_else(|| storage_error(format!("authorized disk serial {serial} is absent")))?;
    if matches.next().is_some() || device.size != identity.size() || device.read_only != read_only {
        return Err(storage_error("Guest disk identity is ambiguous or its size/access differs from the exact Host binding"));
    }
    Ok(device)
}

#[cfg(target_os = "linux")]
pub(super) fn configure_guest_rootfs(manifest: &AgentVmAttachmentManifest) -> Result<()> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    let Some(rootfs) = manifest.rootfs_image() else {
        return Ok(());
    };
    let inventory = inventory_guest_block_devices()?;
    let device = device_from_inventory(&inventory, rootfs.source_identity(), false)?;
    let destination = std::path::Path::new(manifest.guest_bundle().as_str()).join("rootfs");
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(&destination)
        .map_err(|error| storage_error(error.to_string()))?;
    if std::fs::read_dir(&destination)
        .map_err(|error| storage_error(error.to_string()))?
        .next()
        .is_some()
    {
        return Err(storage_error(
            "workload rootfs mountpoint must be an empty directory",
        ));
    }
    let source = CString::new(format!("/dev/{}", device.name))
        .map_err(|error| storage_error(error.to_string()))?;
    let target = CString::new(format!("/proc/self/fd/{}", directory.as_raw_fd()))
        .map_err(|error| storage_error(error.to_string()))?;
    // SAFETY: verified block device and descriptor-pinned empty mountpoint remain live for mount.
    if unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            c"ext4".as_ptr(),
            libc::MS_NOSUID | libc::MS_NODEV,
            std::ptr::null(),
        )
    } != 0
    {
        return Err(storage_error(format!(
            "failed to mount generation-owned workload rootfs: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inventory_guest_block_devices() -> Result<Vec<GuestBlockDevice>> {
    use std::os::unix::fs::FileTypeExt;
    use std::path::Path;

    let root = Path::new("/sys/class/block");
    let entries = std::fs::read_dir(root).map_err(|error| {
        storage_error(format!(
            "failed to inventory Guest block devices at {}: {error}",
            root.display()
        ))
    })?;
    let mut devices = Vec::new();
    for entry in entries.take(MAX_GUEST_BLOCK_DEVICES + 1) {
        let entry = entry.map_err(|error| {
            storage_error(format!(
                "failed to read a Guest block-device entry: {error}"
            ))
        })?;
        if devices.len() == MAX_GUEST_BLOCK_DEVICES {
            return Err(storage_error(format!(
                "Guest exposes more than {MAX_GUEST_BLOCK_DEVICES} block devices"
            )));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| storage_error("Guest block-device inventory contains a non-UTF-8 name"))?;
        if entry.path().join("partition").exists() {
            continue;
        }
        let serial = match std::fs::read_to_string(entry.path().join("serial")) {
            Ok(serial) => serial
                .trim_matches(|character: char| character.is_whitespace() || character == '\0')
                .to_string(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(storage_error(format!(
                    "failed to read Guest block-device {name} serial: {error}"
                )));
            }
        };
        if serial.is_empty() {
            continue;
        }
        let sectors = read_u64(
            entry.path().join("size"),
            &format!("Guest disk {name} size"),
        )?;
        let size = sectors
            .checked_mul(512)
            .ok_or_else(|| storage_error(format!("Guest disk {name} byte size overflows u64")))?;
        let read_only = match read_u64(entry.path().join("ro"), &format!("Guest disk {name} ro"))? {
            0 => false,
            1 => true,
            value => {
                return Err(storage_error(format!(
                    "Guest disk {name} exposes invalid read-only state {value}"
                )));
            }
        };
        let device_path = Path::new("/dev").join(&name);
        let metadata = std::fs::symlink_metadata(&device_path).map_err(|error| {
            storage_error(format!(
                "Guest disk {name} has no matching device node {}: {error}",
                device_path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_block_device() {
            return Err(storage_error(format!(
                "Guest disk {name} does not map to a plain block device at {}",
                device_path.display()
            )));
        }
        devices.push(GuestBlockDevice {
            name,
            serial,
            size,
            read_only,
        });
    }
    Ok(devices)
}

#[cfg(target_os = "linux")]
fn read_u64(path: std::path::PathBuf, label: &str) -> Result<u64> {
    let encoded = std::fs::read_to_string(&path).map_err(|error| {
        storage_error(format!(
            "failed to read {label} at {}: {error}",
            path.display()
        ))
    })?;
    encoded
        .trim()
        .parse::<u64>()
        .map_err(|error| storage_error(format!("{label} is not an unsigned integer: {error}")))
}

fn storage_error(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::FailedPrecondition, message).for_operation("bootstrap-agent-vm-storage")
}

#[cfg(test)]
mod tests {
    #[test]
    fn directory_sources_reject_paths_outside_the_reserved_bundle_subtree() {
        let valid = format!(".koci-directories/kdir-{}", "a".repeat(24));
        for source in [
            valid.as_str(),
            "../../etc",
            "/etc",
            ".koci-directories/../secret",
            ".koci-directories/kdir-a",
            ".koci-directories/kdir-AAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let encoded = serde_json::json!([{"mountIndex":0, "configuredSource":"/srv/workspace",
                "guestSource":source}])
            .to_string();
            let result = super::UtilityVmStorageSources::from_json(&encoded);
            if source == valid {
                assert!(result.unwrap().as_slice()[0].is_bind());
            } else {
                assert!(result.is_err(), "{source}");
            }
        }
    }

    #[test]
    fn file_sources_admit_only_the_exact_alias_under_the_reserved_projection() {
        let valid = format!(".koci-files/kfile-{}/file", "a".repeat(24));
        for source in [
            valid.as_str(),
            ".koci-files/kfile-aaaaaaaaaaaaaaaaaaaaaaaa",
            ".koci-files/kfile-aaaaaaaaaaaaaaaaaaaaaaaa/sibling",
            ".koci-files/kfile-aaaaaaaaaaaaaaaaaaaaaaaa/../file",
            ".koci-files/kfile-AAAAAAAAAAAAAAAAAAAAAAAA/file",
            ".koci-files/../../caller/file",
        ] {
            let encoded=serde_json::json!([{"mountIndex":0,"configuredSource":"/caller/file","guestSource":source}]).to_string();
            let result = super::UtilityVmStorageSources::from_json(&encoded);
            if source == valid {
                assert!(result.unwrap().as_slice()[0].is_bind());
            } else {
                assert!(result.is_err(), "{source}");
            }
        }
    }

    use koci_agent_protocol::{
        AgentVmAttachmentManifest, AgentVmStorageAttachment, AgentVmStorageSourceIdentity,
        GuestPath,
    };
    use koci_sdk::{
        oci_spec::runtime::Spec, ContainerId, ContainerTarget, CreateAttachments, Generation,
        OciBundle, ProcessIo, StorageAccessMode, StorageAttachmentId, StorageCleanup,
        StorageOwnership,
    };
    use serde_json::json;

    use super::{
        device_from_inventory, sources_from_inventory, GuestBlockDevice, UtilityVmStorageSources,
    };

    #[test]
    fn workload_disk_inventory_requires_unique_exact_writable_identity() {
        let identity =
            koci_agent_protocol::AgentVmStorageSourceIdentity::new(1, 0, 41, 4096).unwrap();
        let device = GuestBlockDevice {
            name: "vdb".into(),
            serial: identity.virtio_serial(),
            size: 4096,
            read_only: false,
        };
        assert_eq!(
            device_from_inventory(std::slice::from_ref(&device), identity, false).unwrap(),
            &device
        );
        assert!(device_from_inventory(&[], identity, false).is_err());
        assert!(device_from_inventory(&[device.clone(), device.clone()], identity, false).is_err());
        for altered in [
            GuestBlockDevice {
                read_only: true,
                ..device.clone()
            },
            GuestBlockDevice {
                size: 8192,
                ..device.clone()
            },
            GuestBlockDevice {
                serial: "other-generation".into(),
                ..device
            },
        ] {
            assert!(device_from_inventory(&[altered], identity, false).is_err());
        }
    }

    fn manifest() -> AgentVmAttachmentManifest {
        let mut value = serde_json::to_value(Spec::default()).expect("default spec");
        value["mounts"] = json!([{
            "destination": "/data",
            "type": "ext4",
            "source": "/srv/authorized.raw",
            "options": ["ro"]
        }]);
        let bundle = OciBundle::from_json(
            std::env::temp_dir().join("kagent-storage-inventory"),
            value.to_string(),
        )
        .expect("bundle");
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
            .expect("attachments")
            .attach_storage_mount(
                &bundle,
                0,
                StorageAttachmentId::new("volume-1").expect("identity"),
                StorageAccessMode::ReadOnly,
                StorageOwnership::Caller,
                StorageCleanup::DetachOnly,
            )
            .expect("storage attachment");
        let digest = attachments.digest().expect("digest");
        let public = &attachments.storage()[0];
        let storage = AgentVmStorageAttachment::new(
            public.identity().clone(),
            public.mount().clone(),
            public.access_mode(),
            public.ownership(),
            public.cleanup(),
            "/srv/authorized.raw",
            AgentVmStorageSourceIdentity::new(17, 0, 4_203, 4096).expect("identity"),
            &digest,
        )
        .expect("transport");
        AgentVmAttachmentManifest::new(
            ContainerTarget::exact(ContainerId::new("stored").unwrap(), Generation(1)),
            GuestPath::new("/run/koci-runtime/bundle").unwrap(),
            bundle.config_digest(),
            digest,
            Vec::new(),
            vec![storage],
        )
        .expect("manifest")
    }

    #[test]
    fn maps_serial_without_trusting_guest_enumeration_order() {
        let manifest = manifest();
        let sources = sources_from_inventory(
            &manifest,
            &[
                GuestBlockDevice {
                    name: "vda".into(),
                    serial: "system".into(),
                    size: 64 * 1024 * 1024,
                    read_only: true,
                },
                GuestBlockDevice {
                    name: "vdc".into(),
                    serial: "1704203".into(),
                    size: 4096,
                    read_only: true,
                },
            ],
        )
        .expect("mapped source");
        assert_eq!(sources.as_slice()[0].mount_index(), 0);
        assert_eq!(sources.as_slice()[0].guest_source(), "/dev/vdc");
        let encoded = sources.to_json().expect("encoded sources");
        assert_eq!(
            UtilityVmStorageSources::from_json(&encoded).unwrap(),
            sources
        );
    }

    #[test]
    fn rejects_missing_ambiguous_size_and_access_drift() {
        let manifest = manifest();
        assert!(sources_from_inventory(&manifest, &[]).is_err());
        let exact = GuestBlockDevice {
            name: "vdb".into(),
            serial: "1704203".into(),
            size: 4096,
            read_only: true,
        };
        assert!(sources_from_inventory(&manifest, &[exact.clone(), exact.clone()]).is_err());
        assert!(sources_from_inventory(
            &manifest,
            &[GuestBlockDevice {
                size: 8192,
                ..exact.clone()
            }]
        )
        .is_err());
        assert!(sources_from_inventory(
            &manifest,
            &[GuestBlockDevice {
                read_only: false,
                ..exact
            }]
        )
        .is_err());
    }
}
