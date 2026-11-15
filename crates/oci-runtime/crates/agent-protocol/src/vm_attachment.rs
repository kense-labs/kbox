use std::collections::BTreeSet;

use koci_sdk::{
    ConfigurationAttachment, ContainerTarget, Error, ErrorCode, NetworkAttachmentIdentity,
    NetworkCleanup, OciBundle, Result, StoragePathRestrictions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{GuestPath, AGENT_RUNTIME_SHARE_GUEST_ROOT};

mod file;
#[cfg(test)]
#[path = "vm_attachment/file_tests.rs"]
mod file_tests;
pub use file::{
    file_projection_root, file_tag, AgentVmFileAttachment, AGENT_VM_FILES_GUEST_ROOT,
    AGENT_VM_FILES_RELATIVE_ROOT, AGENT_VM_PROJECTED_FILE_NAME,
};
/// Cumulative exact-file transport schema; prior versions retain their encoding.
pub const AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v6";
/// File transport with required Host memory provenance; old shims/Agents must refuse.
pub const AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v8";
/// Memory file version2 also binds environment projection to protected exact sources.
pub const AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION: &str =
    "koci.agent-vm-attachments.v9";
/// RAM files/optional protected environment together with exact raw-storage disks.
pub const AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION: &str =
    "koci.agent-vm-attachments.v11";

fn memory_manifest_schema(bundle: &OciBundle) -> Result<Option<&'static str>> {
    Ok(
        koci_sdk::MemorySecretFiles::from_bundle(bundle)?.map(|memory| match memory.version() {
            koci_sdk::MEMORY_SECRET_RAW_STORAGE_VERSION => {
                AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
            }
            koci_sdk::MEMORY_SECRET_ENVIRONMENT_VERSION => {
                AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
            }
            _ => AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION,
        }),
    )
}

/// Explicit process-admission guard; v1-v9 transport encodings retain their meaning.
pub const AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v10";

mod directory;
#[cfg(test)]
#[path = "vm_attachment/directory_tests.rs"]
mod directory_tests;
mod raw_storage;
mod rootfs;
mod storage;
/// Explicit combined raw-disk/rootfs/file transport; v1–v6 meanings stay unchanged.
pub const AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v7";
pub use rootfs::{workload_rootfs_disk_path, AgentVmRootfsImage};
/// Workload-rootfs disk schema; retains v1-v4 encodings unchanged.
pub const AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v5";

pub use directory::{AgentVmDirectoryAttachment, AgentVmDirectorySourceIdentity};
pub use storage::{AgentVmStorageAttachment, AgentVmStorageSourceIdentity};

/// Immutable raw-storage Host-to-shim-to-Guest transport schema.
pub const AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v2";
/// Cumulative transport schema that binds independent Host directories.
pub const AGENT_VM_DIRECTORY_MANIFEST_SCHEMA_VERSION: &str = "koci.agent-vm-attachments.v3";
/// Directory transport requiring positive Host path-restriction enforcement.
pub const AGENT_VM_RESTRICTED_DIRECTORY_MANIFEST_SCHEMA_VERSION: &str =
    "koci.agent-vm-attachments.v4";
/// Private Guest bootstrap binding emitted by the confined Host worker.
pub const AGENT_HOST_STORAGE_PATH_RESTRICTIONS_SHA256_ENV: &str =
    "K_OCI_HOST_STORAGE_PATH_RESTRICTIONS_SHA256";
/// Protected Guest directory beneath the private runtime share.
pub const AGENT_VM_DIRECTORY_GUEST_ROOT: &str = "/run/koci-runtime/bundle/.koci-directories";
/// Directory transport sources stay inside the existing bundle-only boundary.
pub const AGENT_VM_DIRECTORY_RELATIVE_ROOT: &str = ".koci-directories";
/// Network-only transport schema emitted before raw storage transport existed.
pub const AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION_V1: &str = "koci.agent-vm-attachments.v1";
/// Fixed file name inside one exact utility-VM runtime share.
pub const AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME: &str = ".koci-agent-vm-attachments.json";
/// Guest environment key carrying the exact encoded-manifest SHA-256 digest.
pub const AGENT_VM_ATTACHMENT_MANIFEST_SHA256_ENV: &str =
    "K_OCI_AGENT_VM_ATTACHMENT_MANIFEST_SHA256";
/// Maximum encoded attachment transport manifest size.
pub const AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES: usize = 64 * 1024;

const MAX_NETWORK_ATTACHMENTS: usize = 256;
const MAX_STORAGE_ATTACHMENTS: usize = 128;
const LINUX_INTERFACE_NAME_BYTES: usize = 15;
const TRANSPORT_MAC_DOMAIN: &[u8] = b"koci.agent-vm-network-mac.v1\0";

/// Exact locally administered unicast MAC assigned to one Guest virtio NIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentVmMacAddress([u8; 6]);

impl AgentVmMacAddress {
    /// Validate an exact six-byte locally administered unicast address.
    pub fn new(bytes: [u8; 6]) -> Result<Self> {
        if bytes[0] & 0x01 != 0 || bytes[0] & 0x02 == 0 || bytes.iter().all(|byte| *byte == 0) {
            return Err(manifest_error(
                "utility-VM transport MAC must be a nonzero locally administered unicast address",
            ));
        }
        Ok(Self(bytes))
    }

    /// Deterministically derive a transport-local address from immutable evidence.
    pub fn derive(
        attachment_digest: &str,
        identity: &NetworkAttachmentIdentity,
        tap_name: &str,
    ) -> Result<Self> {
        validate_digest(attachment_digest, "attachment digest")?;
        validate_interface_name(tap_name)?;
        let mut hasher = Sha256::new();
        hasher.update(TRANSPORT_MAC_DOMAIN);
        for value in [
            attachment_digest,
            identity.namespace().as_str(),
            identity.interface().as_str(),
            identity.cleanup().as_str(),
            tap_name,
        ] {
            let length = u64::try_from(value.len()).map_err(|error| {
                manifest_error(format!(
                    "utility-VM network MAC evidence length is not portable: {error}"
                ))
            })?;
            hasher.update(length.to_le_bytes());
            hasher.update(value.as_bytes());
        }
        let digest = hasher.finalize();
        let mut bytes = [0_u8; 6];
        bytes.copy_from_slice(&digest[..6]);
        bytes[0] = (bytes[0] & 0xfc) | 0x02;
        Self::new(bytes)
    }

    /// Borrow the exact six bytes passed to the VMM and matched in the Guest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

/// One caller-authorized TAP carried into a dedicated Linux utility VM.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentVmNetworkAttachment {
    identity: NetworkAttachmentIdentity,
    tap_name: String,
    namespace: ConfigurationAttachment,
    interface: ConfigurationAttachment,
    cleanup: NetworkCleanup,
    mac_address: AgentVmMacAddress,
}

impl AgentVmNetworkAttachment {
    /// Bind one exact public attachment entry to its VMM TAP transport.
    pub fn new(
        identity: NetworkAttachmentIdentity,
        tap_name: impl Into<String>,
        namespace: ConfigurationAttachment,
        interface: ConfigurationAttachment,
        cleanup: NetworkCleanup,
        mac_address: AgentVmMacAddress,
    ) -> Result<Self> {
        let entry = Self {
            identity,
            tap_name: tap_name.into(),
            namespace,
            interface,
            cleanup,
            mac_address,
        };
        entry.validate_shape()?;
        Ok(entry)
    }

    /// Immutable caller-issued network identities.
    #[must_use]
    pub const fn identity(&self) -> &NetworkAttachmentIdentity {
        &self.identity
    }

    /// Host TAP name consumed by libkrun and recreated as the Guest source name.
    #[must_use]
    pub fn tap_name(&self) -> &str {
        &self.tap_name
    }

    /// Exact OCI network namespace descriptor.
    #[must_use]
    pub const fn namespace(&self) -> &ConfigurationAttachment {
        &self.namespace
    }

    /// Exact OCI `linux.netDevices` descriptor.
    #[must_use]
    pub const fn interface(&self) -> &ConfigurationAttachment {
        &self.interface
    }

    /// Caller-declared namespace cleanup boundary.
    #[must_use]
    pub const fn cleanup(&self) -> NetworkCleanup {
        self.cleanup
    }

    /// Exact MAC configured in libkrun and matched before Guest protocol startup.
    #[must_use]
    pub const fn mac_address(&self) -> AgentVmMacAddress {
        self.mac_address
    }

    fn validate_shape(&self) -> Result<()> {
        validate_interface_name(&self.tap_name)?;
        if self.cleanup != NetworkCleanup::ReleaseRuntimeNamespace {
            return Err(manifest_error(
                "KVM TAP transport requires release-runtime-namespace cleanup",
            ));
        }
        let expected_pointer = format!("/linux/netDevices/{}", escape_json_pointer(&self.tap_name));
        if self.interface.json_pointer() != expected_pointer {
            return Err(manifest_error(format!(
                "KVM TAP {} does not match interface attachment {}",
                self.tap_name,
                self.interface.json_pointer()
            )));
        }
        if !self
            .namespace
            .json_pointer()
            .starts_with("/linux/namespaces/")
        {
            return Err(manifest_error(format!(
                "KVM TAP {} does not reference an OCI Linux namespace",
                self.tap_name
            )));
        }
        validate_digest(self.namespace.value_digest(), "namespace value digest")?;
        validate_digest(self.interface.value_digest(), "interface value digest")?;
        AgentVmMacAddress::new(*self.mac_address.as_bytes())?;
        Ok(())
    }

    fn validate_configuration(&self, configuration: &Value) -> Result<()> {
        validate_configuration_attachment(configuration, &self.namespace, "network namespace")?;
        validate_configuration_attachment(configuration, &self.interface, "network interface")?;
        let namespace = configuration
            .pointer(self.namespace.json_pointer())
            .and_then(Value::as_object)
            .ok_or_else(|| manifest_error("network namespace attachment is not an object"))?;
        if namespace.get("type").and_then(Value::as_str) != Some("network") {
            return Err(manifest_error(format!(
                "KVM TAP {} does not select an OCI network namespace",
                self.tap_name
            )));
        }
        if namespace.contains_key("path") {
            return Err(manifest_error(format!(
                "KVM TAP {} cannot enter a caller-owned joined network namespace",
                self.tap_name
            )));
        }
        Ok(())
    }
}

/// Complete immutable storage/network transport authority for one dedicated utility VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentVmAttachmentManifest {
    schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nproc_transport_schema: Option<String>,
    target: ContainerTarget,
    guest_bundle: GuestPath,
    config_digest: String,
    attachment_digest: String,
    network: Vec<AgentVmNetworkAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    storage: Vec<AgentVmStorageAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    directories: Vec<AgentVmDirectoryAttachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    storage_path_restrictions_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rootfs_image: Option<AgentVmRootfsImage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    files: Vec<AgentVmFileAttachment>,
}

impl AgentVmAttachmentManifest {
    fn transport_schema(&self) -> &str {
        self.nproc_transport_schema
            .as_deref()
            .unwrap_or(&self.schema_version)
    }

    /// Bind kernel process admission without reinterpreting any prior transport.
    pub fn with_nproc_admission(mut self, bundle: &OciBundle) -> Result<Self> {
        if koci_sdk::nproc_admission_required(bundle)
            && self.schema_version != AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION
        {
            self.nproc_transport_schema = Some(self.schema_version);
            self.schema_version = AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION.into();
        }
        self.validate_bundle(bundle)?;
        Ok(self)
    }

    /// Construct and validate one canonical, non-empty attachment transport manifest.
    pub fn new(
        target: ContainerTarget,
        guest_bundle: GuestPath,
        config_digest: impl Into<String>,
        attachment_digest: impl Into<String>,
        network: Vec<AgentVmNetworkAttachment>,
        storage: Vec<AgentVmStorageAttachment>,
    ) -> Result<Self> {
        Self::new_with_directories(
            target,
            guest_bundle,
            config_digest,
            attachment_digest,
            network,
            storage,
            Vec::new(),
        )
    }

    /// Bind directory transports without changing the existing v1/v2 wire encodings.
    pub fn new_with_directories(
        target: ContainerTarget,
        guest_bundle: GuestPath,
        config_digest: impl Into<String>,
        attachment_digest: impl Into<String>,
        network: Vec<AgentVmNetworkAttachment>,
        storage: Vec<AgentVmStorageAttachment>,
        directories: Vec<AgentVmDirectoryAttachment>,
    ) -> Result<Self> {
        let schema_version = if !directories.is_empty() {
            AGENT_VM_DIRECTORY_MANIFEST_SCHEMA_VERSION
        } else if storage.is_empty() {
            AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION_V1
        } else {
            AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION
        };
        let manifest = Self {
            schema_version: schema_version.to_string(),
            nproc_transport_schema: None,
            target,
            guest_bundle,
            config_digest: config_digest.into(),
            attachment_digest: attachment_digest.into(),
            network,
            storage,
            directories,
            storage_path_restrictions_digest: None,
            rootfs_image: None,
            files: Vec::new(),
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Publish exact files; memory constraints require v8 while ordinary files retain v6.
    pub fn new_with_file_grants(
        target: ContainerTarget,
        guest_bundle: GuestPath,
        bundle: &OciBundle,
        attachment_digest: impl Into<String>,
        directories: Vec<AgentVmDirectoryAttachment>,
        files: Vec<AgentVmFileAttachment>,
        rootfs_image: Option<AgentVmRootfsImage>,
    ) -> Result<Self> {
        let manifest = Self {
            schema_version: memory_manifest_schema(bundle)?
                .unwrap_or(AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION)
                .into(),
            nproc_transport_schema: None,
            target,
            guest_bundle,
            config_digest: bundle.config_digest().into(),
            attachment_digest: attachment_digest.into(),
            network: Vec::new(),
            storage: Vec::new(),
            directories,
            files,
            rootfs_image,
            storage_path_restrictions_digest: StoragePathRestrictions::from_bundle(bundle)?
                .as_ref()
                .map(StoragePathRestrictions::digest)
                .transpose()?,
        };
        manifest.with_nproc_admission(bundle)
    }
    /// Exact ordinary-file grants, distinct from directory grants and raw block storage.
    pub fn files(&self) -> &[AgentVmFileAttachment] {
        &self.files
    }

    /// Bind the independent workload disk without reinterpreting older manifests.
    pub fn with_rootfs_image(
        mut self,
        rootfs: AgentVmRootfsImage,
        bundle: &OciBundle,
    ) -> Result<Self> {
        self.schema_version = memory_manifest_schema(bundle)?
            .unwrap_or(if self.files.is_empty() {
                AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION
            } else {
                AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION
            })
            .into();
        self.rootfs_image = Some(rootfs);
        self.storage_path_restrictions_digest = StoragePathRestrictions::from_bundle(bundle)?
            .as_ref()
            .map(StoragePathRestrictions::digest)
            .transpose()?;
        self.with_nproc_admission(bundle)
    }

    /// Exact generation-owned workload disk, if this is the new transport.
    pub fn rootfs_image(&self) -> Option<&AgentVmRootfsImage> {
        self.rootfs_image.as_ref()
    }

    /// Require Host restrictions without expanding the bounded transport manifest.
    pub fn with_storage_path_restrictions(
        mut self,
        bundle: &OciBundle,
        restrictions: &StoragePathRestrictions,
    ) -> Result<Self> {
        if self.rootfs_image.is_none() && self.files.is_empty() {
            self.schema_version = AGENT_VM_RESTRICTED_DIRECTORY_MANIFEST_SCHEMA_VERSION.into();
        }
        self.storage_path_restrictions_digest = Some(restrictions.digest()?);
        self.with_nproc_admission(bundle)
    }

    /// Positive Host enforcement binding required before the Guest serves requests.
    pub fn storage_path_restrictions_digest(&self) -> Option<&str> {
        self.storage_path_restrictions_digest.as_deref()
    }

    /// Exact container generation owning this dedicated VM.
    #[must_use]
    pub const fn target(&self) -> &ContainerTarget {
        &self.target
    }

    /// Guest-visible bundle directory bound to the later create request.
    #[must_use]
    pub const fn guest_bundle(&self) -> &GuestPath {
        &self.guest_bundle
    }

    /// SHA-256 digest of the exact OCI configuration bytes.
    #[must_use]
    pub fn config_digest(&self) -> &str {
        &self.config_digest
    }

    /// SHA-256 digest of the complete public create attachment contract.
    #[must_use]
    pub fn attachment_digest(&self) -> &str {
        &self.attachment_digest
    }

    /// Canonically ordered VMM network transports.
    #[must_use]
    pub fn network(&self) -> &[AgentVmNetworkAttachment] {
        &self.network
    }

    /// Canonically ordered VMM raw-storage transports.
    #[must_use]
    pub fn storage(&self) -> &[AgentVmStorageAttachment] {
        &self.storage
    }

    /// Canonically ordered directory transports and their exact public grants.
    #[must_use]
    pub fn directories(&self) -> &[AgentVmDirectoryAttachment] {
        &self.directories
    }

    /// Encode the exact bounded JSON representation used for persistence and hashing.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let encoded = serde_json::to_vec(self).map_err(|error| {
            manifest_error(format!(
                "failed to encode utility-VM attachment manifest: {error}"
            ))
        })?;
        if encoded.len() > AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES {
            return Err(manifest_error(
                "encoded utility-VM attachment manifest exceeds its bounded size",
            ));
        }
        Ok(encoded)
    }

    /// Decode and validate one bounded JSON representation.
    pub fn from_bytes(encoded: &[u8]) -> Result<Self> {
        if encoded.is_empty() || encoded.len() > AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES {
            return Err(manifest_error(
                "utility-VM attachment manifest must have a bounded nonzero size",
            ));
        }
        let manifest: Self = serde_json::from_slice(encoded).map_err(|error| {
            manifest_error(format!(
                "failed to decode utility-VM attachment manifest: {error}"
            ))
        })?;
        manifest.validate()?;
        if manifest.to_bytes()?.as_slice() != encoded {
            return Err(manifest_error(
                "utility-VM attachment manifest is not canonically encoded",
            ));
        }
        Ok(manifest)
    }

    /// SHA-256 digest of the exact encoded manifest.
    pub fn digest(&self) -> Result<String> {
        Ok(digest_bytes(&self.to_bytes()?))
    }

    /// Revalidate all public attachment pointers against the exact OCI bundle.
    pub fn validate_bundle(&self, bundle: &OciBundle) -> Result<()> {
        self.validate()?;
        if koci_sdk::nproc_admission_required(bundle)
            != (self.schema_version == AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION)
        {
            return Err(manifest_error(
                "RLIMIT_NPROC requires exact v10 admission enforcement; downgrade is forbidden",
            ));
        }
        if koci_sdk::raw_storage_images_requested(bundle)?
            != matches!(
                self.transport_schema(),
                AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
                    | AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
            )
        {
            return Err(manifest_error(
                "raw storage images require their explicit v7/v11 transport and SDK capability",
            ));
        }
        if bundle.config_digest() != self.config_digest {
            return Err(manifest_error(format!(
                "utility-VM attachment manifest expects configuration {}, received {}",
                self.config_digest,
                bundle.config_digest()
            )));
        }
        let memory_schema = memory_manifest_schema(bundle)?;
        if memory_schema.is_some_and(|version| self.transport_schema() != version)
            || (memory_schema.is_none()
                && matches!(
                    self.transport_schema(),
                    AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
                ))
        {
            return Err(manifest_error(
                "memory Secret files/environment require their exact v8/v9/v11 transport; downgrade is forbidden",
            ));
        }
        let configuration: Value = serde_json::from_str(bundle.config_json()).map_err(|error| {
            manifest_error(format!(
                "validated OCI configuration could not be decoded for VM attachments: {error}"
            ))
        })?;
        for attachment in &self.network {
            attachment.validate_configuration(&configuration)?;
        }
        for attachment in &self.storage {
            attachment.validate_configuration(&configuration)?;
        }
        for attachment in &self.directories {
            attachment.validate_configuration(&configuration)?;
        }
        let file_grants = koci_sdk::FileGrants::from_bundle(bundle)?;
        if file_grants.as_ref().map_or(0, |g| g.entries().len()) != self.files.len() {
            return Err(manifest_error(
                "file manifest inventory differs from its required SDK file grants",
            ));
        }
        if let Some(grants) = file_grants {
            for file in &self.files {
                file.validate_configuration(&configuration)?;
                let exact = grants
                    .entries()
                    .iter()
                    .find(|g| g.mount_index() == file.mount_index().unwrap_or(usize::MAX))
                    .ok_or_else(|| manifest_error("file manifest has an undeclared mount"))?;
                if exact.identity() != file.attachment().identity()
                    || exact.source_identity() != file.source_identity()
                    || exact.access_mode() != file.attachment().access_mode()
                {
                    return Err(manifest_error(
                        "file manifest changed its caller allocation, source inode or access",
                    ));
                }
            }
        }
        if koci_sdk::RootfsImage::from_bundle(bundle)?.as_ref()
            != self.rootfs_image.as_ref().map(AgentVmRootfsImage::image)
        {
            return Err(manifest_error(
                "workload rootfs descriptor differs from the exact bound configuration",
            ));
        }
        let restrictions = StoragePathRestrictions::from_bundle(bundle)?;
        let digest = restrictions
            .as_ref()
            .map(StoragePathRestrictions::digest)
            .transpose()?;
        if digest != self.storage_path_restrictions_digest {
            return Err(manifest_error(
                "directory restrictions differ from the bound Host enforcement contract",
            ));
        }
        if let Some(restrictions) = restrictions {
            for entry in restrictions.entries() {
                let directory = self
                    .directories
                    .iter()
                    .find(|directory| directory.attachment().identity() == entry.identity())
                    .ok_or_else(|| {
                        manifest_error("directory restriction references an ungranted identity")
                    })?;
                if directory.attachment().access_mode() == koci_sdk::StorageAccessMode::ReadOnly
                    && !entry.write_exceptions().is_empty()
                {
                    return Err(manifest_error(
                        "directory restriction cannot widen a read-only grant",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate schema, target, digests, canonical order, and transport uniqueness.
    pub fn validate(&self) -> Result<()> {
        if (self.schema_version == AGENT_VM_NPROC_MANIFEST_SCHEMA_VERSION)
            != self.nproc_transport_schema.is_some()
        {
            return Err(manifest_error(
                "nproc admission requires its explicit v10 transport binding",
            ));
        }
        let schema = self.transport_schema();
        if schema != AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION_V1
            && schema != AGENT_VM_DIRECTORY_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_RESTRICTED_DIRECTORY_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
            && schema != AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
        {
            return Err(manifest_error(format!(
                "unsupported utility-VM attachment manifest schema {}",
                self.schema_version
            )));
        }
        if self
            .target
            .generation
            .is_none_or(|generation| generation.0 == 0)
        {
            return Err(manifest_error(format!(
                "utility-VM attachment manifest for {} requires a positive exact generation",
                self.target.id
            )));
        }
        let guest_bundle = self.guest_bundle.as_str();
        let expected_prefix = format!("{AGENT_RUNTIME_SHARE_GUEST_ROOT}/");
        if !guest_bundle.starts_with(&expected_prefix) {
            return Err(manifest_error(format!(
                "utility-VM attachment bundle must remain inside {AGENT_RUNTIME_SHARE_GUEST_ROOT}: {guest_bundle}"
            )));
        }
        validate_digest(&self.config_digest, "configuration digest")?;
        validate_digest(&self.attachment_digest, "attachment digest")?;
        if self.network.len() > MAX_NETWORK_ATTACHMENTS {
            return Err(manifest_error(format!(
                "utility-VM attachment manifest carries more than {MAX_NETWORK_ATTACHMENTS} network entries"
            )));
        }
        if self.storage.len()
            + self.directories.len()
            + self.files.len()
            + usize::from(self.rootfs_image.is_some())
            > MAX_STORAGE_ATTACHMENTS
        {
            return Err(manifest_error(format!(
                "utility-VM attachment manifest carries more than {MAX_STORAGE_ATTACHMENTS} storage entries"
            )));
        }
        let raw_images = matches!(
            self.transport_schema(),
            AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
                | AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
        );
        if schema == AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION && self.files.is_empty() {
            return Err(manifest_error(
                "combined memory/raw transport requires exact file grants",
            ));
        }
        if raw_images
            && (self.storage.is_empty()
                || !self.network.is_empty()
                || self.guest_bundle.as_str() != format!("{AGENT_RUNTIME_SHARE_GUEST_ROOT}/bundle"))
        {
            return Err(manifest_error(
                "v7/v11 requires raw storage in the exact dedicated bundle without network",
            ));
        }
        if !raw_images
            && matches!(
                self.transport_schema(),
                AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION
                    | AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION
                    | AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
            ) != !self.files.is_empty()
        {
            return Err(manifest_error(
                "file transports require schema v6/v8 and either version requires files",
            ));
        }
        if (self.transport_schema() == AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION
            && self.rootfs_image.is_none())
            || (self.rootfs_image.is_some()
                && !matches!(
                    self.transport_schema(),
                    AGENT_VM_ROOTFS_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_FILE_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_MEMORY_FILE_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_MEMORY_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
                        | AGENT_VM_MEMORY_RAW_STORAGE_MANIFEST_SCHEMA_VERSION
                ))
        {
            return Err(manifest_error(
                "workload rootfs requires schema v5/v6 and v5 requires a rootfs disk",
            ));
        }
        if !self.files.is_empty() {
            if self.guest_bundle.as_str() != format!("{AGENT_RUNTIME_SHARE_GUEST_ROOT}/bundle")
                || (!raw_images && !self.storage.is_empty())
                || !self.network.is_empty()
                || self.files.windows(2).any(|w| w[0] >= w[1])
            {
                return Err(manifest_error("file transport requires ordered unique dedicated-VM grants without raw storage or network"));
            }
            let mut inodes = BTreeSet::new();
            let mut allocations = BTreeSet::new();
            let mut mounts = BTreeSet::new();
            for file in &self.files {
                file.validate_shape(&self.config_digest)?;
                if !inodes.insert(file.source_identity())
                    || !allocations.insert(file.attachment().identity())
                    || !mounts.insert(file.attachment().mount())
                    || self.directories.iter().any(|d| {
                        d.attachment().identity() == file.attachment().identity()
                            || d.attachment().mount() == file.attachment().mount()
                    })
                {
                    return Err(manifest_error(
                        "file grants duplicate source objects, mounts or allocation identities",
                    ));
                }
            }
        }
        if let Some(rootfs) = &self.rootfs_image {
            rootfs.validate()?;
            if (!raw_images && !self.storage.is_empty())
                || !self.network.is_empty()
                || self.guest_bundle.as_str() != format!("{AGENT_RUNTIME_SHARE_GUEST_ROOT}/bundle")
            {
                return Err(manifest_error("workload rootfs v5 requires the dedicated bundle without raw caller storage or network"));
            }
        }
        if self.network.is_empty()
            && self.storage.is_empty()
            && self.directories.is_empty()
            && self.rootfs_image.is_none()
            && self.files.is_empty()
        {
            return Err(manifest_error(
                "utility-VM attachment manifest must carry at least one storage or network entry",
            ));
        }
        if self.transport_schema() == AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION_V1
            && (!self.storage.is_empty() || self.network.is_empty())
        {
            return Err(manifest_error(
                "utility-VM attachment manifest v1 requires network entries and cannot carry storage",
            ));
        }
        if self.transport_schema() == AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION
            && self.storage.is_empty()
        {
            return Err(manifest_error(
                "utility-VM attachment manifest v2 requires at least one storage entry",
            ));
        }
        if !raw_images
            && self.rootfs_image.is_none()
            && self.files.is_empty()
            && (matches!(
                self.transport_schema(),
                AGENT_VM_DIRECTORY_MANIFEST_SCHEMA_VERSION
                    | AGENT_VM_RESTRICTED_DIRECTORY_MANIFEST_SCHEMA_VERSION
            )) != !self.directories.is_empty()
        {
            return Err(manifest_error(
                "directory transports require schema v3, and v3 requires directory transports",
            ));
        }
        if !raw_images
            && self.rootfs_image.is_none()
            && self.files.is_empty()
            && ((self.transport_schema() == AGENT_VM_RESTRICTED_DIRECTORY_MANIFEST_SCHEMA_VERSION)
                != self.storage_path_restrictions_digest.is_some())
        {
            return Err(manifest_error(
                "Host path restrictions require schema v4 and an enforcement digest",
            ));
        }
        if let Some(digest) = &self.storage_path_restrictions_digest {
            validate_digest(digest, "Host storage path restrictions")?;
        }
        if self.network.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(manifest_error(
                "utility-VM network attachments must be unique and canonically ordered",
            ));
        }
        if self.storage.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(manifest_error(
                "utility-VM storage attachments must be unique and canonically ordered",
            ));
        }
        if self.directories.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(manifest_error(
                "directory transports must be unique and canonically ordered",
            ));
        }

        let mut tap_names = BTreeSet::new();
        let mut mac_addresses = BTreeSet::new();
        for attachment in &self.network {
            attachment.validate_shape()?;
            let expected_mac = AgentVmMacAddress::derive(
                &self.attachment_digest,
                attachment.identity(),
                attachment.tap_name(),
            )?;
            if attachment.mac_address() != expected_mac {
                return Err(manifest_error(format!(
                    "KVM TAP {} MAC is not derived from its immutable attachment evidence",
                    attachment.tap_name()
                )));
            }
            if !tap_names.insert(attachment.tap_name()) {
                return Err(manifest_error(format!(
                    "KVM TAP {} is declared more than once",
                    attachment.tap_name()
                )));
            }
            if !mac_addresses.insert(attachment.mac_address()) {
                return Err(manifest_error(format!(
                    "KVM TAP {} collides with another derived transport MAC",
                    attachment.tap_name()
                )));
            }
        }

        let mut storage_mounts = BTreeSet::new();
        let mut storage_sources = BTreeSet::new();
        let mut storage_serials = BTreeSet::new();
        let mut block_ids = BTreeSet::new();
        let mut storage_identities = BTreeSet::new();
        let mut source_objects = BTreeSet::new();
        if let Some(rootfs) = &self.rootfs_image {
            storage_serials.insert(rootfs.source_identity().virtio_serial());
            source_objects.insert((
                rootfs.source_identity().device(),
                rootfs.source_identity().inode(),
            ));
        }
        for attachment in &self.storage {
            attachment.validate_shape(&self.attachment_digest)?;
            if !storage_mounts.insert(attachment.mount().json_pointer()) {
                return Err(manifest_error(format!(
                    "storage mount {} is transported more than once",
                    attachment.mount().json_pointer()
                )));
            }
            if !storage_identities.insert(attachment.identity())
                || !source_objects.insert((
                    attachment.source_identity().device(),
                    attachment.source_identity().inode(),
                ))
            {
                return Err(manifest_error(
                    "raw storage repeats an allocation or source object",
                ));
            }
            if !storage_sources.insert(attachment.host_source()) {
                return Err(manifest_error(format!(
                    "Host storage source {} is transported more than once",
                    attachment.host_source()
                )));
            }
            let serial = attachment.virtio_serial();
            if !storage_serials.insert(serial.clone()) {
                return Err(manifest_error(format!(
                    "storage attachment {} collides on virtio-blk serial {serial}",
                    attachment.identity()
                )));
            }
            if !block_ids.insert(attachment.block_id()) {
                return Err(manifest_error(format!(
                    "storage attachment {} collides on libkrun block ID {}",
                    attachment.identity(),
                    attachment.block_id()
                )));
            }
        }
        let mut tags = BTreeSet::new();
        let mut directory_identities = BTreeSet::new();
        for directory in &self.directories {
            directory.validate_shape(&self.attachment_digest)?;
            let attachment = directory.attachment();
            if !storage_identities.insert(attachment.identity())
                || !storage_mounts.insert(attachment.mount().json_pointer())
                || !storage_sources.insert(directory.host_source())
                || !directory_identities.insert(directory.source_identity())
                || !tags.insert(directory.tag())
            {
                return Err(manifest_error(
                    "directory transport repeats a storage identity, mount, source, inode, or tag",
                ));
            }
        }
        for file in &self.files {
            if !storage_identities.insert(file.attachment().identity())
                || !storage_mounts.insert(file.attachment().mount().json_pointer())
                || !storage_sources.insert(file.host_source())
                || !source_objects.insert((
                    file.source_identity().device(),
                    file.source_identity().inode(),
                ))
            {
                return Err(manifest_error("file transport repeats a raw storage/rootfs allocation, mount or source object"));
            }
        }
        Ok(())
    }
}

fn validate_configuration_attachment(
    configuration: &Value,
    attachment: &ConfigurationAttachment,
    label: &str,
) -> Result<()> {
    let value = configuration
        .pointer(attachment.json_pointer())
        .ok_or_else(|| {
            manifest_error(format!(
                "{label} pointer does not exist in config.json: {}",
                attachment.json_pointer()
            ))
        })?;
    let digest = serde_json::to_vec(value)
        .map(|encoded| digest_bytes(&encoded))
        .map_err(|error| manifest_error(format!("failed to encode {label} evidence: {error}")))?;
    if digest != attachment.value_digest() {
        return Err(manifest_error(format!(
            "{label} evidence drifted at {}",
            attachment.json_pointer()
        )));
    }
    Ok(())
}

fn validate_interface_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > LINUX_INTERFACE_NAME_BYTES
        || matches!(name, "." | "..")
        || name
            .bytes()
            .any(|byte| matches!(byte, b'/' | b':' | 0) || byte.is_ascii_whitespace())
    {
        return Err(manifest_error(format!(
            "KVM TAP name must be a valid 1..={LINUX_INTERFACE_NAME_BYTES} byte Linux interface name: {name:?}"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(manifest_error(format!(
            "utility-VM {label} must use a sha256 digest"
        )));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(manifest_error(format!(
            "utility-VM {label} must contain exactly 64 lowercase hexadecimal characters"
        )));
    }
    Ok(())
}

fn escape_json_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn manifest_error(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::InvalidArgument, message).for_operation("validate-agent-vm-attachments")
}

#[cfg(test)]
mod tests {
    use koci_sdk::{
        oci_spec::runtime::Spec, ContainerTarget, CreateAttachments, Generation,
        NetworkAttachmentIdentity, NetworkCleanup, NetworkCleanupId, NetworkInterfaceId,
        NetworkNamespaceId, OciBundle, ProcessIo, StorageAccessMode, StorageAttachmentId,
        StorageCleanup, StorageOwnership,
    };
    use serde_json::{json, Value};

    use super::{
        AgentVmAttachmentManifest, AgentVmMacAddress, AgentVmNetworkAttachment,
        AgentVmStorageAttachment, AgentVmStorageSourceIdentity,
        AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES, AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION,
    };
    use crate::GuestPath;

    fn fixture() -> (OciBundle, CreateAttachments, AgentVmAttachmentManifest) {
        let mut value = serde_json::to_value(Spec::default()).expect("default OCI spec");
        value["linux"] = json!({
            "namespaces": [{"type": "network"}, {"type": "uts"}],
            "netDevices": {"tap0": {"name": "eth0"}}
        });
        let bundle = OciBundle::from_json(
            std::env::temp_dir().join("kagent-vm-manifest-fixture"),
            serde_json::to_string(&value).expect("fixture JSON"),
        )
        .expect("valid OCI bundle");
        let identity = NetworkAttachmentIdentity::new(
            NetworkNamespaceId::new("namespace-1").expect("namespace identity"),
            NetworkInterfaceId::new("interface-1").expect("interface identity"),
            NetworkCleanupId::new("cleanup-1").expect("cleanup identity"),
        );
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
            .expect("base attachments")
            .attach_linux_network_interface(
                &bundle,
                0,
                "tap0",
                identity,
                NetworkCleanup::ReleaseRuntimeNamespace,
            )
            .expect("authorized network attachment");
        let digest = attachments.digest().expect("attachment digest");
        let public = attachments.network_attachments()[0].clone();
        let mac =
            AgentVmMacAddress::derive(&digest, public.identity(), "tap0").expect("transport MAC");
        let network = AgentVmNetworkAttachment::new(
            public.identity().clone(),
            "tap0",
            public.namespace().clone(),
            public.interface().clone(),
            public.cleanup(),
            mac,
        )
        .expect("network transport");
        let manifest = AgentVmAttachmentManifest::new(
            ContainerTarget::exact(
                koci_sdk::ContainerId::new("networked").expect("container ID"),
                Generation(1),
            ),
            GuestPath::new("/run/koci-runtime/bundle").expect("guest bundle"),
            bundle.config_digest(),
            digest,
            vec![network],
            Vec::new(),
        )
        .expect("VM attachment manifest");
        (bundle, attachments, manifest)
    }

    fn storage_fixture() -> (OciBundle, CreateAttachments, AgentVmAttachmentManifest) {
        let mut value = serde_json::to_value(Spec::default()).expect("default OCI spec");
        value["mounts"] = json!([{
            "destination": "/data",
            "type": "ext4",
            "source": "/var/lib/kbox/authorized-volume.raw",
            "options": ["ro", "nodev"]
        }]);
        let bundle = OciBundle::from_json(
            std::env::temp_dir().join("kagent-vm-storage-manifest"),
            serde_json::to_string(&value).expect("fixture JSON"),
        )
        .expect("valid storage bundle");
        let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
            .expect("base attachments")
            .attach_storage_mount(
                &bundle,
                0,
                StorageAttachmentId::new("authorized-volume-7").expect("storage identity"),
                StorageAccessMode::ReadOnly,
                StorageOwnership::Caller,
                StorageCleanup::DetachOnly,
            )
            .expect("storage attachment");
        let digest = attachments.digest().expect("attachment digest");
        let public = &attachments.storage()[0];
        let storage = AgentVmStorageAttachment::new(
            public.identity().clone(),
            public.mount().clone(),
            public.access_mode(),
            public.ownership(),
            public.cleanup(),
            "/var/lib/kbox/authorized-volume.raw",
            AgentVmStorageSourceIdentity::new(17, 0, 4_203, 8 * 1024 * 1024)
                .expect("source identity"),
            &digest,
        )
        .expect("storage transport");
        let manifest = AgentVmAttachmentManifest::new(
            ContainerTarget::exact(
                koci_sdk::ContainerId::new("stored").expect("container ID"),
                Generation(3),
            ),
            GuestPath::new("/run/koci-runtime/bundle").expect("guest bundle"),
            bundle.config_digest(),
            digest,
            Vec::new(),
            vec![storage],
        )
        .expect("storage VM attachment manifest");
        (bundle, attachments, manifest)
    }

    #[test]
    fn round_trips_and_binds_the_exact_oci_configuration() {
        let (bundle, _, manifest) = fixture();
        let encoded = manifest.to_bytes().expect("encoded manifest");
        assert!(encoded.len() < AGENT_VM_ATTACHMENT_MANIFEST_MAX_BYTES);
        let decoded = AgentVmAttachmentManifest::from_bytes(&encoded).expect("decoded manifest");
        assert_eq!(decoded, manifest);
        assert_eq!(
            decoded.digest().expect("manifest digest"),
            manifest.digest().unwrap()
        );
        decoded
            .validate_bundle(&bundle)
            .expect("exact bundle binding");
        assert_eq!(decoded.network()[0].tap_name(), "tap0");
        assert_eq!(
            decoded.network()[0].mac_address().as_bytes()[0] & 0x03,
            0x02
        );
    }

    #[test]
    fn mac_derivation_has_an_architecture_independent_vector() {
        let identity = NetworkAttachmentIdentity::new(
            NetworkNamespaceId::new("namespace-1").expect("namespace identity"),
            NetworkInterfaceId::new("interface-1").expect("interface identity"),
            NetworkCleanupId::new("cleanup-1").expect("cleanup identity"),
        );
        let mac =
            AgentVmMacAddress::derive(&format!("sha256:{}", "1".repeat(64)), &identity, "tap0")
                .expect("transport MAC");
        assert_eq!(mac.as_bytes(), &[0xe6, 0x17, 0x88, 0x7f, 0xc5, 0xf8]);
    }

    #[test]
    fn raw_storage_round_trips_with_exact_serial_access_and_schema() {
        let (bundle, _, manifest) = storage_fixture();
        manifest.validate_bundle(&bundle).expect("storage bundle");
        let encoded = manifest.to_bytes().expect("storage manifest bytes");
        let decoded = AgentVmAttachmentManifest::from_bytes(&encoded).expect("storage manifest");

        assert_eq!(decoded, manifest);
        assert_eq!(
            decoded.schema_version,
            AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION
        );
        assert!(decoded.network().is_empty());
        assert_eq!(decoded.storage().len(), 1);
        assert_eq!(decoded.storage()[0].mount_index().unwrap(), 0);
        assert_eq!(
            decoded.storage()[0].access_mode(),
            StorageAccessMode::ReadOnly
        );
        assert_eq!(decoded.storage()[0].virtio_serial(), "1704203");
        assert!(decoded.storage()[0].block_id().starts_with("koci-storage-"));
    }

    #[test]
    fn raw_storage_rejects_source_access_identity_and_block_rebinding() {
        let (_, _, manifest) = storage_fixture();
        let bytes = manifest.to_bytes().expect("storage manifest bytes");

        for (field, replacement) in [
            ("hostSource", json!("relative.raw")),
            ("accessMode", json!("read-write")),
            ("blockId", json!("koci-storage-unbound")),
        ] {
            let mut encoded: Value = serde_json::from_slice(&bytes).expect("manifest JSON");
            encoded["storage"][0][field] = replacement;
            assert!(
                AgentVmAttachmentManifest::from_bytes(
                    &serde_json::to_vec(&encoded).expect("mutated manifest")
                )
                .is_err(),
                "{field}"
            );
        }

        let mut encoded: Value = serde_json::from_slice(&bytes).expect("manifest JSON");
        encoded["storage"][0]["sourceIdentity"]["size"] = json!(513);
        assert!(AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("misaligned storage manifest")
        )
        .is_err());
    }

    #[test]
    fn rejects_joined_namespaces_and_configuration_drift() {
        let (bundle, _, manifest) = fixture();
        let mut joined: Value = serde_json::from_str(bundle.config_json()).expect("fixture JSON");
        joined["linux"]["namespaces"][0]["path"] = json!("/proc/1/ns/net");
        let joined = OciBundle::from_json(
            std::env::temp_dir().join("kagent-vm-joined-fixture"),
            serde_json::to_string(&joined).expect("joined JSON"),
        )
        .expect("joined OCI bundle");
        let error = manifest
            .validate_bundle(&joined)
            .expect_err("a different config digest must fail closed");
        assert!(error.message.contains("expects configuration"));

        let mut encoded: Value =
            serde_json::from_slice(&manifest.to_bytes().expect("manifest bytes"))
                .expect("manifest JSON");
        encoded["network"][0]["cleanup"] = json!("preserve-caller-namespace");
        let error = AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("mutated manifest"),
        )
        .expect_err("joined cleanup must be rejected");
        assert!(error.message.contains("release-runtime-namespace"));
    }

    #[test]
    fn rejects_reordered_duplicates_and_digest_drift() {
        let (_, _, manifest) = fixture();
        let mut noncanonical = manifest.to_bytes().expect("manifest bytes");
        noncanonical.push(b'\n');
        assert!(AgentVmAttachmentManifest::from_bytes(&noncanonical).is_err());

        let mut encoded: Value =
            serde_json::from_slice(&manifest.to_bytes().expect("manifest bytes"))
                .expect("manifest JSON");
        encoded["schemaVersion"] = json!(AGENT_VM_ATTACHMENT_MANIFEST_SCHEMA_VERSION);
        assert!(AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("v2 network-only manifest")
        )
        .is_err());

        let mut encoded: Value =
            serde_json::from_slice(&manifest.to_bytes().expect("manifest bytes"))
                .expect("manifest JSON");
        let duplicate = encoded["network"][0].clone();
        encoded["network"]
            .as_array_mut()
            .expect("network array")
            .push(duplicate);
        assert!(AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("duplicate manifest")
        )
        .is_err());

        let mut encoded: Value =
            serde_json::from_slice(&manifest.to_bytes().expect("manifest bytes"))
                .expect("manifest JSON");
        encoded["attachmentDigest"] = json!(format!("sha256:{}", "A".repeat(64)));
        assert!(AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("drifted manifest")
        )
        .is_err());

        let mut encoded: Value =
            serde_json::from_slice(&manifest.to_bytes().expect("manifest bytes"))
                .expect("manifest JSON");
        encoded["attachmentDigest"] = json!(format!("sha256:{}", "0".repeat(64)));
        let error = AgentVmAttachmentManifest::from_bytes(
            &serde_json::to_vec(&encoded).expect("digest-rebound manifest"),
        )
        .expect_err("transport MAC must remain bound to the attachment digest");
        assert!(error.message.contains("not derived"));
    }
}
