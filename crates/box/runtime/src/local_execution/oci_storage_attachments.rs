//! Bind Box-owned storage allocations into `koci.attachments.v2`.
//!
//! Volume lookup, authorization, and deletion stay in Box. The runtime only
//! receives immutable storage identities + caller-owned detach-only lifetime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kbox_core::{ExecutionManagerError, ExecutionManagerResult};
use koci_sdk::{
    CreateAttachments, OciBundle, StorageAccessMode, StorageAttachmentId, StorageCleanup,
    StorageOwnership,
};

use crate::{BoxRecord, VolumeStore};

/// Stable storage identity for the Box-owned Sandbox workspace bind.
const WORKSPACE_STORAGE_IDENTITY: &str = "kbox.workspace";

/// Attach Box-owned VolumeStore mounts and the implicit `/workspace` bind into
/// `koci.attachments.v2`.
///
/// Caller ownership + DetachOnly matches Box retaining deletion authority.
/// Staged caller bind aliases are attached separately via
/// [`attach_staged_caller_bind_aliases`]; anything else still fails
/// [`reject_unclassified_bind_mounts`].
pub(super) fn attach_box_owned_volume_storage(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    home_dir: &Path,
    record: &BoxRecord,
    anonymous_volumes: &[String],
) -> ExecutionManagerResult<CreateAttachments> {
    let mounts = oci_storage_mounts(bundle)?;
    let mut attachments = attach_named_and_anonymous_volumes(
        bundle,
        attachments,
        home_dir,
        record,
        anonymous_volumes,
        &mounts,
    )?;
    attachments = attach_workspace_bind(bundle, attachments, record, &mounts)?;
    Ok(attachments)
}

/// Classify Box-staged caller bind aliases under
/// `{box_dir}/sandbox/attachments/{slot}` into `koci.attachments.v2`.
///
/// Preparation rewrites external `-v` sources onto these O_PATH aliases; this
/// attaches the matching storage identity (`kbox.bind.{slot}`) so create no
/// longer sees an unclassified bind for the common Docker bind surface.
pub(super) fn attach_staged_caller_bind_aliases(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    home_dir: &Path,
    record: &BoxRecord,
) -> ExecutionManagerResult<CreateAttachments> {
    let alias_root = staged_caller_bind_alias_root(home_dir, &record.id);
    let Ok(alias_root) = alias_root.canonicalize() else {
        // No staged aliases for this generation (no external binds, or not yet
        // prepared). Leave attachments unchanged.
        return Ok(attachments);
    };

    let mounts = oci_storage_mounts(bundle)?;
    let already = classified_bind_indices(&attachments)?;
    let mut attachments = attachments;
    for (index, mount) in mounts.iter().enumerate() {
        if already.contains(&index) {
            continue;
        }
        let Some(source) = mount.source.as_ref() else {
            continue;
        };
        let Ok(canonical) = source.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(&alias_root) {
            continue;
        }
        let slot = bind_alias_slot(&alias_root, &canonical).ok_or_else(|| {
            ExecutionManagerError::InvalidRequest(format!(
                "OCI bind alias {} is not a direct slot under {}",
                canonical.display(),
                alias_root.display()
            ))
        })?;
        let identity = format!("kbox.bind.{slot}");
        attachments = attach_storage(bundle, attachments, index, &identity, mount.read_only)?;
    }
    Ok(attachments)
}

/// Fail closed when any OCI `type=bind` mount remains unclassified as storage
/// or secret after Box attachment assembly.
///
/// External binds outside Box-owned volumes, `/workspace`, managed Secrets, and
/// staged caller bind aliases must not reach runtime create.
pub(super) fn reject_unclassified_bind_mounts(
    bundle: &OciBundle,
    attachments: CreateAttachments,
) -> ExecutionManagerResult<CreateAttachments> {
    let mounts = oci_storage_mounts(bundle)?;
    let classified = classified_bind_indices(&attachments)?;
    for (index, mount) in mounts.iter().enumerate() {
        if mount.source.is_none() {
            continue;
        }
        if classified.contains(&index) {
            continue;
        }
        let destination = mount
            .destination
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("mount index {index}"));
        return Err(ExecutionManagerError::InvalidRequest(format!(
            "OCI rejects unclassified external bind mount at {destination}; \
             use a Box-owned named/anonymous volume, the /workspace bind, a managed Secret, \
             or a prepare-staged caller bind alias under sandbox/attachments"
        )));
    }
    Ok(attachments)
}

/// Require `{alias_root}/{slot}` with a four-digit slot basename (no nesting).
fn bind_alias_slot(alias_root: &Path, canonical: &Path) -> Option<String> {
    let relative = canonical.strip_prefix(alias_root).ok()?;
    let mut components = relative.components();
    let slot = match components.next()? {
        std::path::Component::Normal(slot) => slot.to_str()?.to_string(),
        _ => return None,
    };
    if components.next().is_some() {
        return None;
    }
    if slot.len() != 4 || !slot.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(slot)
}

/// Box-owned alias directory for prepare-rewritten caller binds.
///
/// Mirrors `host_fs::mount_alias_root` so attachment classification
/// compiles on non-Linux hosts even though alias staging itself is Linux-only.
fn staged_caller_bind_alias_root(home_dir: &Path, box_id: &str) -> PathBuf {
    home_dir
        .join("boxes")
        .join(box_id)
        .join("sandbox")
        .join("attachments")
}

fn classified_bind_indices(
    attachments: &CreateAttachments,
) -> ExecutionManagerResult<std::collections::HashSet<usize>> {
    let mut classified = std::collections::HashSet::new();
    for storage in attachments.storage() {
        if let Some(index) = mount_index_from_pointer(storage.mount().json_pointer()) {
            classified.insert(index);
        }
    }
    let encoded = serde_json::to_value(attachments).map_err(|error| {
        ExecutionManagerError::Internal(format!(
            "failed to inspect attachment secrets for bind classification: {error}"
        ))
    })?;
    if let Some(secrets) = encoded.get("secrets").and_then(|value| value.as_array()) {
        for secret in secrets {
            let pointer = secret
                .pointer("/configuration/jsonPointer")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    secret
                        .get("configuration")
                        .and_then(|value| value.get("jsonPointer"))
                        .and_then(|value| value.as_str())
                });
            if let Some(index) = pointer.and_then(mount_index_from_pointer) {
                classified.insert(index);
            }
        }
    }
    Ok(classified)
}

fn mount_index_from_pointer(pointer: &str) -> Option<usize> {
    let rest = pointer.strip_prefix("/mounts/")?;
    if rest.contains('/') {
        return None;
    }
    rest.parse().ok()
}

fn box_owned_volume_sources(
    home_dir: &Path,
    record: &BoxRecord,
    anonymous_volumes: &[String],
    raw_anonymous: bool,
) -> ExecutionManagerResult<HashMap<PathBuf, String>> {
    let mut owned_names = std::collections::HashSet::new();
    for name in record.volume_names.iter().chain(anonymous_volumes.iter()) {
        owned_names.insert(name.as_str());
    }
    if owned_names.is_empty() {
        return Ok(HashMap::new());
    }

    let store = VolumeStore::new(home_dir.join("volumes.json"), home_dir.join("volumes"));
    let volumes = store.load_strict().map_err(|error| {
        ExecutionManagerError::Unavailable(format!(
            "failed to load volume store for storage attachments: {error}"
        ))
    })?;

    let mut by_source = HashMap::<PathBuf, String>::new();
    for name in &owned_names {
        let Some(volume) = volumes.get(*name) else {
            return Err(ExecutionManagerError::Unavailable(format!(
                "volume '{name}' required by execution {} is missing from the volume store",
                record.id
            )));
        };
        let is_named = record.volume_names.iter().any(|named| named == *name);
        let raw = raw_anonymous && !is_named;
        let checked = if raw {
            Some(
                store
                    .require_anonymous(name, &record.id)
                    .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?,
            )
        } else {
            None
        };
        let volume = checked.as_ref().unwrap_or(volume);
        if volume.size_limit != 0 {
            return Err(ExecutionManagerError::InvalidRequest(format!(
                "volume {name:?} quota is not enforced by this OCI storage path"
            )));
        }
        let expected = home_dir.join("volumes").join(name);
        let metadata = std::fs::symlink_metadata(&expected)
            .map_err(|error| ExecutionManagerError::Unavailable(error.to_string()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ExecutionManagerError::InvalidRequest(
                "named volume source is not a plain managed directory".into(),
            ));
        }
        let source_path = if raw {
            crate::volume::anonymous_ext4_disk_path(Path::new(&volume.mount_point))
        } else {
            PathBuf::from(&volume.mount_point)
        };
        if raw {
            let metadata = std::fs::symlink_metadata(&source_path)
                .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(ExecutionManagerError::InvalidRequest(
                    "anonymous disk is not a plain raw image".into(),
                ));
            }
        }
        let source = source_path.canonicalize().map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "volume '{name}' mount point {} is not reachable for storage attachment: {error}",
                volume.mount_point
            ))
        })?;
        if raw && source != source_path {
            return Err(ExecutionManagerError::InvalidRequest(
                "anonymous raw image path aliases another source".into(),
            ));
        }
        let identity = if is_named || raw {
            if let Some(allocation) = volume.labels.get(crate::volume::ALLOCATION_ID_LABEL) {
                let allocation = uuid::Uuid::parse_str(allocation).map_err(|_| {
                    ExecutionManagerError::InvalidRequest(
                        "named volume allocation identity is invalid".into(),
                    )
                })?;
                format!("kbox.volume.v1.{}", allocation.simple())
            } else {
                if raw {
                    return Err(ExecutionManagerError::InvalidRequest(
                        "raw allocation has no store-issued identity".into(),
                    ));
                }
                chrono::DateTime::parse_from_rfc3339(&volume.created_at).map_err(|_| {
                    ExecutionManagerError::InvalidRequest(
                        "named volume has no valid allocation creation identity".into(),
                    )
                })?;
                use sha2::{Digest, Sha256};
                let encoded = serde_json::to_vec(&(&volume.name, &volume.created_at, &source))
                    .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
                format!("kbox.volume.v1.{:x}", Sha256::digest(encoded))
            }
        } else {
            (*name).to_string()
        };
        if let Some(previous) = by_source.insert(source.clone(), identity) {
            return Err(ExecutionManagerError::Internal(format!(
                "volumes '{previous}' and '{name}' share mount point {} for execution {}",
                source.display(),
                record.id
            )));
        }
    }

    Ok(by_source)
}

#[cfg(feature = "vm")]
pub(super) fn validate_box_named_volume_sources(
    home_dir: &Path,
    record: &BoxRecord,
) -> ExecutionManagerResult<()> {
    let metadata = record.managed_execution.as_ref().ok_or_else(|| {
        ExecutionManagerError::InvalidRequest(
            "named volume admission requires managed metadata".into(),
        )
    })?;
    if metadata.request.policy.volume_names != record.volume_names {
        return Err(ExecutionManagerError::InvalidRequest(
            "named volume declarations differ from the persisted request".into(),
        ));
    }
    if record.volume_names.is_empty() {
        return Ok(());
    }
    let named = box_owned_volume_sources(home_dir, record, &[], false)?;
    let mut bound = HashMap::<PathBuf, usize>::new();
    for value in &metadata.request.config.volumes {
        let source = crate::VmManager::parse_volume_spec(value)
            .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?
            .host_path;
        let canonical = source
            .canonicalize()
            .map_err(|error| ExecutionManagerError::Unavailable(error.to_string()))?;
        if named.contains_key(&canonical) {
            *bound.entry(canonical).or_default() += 1;
        }
    }
    if named.keys().any(|source| bound.get(source) != Some(&1)) {
        return Err(ExecutionManagerError::InvalidRequest(
            "each declared named volume requires one exact bind mount".into(),
        ));
    }
    Ok(())
}

fn attach_named_and_anonymous_volumes(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    home_dir: &Path,
    record: &BoxRecord,
    anonymous_volumes: &[String],
    mounts: &[OciStorageMount],
) -> ExecutionManagerResult<CreateAttachments> {
    let raw = attachments
        .raw_storage_images(bundle)
        .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
    let by_source = box_owned_volume_sources(home_dir, record, anonymous_volumes, raw)?;
    let mut attachments = attachments;
    let mut attached = 0usize;
    for (index, mount) in mounts.iter().enumerate() {
        let Some(source) = mount.source.as_ref() else {
            continue;
        };
        let Ok(canonical) = source.canonicalize() else {
            continue;
        };
        let Some(volume_name) = by_source.get(&canonical) else {
            continue;
        };
        attachments = attach_storage(bundle, attachments, index, volume_name, mount.read_only)?;
        attached += 1;
    }

    if attached != by_source.len() {
        return Err(ExecutionManagerError::Unavailable(format!(
            "execution {} planned {} Box-owned volume(s) but only attached {attached} OCI bind mount(s)",
            record.id,
            by_source.len()
        )));
    }
    Ok(attachments)
}

fn attach_workspace_bind(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    record: &BoxRecord,
    mounts: &[OciStorageMount],
) -> ExecutionManagerResult<CreateAttachments> {
    let workspace = record.box_dir.join("workspace");
    let Ok(workspace) = workspace.canonicalize() else {
        // No materialized workspace bind (tmpfs workspace or not yet prepared).
        return Ok(attachments);
    };
    for (index, mount) in mounts.iter().enumerate() {
        let Some(source) = mount.source.as_ref() else {
            continue;
        };
        let Ok(canonical) = source.canonicalize() else {
            continue;
        };
        if canonical != workspace {
            continue;
        }
        if mount.destination.as_deref() != Some(Path::new("/workspace")) {
            return Err(ExecutionManagerError::Internal(format!(
                "execution {} workspace source is mounted at {:?} instead of /workspace",
                record.id, mount.destination
            )));
        }
        return attach_storage(
            bundle,
            attachments,
            index,
            WORKSPACE_STORAGE_IDENTITY,
            mount.read_only,
        );
    }
    Ok(attachments)
}

fn attach_storage(
    bundle: &OciBundle,
    attachments: CreateAttachments,
    index: usize,
    identity: &str,
    read_only: bool,
) -> ExecutionManagerResult<CreateAttachments> {
    let identity = StorageAttachmentId::new(identity.to_string()).map_err(|error| {
        ExecutionManagerError::InvalidRequest(format!(
            "storage identity '{identity}' is invalid: {error}"
        ))
    })?;
    let access_mode = if read_only {
        StorageAccessMode::ReadOnly
    } else {
        StorageAccessMode::ReadWrite
    };
    attachments
        .attach_storage_mount(
            bundle,
            index,
            identity,
            access_mode,
            StorageOwnership::Caller,
            StorageCleanup::DetachOnly,
        )
        .map_err(|error| {
            ExecutionManagerError::InvalidRequest(format!(
                "failed to attach Box storage at OCI mount {index}: {error}"
            ))
        })
}

#[derive(Debug)]
struct OciStorageMount {
    source: Option<PathBuf>,
    destination: Option<PathBuf>,
    read_only: bool,
}

fn oci_storage_mounts(bundle: &OciBundle) -> ExecutionManagerResult<Vec<OciStorageMount>> {
    let raw = koci_sdk::raw_storage_images_requested(bundle)
        .map_err(|error| ExecutionManagerError::InvalidRequest(error.to_string()))?;
    let configuration: serde_json::Value =
        serde_json::from_str(bundle.config_json()).map_err(|error| {
            ExecutionManagerError::Internal(format!(
                "failed to decode OCI config for storage attachments: {error}"
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
            let source = mount
                .get("source")
                .and_then(|value| value.as_str())
                .map(PathBuf::from);
            let destination = mount
                .get("destination")
                .and_then(|value| value.as_str())
                .map(PathBuf::from);
            let options = mount
                .get("options")
                .and_then(|value| value.as_array())
                .map(|options| {
                    options
                        .iter()
                        .filter_map(|option| option.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let read_only = options.contains(&"ro");
            OciStorageMount {
                source: if typ == Some("bind") || (raw && typ == Some("ext4")) {
                    source
                } else {
                    None
                },
                destination,
                read_only,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kbox_core::volume::VolumeConfig;
    use koci_sdk::ProcessIo;
    use serde_json::json;

    pub(super) fn make_record(home_dir: &Path, id: &str) -> BoxRecord {
        serde_json::from_value(json!({
            "id": id,
            "short_id": &id[..8],
            "name": "managed-storage",
            "image": "alpine:latest",
            "status": "created",
            "pid": null,
            "cpus": 1,
            "memory_mb": 128,
            "volumes": [],
            "env": {},
            "cmd": ["sleep", "60"],
            "box_dir": home_dir.join("boxes").join(id),
            "console_log": home_dir.join("boxes").join(id).join("logs/console.log"),
            "created_at": "2026-07-15T00:00:00Z",
            "started_at": null,
            "auto_remove": false
        }))
        .unwrap()
    }

    #[test]
    fn attaches_named_volume_and_promotes_schema_to_v2() {
        let home = tempfile::tempdir().unwrap();
        let store = VolumeStore::new(
            home.path().join("volumes.json"),
            home.path().join("volumes"),
        );
        let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
        let source = PathBuf::from(&volume.mount_point).canonicalize().unwrap();

        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
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
                        "destination": "/data",
                        "type": "bind",
                        "source": source,
                        "options": ["rbind", "rprivate", "nosuid", "nodev", "rw"]
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let mut record = make_record(home.path(), "11111111-1111-4111-8111-111111111111");
        record.volume_names = vec!["dataset-a".to_string()];

        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        assert_eq!(attachments.schema_version(), "koci.attachments.v1");

        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        assert_eq!(attached.schema_version(), "koci.attachments.v2");
        assert_eq!(attached.storage().len(), 1);
        assert!(attached.storage()[0]
            .identity()
            .as_str()
            .starts_with("kbox.volume.v1."));
        assert_ne!(attached.storage()[0].identity().as_str(), "dataset-a");
        let retry = attach_box_owned_volume_storage(
            &bundle,
            CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap(),
            home.path(),
            &record,
            &[],
        )
        .unwrap();
        assert_eq!(
            attached.storage()[0].identity(),
            retry.storage()[0].identity()
        );
        assert_eq!(
            attached.storage()[0].access_mode(),
            StorageAccessMode::ReadWrite
        );
    }

    #[test]
    fn attaches_workspace_bind_as_box_owned_storage() {
        let home = tempfile::tempdir().unwrap();
        let id = "11111111-1111-4111-8111-111111111113";
        let record = make_record(home.path(), id);
        let workspace = record.box_dir.join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let source = workspace.canonicalize().unwrap();

        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/workspace",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "rprivate", "nosuid", "nodev", "rw"]
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        assert_eq!(attached.schema_version(), "koci.attachments.v2");
        assert_eq!(attached.storage().len(), 1);
        assert_eq!(
            attached.storage()[0].identity().as_str(),
            WORKSPACE_STORAGE_IDENTITY
        );
    }

    #[test]
    fn attaches_staged_caller_bind_alias_and_passes_reject_gate() {
        let home = tempfile::tempdir().unwrap();
        let id = "11111111-1111-4111-8111-111111111115";
        let record = make_record(home.path(), id);
        let alias_root = staged_caller_bind_alias_root(home.path(), id);
        let alias = alias_root.join("0000");
        std::fs::create_dir_all(&alias).unwrap();
        std::fs::write(alias.join("payload"), b"data").unwrap();
        let source = alias.canonicalize().unwrap();

        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/host-data",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "rprivate", "nosuid", "nodev", "ro"]
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        assert!(attached.storage().is_empty());
        let attached =
            attach_staged_caller_bind_aliases(&bundle, attached, home.path(), &record).unwrap();
        assert_eq!(attached.schema_version(), "koci.attachments.v2");
        assert_eq!(attached.storage().len(), 1);
        assert_eq!(attached.storage()[0].identity().as_str(), "kbox.bind.0000");
        assert_eq!(
            attached.storage()[0].access_mode(),
            StorageAccessMode::ReadOnly
        );
        let accepted = reject_unclassified_bind_mounts(&bundle, attached).unwrap();
        assert_eq!(accepted.storage().len(), 1);
    }

    #[test]
    fn still_rejects_external_binds_outside_alias_root() {
        let home = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let source = external.path().canonicalize().unwrap();
        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/external",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "rw"]
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let record = make_record(home.path(), "11111111-1111-4111-8111-111111111116");
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        let attached =
            attach_staged_caller_bind_aliases(&bundle, attached, home.path(), &record).unwrap();
        assert!(attached.storage().is_empty());
        let error = reject_unclassified_bind_mounts(&bundle, attached).unwrap_err();
        assert!(
            error.to_string().contains("unclassified external bind"),
            "{error}"
        );
    }

    #[test]
    fn rejects_external_binds_without_volume_ownership() {
        let home = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let source = external.path().canonicalize().unwrap();
        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
                "ociVersion": "1.3.0",
                "root": {"path": "rootfs"},
                "process": {
                    "cwd": "/",
                    "args": ["/bin/true"],
                    "user": {"uid": 0, "gid": 0}
                },
                "mounts": [{
                    "destination": "/external",
                    "type": "bind",
                    "source": source,
                    "options": ["rbind", "rw"]
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let record = make_record(home.path(), "11111111-1111-4111-8111-111111111112");
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        assert!(attached.storage().is_empty());
        let error = reject_unclassified_bind_mounts(&bundle, attached).unwrap_err();
        assert!(
            error.to_string().contains("unclassified external bind"),
            "{error}"
        );
    }

    #[test]
    fn accepts_volume_owned_binds_after_classification() {
        let home = tempfile::tempdir().unwrap();
        let store = VolumeStore::new(
            home.path().join("volumes.json"),
            home.path().join("volumes"),
        );
        let volume = store.create(VolumeConfig::new("dataset-a", "")).unwrap();
        let source = PathBuf::from(&volume.mount_point).canonicalize().unwrap();

        let bundle_dir = home.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("rootfs")).unwrap();
        let bundle = OciBundle::from_json(
            &bundle_dir,
            serde_json::to_string(&json!({
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
                    "options": ["rbind", "rprivate", "nosuid", "nodev", "rw"]
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let mut record = make_record(home.path(), "11111111-1111-4111-8111-111111111114");
        record.volume_names = vec!["dataset-a".to_string()];
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
        let attached =
            attach_box_owned_volume_storage(&bundle, attachments, home.path(), &record, &[])
                .unwrap();
        let accepted = reject_unclassified_bind_mounts(&bundle, attached).unwrap();
        assert_eq!(accepted.storage().len(), 1);
    }
}

#[cfg(test)]
#[path = "oci_named_storage_tests.rs"]
mod named_tests;
