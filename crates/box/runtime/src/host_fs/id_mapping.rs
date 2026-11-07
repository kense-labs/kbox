//! Host filesystem UID/GID mapping plans used by MicroVM rootfs helpers.

use kbox_core::error::{BoxError, Result};
use serde::{Deserialize, Serialize};

/// One contiguous user-namespace ID mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdMapping {
    pub container_id: u32,
    pub host_id: u32,
    pub size: u32,
}

/// Exact mappings used to translate host-visible rootfs ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdMappingPlan {
    pub uid_mappings: Vec<IdMapping>,
    pub gid_mappings: Vec<IdMapping>,
    pub maximum_container_uid: u32,
    pub maximum_container_gid: u32,
}

/// Validate one exact Box-produced user-namespace mapping plan.
pub(crate) fn validate_id_mapping_plan(plan: &IdMappingPlan) -> Result<()> {
    validate_mapping_set(&plan.uid_mappings, plan.maximum_container_uid, "UID")?;
    validate_mapping_set(&plan.gid_mappings, plan.maximum_container_gid, "GID")?;
    if plan
        .uid_mappings
        .iter()
        .any(|mapping| mapping.container_id == 0 && mapping.host_id == 0)
        || plan
            .gid_mappings
            .iter()
            .any(|mapping| mapping.container_id == 0 && mapping.host_id == 0)
    {
        return Err(BoxError::ConfigError(
            "container root must not map to host root".to_string(),
        ));
    }
    Ok(())
}

fn validate_mapping_set(mappings: &[IdMapping], maximum: u32, kind: &str) -> Result<()> {
    if mappings.is_empty() || mappings[0].container_id != 0 {
        return Err(BoxError::ConfigError(format!(
            "{kind} mappings must start at container ID 0"
        )));
    }
    let mut next = 0u32;
    let mut host_ranges = Vec::new();
    for mapping in mappings {
        if mapping.size == 0 || mapping.container_id != next {
            return Err(BoxError::ConfigError(format!(
                "{kind} mappings must be contiguous and non-empty"
            )));
        }
        next = next.checked_add(mapping.size).ok_or_else(|| {
            BoxError::ConfigError(format!("{kind} container mapping overflows"))
        })?;
        let host_end = mapping.host_id.checked_add(mapping.size).ok_or_else(|| {
            BoxError::ConfigError(format!("{kind} host mapping overflows"))
        })?;
        if host_ranges
            .iter()
            .any(|(start, end)| mapping.host_id < *end && *start < host_end)
        {
            return Err(BoxError::ConfigError(format!(
                "{kind} host mappings overlap"
            )));
        }
        host_ranges.push((mapping.host_id, host_end));
    }
    if next <= maximum {
        return Err(BoxError::ConfigError(format!(
            "{kind} mappings do not cover container ID {maximum}"
        )));
    }
    Ok(())
}

/// Map a container UID into the host UID using the exact plan.
pub fn map_container_uid(plan: &IdMappingPlan, container_uid: u32) -> Result<u32> {
    map_container_id(&plan.uid_mappings, container_uid, "UID")
}

/// Map a container GID into the host GID using the exact plan.
pub fn map_container_gid(plan: &IdMappingPlan, container_gid: u32) -> Result<u32> {
    map_container_id(&plan.gid_mappings, container_gid, "GID")
}

/// Unmap a host UID back to the container UID using the exact plan.
pub fn unmap_host_uid(plan: &IdMappingPlan, host_uid: u32) -> Result<u32> {
    unmap_host_id(&plan.uid_mappings, host_uid, "UID")
}

/// Unmap a host GID back to the container GID using the exact plan.
pub fn unmap_host_gid(plan: &IdMappingPlan, host_gid: u32) -> Result<u32> {
    unmap_host_id(&plan.gid_mappings, host_gid, "GID")
}

fn map_container_id(mappings: &[IdMapping], id: u32, kind: &str) -> Result<u32> {
    for mapping in mappings {
        if id >= mapping.container_id && id < mapping.container_id.saturating_add(mapping.size) {
            return Ok(mapping.host_id + (id - mapping.container_id));
        }
    }
    Err(BoxError::ConfigError(format!(
        "container {kind} {id} is outside the mapping plan"
    )))
}

fn unmap_host_id(mappings: &[IdMapping], id: u32, kind: &str) -> Result<u32> {
    for mapping in mappings {
        if id >= mapping.host_id && id < mapping.host_id.saturating_add(mapping.size) {
            return Ok(mapping.container_id + (id - mapping.host_id));
        }
    }
    Err(BoxError::ConfigError(format!(
        "host {kind} {id} is outside the mapping plan"
    )))
}
