use super::{invalid_attachment, CreateAttachments};
use crate::{OciBundle, Result};

/// Required opt-in for raw ext4 disks in the dedicated HVF attachment profile.
pub const RAW_STORAGE_IMAGES_EXTENSION: &str = "dev.kbox.raw-storage-images";
/// First combined raw storage/rootfs/file transport contract.
pub const RAW_STORAGE_IMAGES_VERSION: u16 = 1;
/// Exact annotation value; this token grants no storage by itself.
pub const RAW_STORAGE_IMAGES_VALUE: &str = "raw-ext4-v1";

/// Decode the explicit bounded token without authorizing any mount source.
pub fn raw_storage_images_requested(bundle: &OciBundle) -> Result<bool> {
    let value = bundle
        .spec()
        .annotations()
        .as_ref()
        .and_then(|annotations| annotations.get(RAW_STORAGE_IMAGES_EXTENSION));
    let Some(value) = value else { return Ok(false) };
    if value != RAW_STORAGE_IMAGES_VALUE
        || !bundle
            .spec()
            .mounts()
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|mount| mount.typ().as_deref() == Some("ext4"))
    {
        return Err(invalid_attachment(
            "raw storage images require the exact supported token and an ext4 mount",
        ));
    }
    Ok(true)
}

impl CreateAttachments {
    /// Validate that raw-image intent and its required capability remain paired.
    pub fn raw_storage_images(&self, bundle: &OciBundle) -> Result<bool> {
        let requested = raw_storage_images_requested(bundle)?;
        if requested != self.extensions.contains_key(RAW_STORAGE_IMAGES_EXTENSION)
            || self
                .extensions
                .get(RAW_STORAGE_IMAGES_EXTENSION)
                .is_some_and(|extension| {
                    extension.version() != RAW_STORAGE_IMAGES_VERSION || !extension.required()
                })
        {
            return Err(invalid_attachment(
                "raw storage images require their exact required extension",
            ));
        }
        Ok(requested)
    }
}

#[cfg(test)]
#[path = "raw_storage_images_tests.rs"]
mod tests;
