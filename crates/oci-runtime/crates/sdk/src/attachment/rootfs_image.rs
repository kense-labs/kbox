use serde::{Deserialize, Serialize};

use super::{invalid_attachment, CreateAttachments};
use crate::{OciBundle, Result};

/// Guest-native workload filesystem; independent of the VM boot system image.
pub const ROOTFS_IMAGE_EXTENSION: &str = "dev.kbox.rootfs-image";
/// First ext4 workload-disk transport contract.
pub const ROOTFS_IMAGE_VERSION: u16 = 1;
/// Writable transport with an explicitly read-only container root view.
pub const ROOTFS_IMAGE_READONLY_VERSION: u16 = 2;
/// Fixed operation-bundle input; ownership transfers with the bundle.
pub const ROOTFS_IMAGE_FILE_NAME: &str = "rootfs.ext4";

/// Immutable initial disk content bound by the OCI configuration digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RootfsImage {
    schema_version: u16,
    sha256: String,
    size: u64,
    image_digest: String,
}

impl RootfsImage {
    /// Bind verified ext4 bytes to their immutable OCI image source.
    pub fn new(
        sha256: impl Into<String>,
        size: u64,
        image_digest: impl Into<String>,
    ) -> Result<Self> {
        let image = Self {
            schema_version: ROOTFS_IMAGE_VERSION,
            sha256: sha256.into(),
            size,
            image_digest: image_digest.into(),
        };
        image.validate()?;
        Ok(image)
    }

    /// Require a read-only Guest root without changing disk ownership or access.
    pub fn with_readonly_root(mut self) -> Result<Self> {
        self.validate()?;
        self.schema_version = ROOTFS_IMAGE_READONLY_VERSION;
        Ok(self)
    }

    pub fn version(&self) -> u16 {
        self.schema_version
    }

    pub fn readonly_root(&self) -> bool {
        self.schema_version == ROOTFS_IMAGE_READONLY_VERSION
    }

    /// Initial ext4 SHA256, including its algorithm prefix.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    /// Exact raw disk capacity, in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }
    /// Immutable OCI manifest digest from which the filesystem was assembled.
    pub fn image_digest(&self) -> &str {
        &self.image_digest
    }
    /// Exact OCI annotation value.
    pub fn annotation_value(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|error| invalid_attachment(error.to_string()))
    }
    /// Decode the bounded annotation and require the exact versioned root view.
    pub fn from_bundle(bundle: &OciBundle) -> Result<Option<Self>> {
        let value = bundle
            .spec()
            .annotations()
            .as_ref()
            .and_then(|a| a.get(ROOTFS_IMAGE_EXTENSION));
        let Some(value) = value else {
            return Ok(None);
        };
        if value.len() > 1024 {
            return Err(invalid_attachment(
                "rootfs image descriptor exceeds its byte limit",
            ));
        }
        let image: Self = serde_json::from_str(value).map_err(|error| {
            invalid_attachment(format!("invalid rootfs image descriptor: {error}"))
        })?;
        image.validate()?;
        let root = bundle
            .spec()
            .root()
            .as_ref()
            .ok_or_else(|| invalid_attachment("rootfs image requires an OCI root"))?;
        if root.path() != std::path::Path::new("rootfs")
            || (root.readonly() == Some(true)) != image.readonly_root()
        {
            return Err(invalid_attachment(
                "rootfs image requires the exact bundle rootfs path and matching versioned root access",
            ));
        }
        Ok(Some(image))
    }
    /// Revalidate decoded protocol values.
    pub fn validate(&self) -> Result<()> {
        let digest = |value: &str| {
            value.strip_prefix("sha256:").is_some_and(|v| {
                v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            })
        };
        if !matches!(
            self.schema_version,
            ROOTFS_IMAGE_VERSION | ROOTFS_IMAGE_READONLY_VERSION
        ) || !digest(&self.sha256)
            || !digest(&self.image_digest)
            || !(16 * 1024 * 1024..=64 * 1024 * 1024 * 1024).contains(&self.size)
            || self.size & 4095 != 0
        {
            return Err(invalid_attachment(
                "unsupported or invalid ext4 rootfs image identity/capacity",
            ));
        }
        Ok(())
    }
}

impl CreateAttachments {
    /// Declare the required disk transport; old runtimes must refuse it.
    pub fn with_rootfs_image(self, bundle: &OciBundle) -> Result<Self> {
        let image = RootfsImage::from_bundle(bundle)?
            .ok_or_else(|| invalid_attachment("rootfs image extension has no descriptor"))?;
        if self.extensions.contains_key(ROOTFS_IMAGE_EXTENSION) {
            self.validate(bundle)?;
            Ok(self)
        } else {
            self.add_extension_from_annotation(
                bundle,
                ROOTFS_IMAGE_EXTENSION,
                image.version(),
                true,
            )
        }
    }
    /// Revalidate the explicit extension declaration independently of legacy directories.
    pub fn rootfs_image(&self, bundle: &OciBundle) -> Result<Option<RootfsImage>> {
        let image = RootfsImage::from_bundle(bundle)?;
        if image.is_some() != self.extensions.contains_key(ROOTFS_IMAGE_EXTENSION) {
            return Err(invalid_attachment(
                "rootfs image descriptor and required extension must be declared together",
            ));
        }
        if let Some(extension) = self.extensions.get(ROOTFS_IMAGE_EXTENSION) {
            if image
                .as_ref()
                .is_none_or(|image| extension.version() != image.version())
                || !extension.required()
                || image.is_none()
            {
                return Err(invalid_attachment(
                    "rootfs image requires the exact supported required extension",
                ));
            }
        }
        Ok(image)
    }
}

#[cfg(test)]
#[path = "rootfs_image_tests.rs"]
mod tests;
