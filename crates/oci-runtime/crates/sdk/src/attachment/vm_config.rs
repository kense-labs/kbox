//! Validated host VM resources, bound separately from workload cgroup limits.

use serde::{Deserialize, Serialize};

use crate::{CreateAttachments, Error, ErrorCode, OciBundle, Result};

/// Required immutable VM hardware configuration extension.
pub const VM_RESOURCES_EXTENSION: &str = "dev.kbox.vm-resources";
/// First explicit VM CPU/memory configuration contract.
pub const VM_RESOURCES_VERSION: u16 = 1;

/// Validated utility-VM resource configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmConfig {
    vcpus: u8,
    memory_mib: u32,
}

impl VmConfig {
    /// Validate a libkrun VM resource request against the shared hardware profile.
    /// The Windows WHPX host gate retains its certified one-vCPU limit.
    pub fn new(vcpus: u8, memory_mib: u32) -> Result<Self> {
        koci_core::hardware_vm::validate_profile_resources(u32::from(vcpus), memory_mib)
            .map_err(|error| Error::new(ErrorCode::InvalidArgument, error.to_string()))?;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        if vcpus != 1 {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                format!(
                    "the certified Windows WHPX path currently supports exactly 1 vCPU; \
                     requested {vcpus}"
                ),
            ));
        }
        Ok(Self { vcpus, memory_mib })
    }

    /// Virtual CPU count accepted by the existing hardware profile.
    #[must_use]
    pub const fn vcpus(self) -> u8 {
        self.vcpus
    }

    /// Guest memory in MiB.
    #[must_use]
    pub const fn memory_mib(self) -> u32 {
        self.memory_mib
    }

    /// Canonical value for the immutable required VM resource annotation.
    pub fn annotation_value(self) -> Result<String> {
        serde_json::to_string(&Descriptor {
            schema_version: VM_RESOURCES_VERSION,
            vcpus: self.vcpus,
            memory_mib: self.memory_mib,
        })
        .map_err(|error| invalid(error.to_string()))
    }

    /// Decode one bounded explicit configuration; absence retains legacy1/512.
    pub fn from_bundle(bundle: &OciBundle) -> Result<Option<Self>> {
        let Some(value) = bundle
            .spec()
            .annotations()
            .as_ref()
            .and_then(|annotations| annotations.get(VM_RESOURCES_EXTENSION))
        else {
            return Ok(None);
        };
        if value.len() > 1024 {
            return Err(invalid("VM resource descriptor exceeds1024 bytes"));
        }
        let descriptor: Descriptor = serde_json::from_str(value)
            .map_err(|error| invalid(format!("invalid VM resource descriptor: {error}")))?;
        if descriptor.schema_version != VM_RESOURCES_VERSION {
            return Err(invalid("unsupported VM resource descriptor version"));
        }
        Self::new(descriptor.vcpus, descriptor.memory_mib)
            .map(Some)
            .map_err(|error| error.for_operation("validate-attachments"))
    }
}

impl Default for VmConfig {
    fn default() -> Self {
        // The legacy1/512 agent profile satisfies shared bounds and the WHPX gate.
        Self {
            vcpus: 1,
            memory_mib: 512,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Descriptor {
    schema_version: u16,
    vcpus: u8,
    memory_mib: u32,
}

impl CreateAttachments {
    /// Require the exact versioned hardware declaration bound to the OCI bundle.
    pub fn vm_config(&self, bundle: &OciBundle) -> Result<Option<VmConfig>> {
        let configuration = VmConfig::from_bundle(bundle)?;
        match (configuration, self.extensions().get(VM_RESOURCES_EXTENSION)) {
            (None, None) => Ok(None),
            (Some(configuration), Some(extension)) => {
                let expected = super::RuntimeExtensionAttachment::from_annotation(
                    &super::decode_configuration(bundle)?,
                    VM_RESOURCES_EXTENSION,
                    VM_RESOURCES_VERSION,
                    true,
                )?;
                if extension == &expected {
                    Ok(Some(configuration))
                } else {
                    Err(invalid(
                        "VM resource extension differs from its exact required configuration",
                    ))
                }
            }
            _ => Err(invalid(
                "VM resources require the exact required dev.kbox.vm-resources version1 extension",
            )),
        }
    }
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::InvalidArgument, message).for_operation("validate-attachments")
}

#[cfg(test)]
#[path = "vm_config_tests.rs"]
mod tests;
