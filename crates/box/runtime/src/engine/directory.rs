//! Explicit directory authorization frozen in accepted intent, never a second mount registry.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use kbox_contract::contract::{RuntimeMount, RuntimeMountSource, RuntimeUnitSpec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::{EngineError, EngineErrorCode, EngineIntent, EngineJournal, EngineResult};
use crate::{BoxDirectoryPort, BoxDirectoryPortError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineDirectoryGrant {
    pub reference: String,
    pub source: PathBuf,
    pub read_only: bool,
}

impl EngineDirectoryGrant {
    /// The reference binds path identity, not the mutable directory contents.
    pub fn new(source: PathBuf, read_only: bool) -> EngineResult<Self> {
        if !source.is_absolute()
            || source
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        {
            return Err(invalid());
        }
        let path = source.to_str().ok_or_else(invalid)?;
        if path.contains(['\0', ':', '\r', '\n']) {
            return Err(invalid());
        }
        let mut digest = Sha256::new();
        digest.update(b"kbox.engine.directory.v1\0");
        digest.update(path.as_bytes());
        Ok(Self {
            reference: format!("directory.{:x}", digest.finalize()),
            source,
            read_only,
        })
    }
}

pub(super) fn validate_grants(intent: &EngineIntent) -> EngineResult<()> {
    let mut grants = BTreeMap::new();
    for grant in &intent.directories {
        if *grant != EngineDirectoryGrant::new(grant.source.clone(), grant.read_only)?
            || grants.insert(grant.reference.as_str(), grant).is_some()
        {
            return Err(invalid());
        }
    }
    let mut used = BTreeSet::new();
    if let Some(template) = &intent.service {
        for mount in &template.mounts {
            if let RuntimeMountSource::Directory { reference } = &mount.source {
                let grant = grants.get(reference.as_str()).ok_or_else(invalid)?;
                if grant.read_only && !mount.read_only {
                    return Err(invalid());
                }
                used.insert(reference.as_str());
            }
        }
    }
    if used.len() != grants.len() {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> EngineError {
    EngineError::new(
        EngineErrorCode::InvalidRequest,
        "invalid explicit directory grants",
    )
}

pub(super) struct EngineDirectories(pub Arc<Mutex<EngineJournal>>);

#[async_trait]
impl BoxDirectoryPort for EngineDirectories {
    async fn mount_path(
        &self,
        spec: &RuntimeUnitSpec,
        mount: &RuntimeMount,
    ) -> Result<PathBuf, BoxDirectoryPortError> {
        let journal = self.0.lock().await;
        journal
            .ensure_usable()
            .map_err(|_| BoxDirectoryPortError::Unavailable("intent journal unavailable".into()))?;
        for record in journal.data.requests.values() {
            // A completed primary advances the attempt in this same accepted intent.
            let first = record
                .generation
                .checked_sub(record.attempt.saturating_sub(1));
            if first.is_none_or(|first| spec.generation < first)
                || spec.generation > record.generation
                || journal.unit_id(&record.intent.instance_id) != spec.unit_id
                || record.intent.service.is_none()
            {
                continue;
            }
            if record
                .intent
                .bind_service(spec.unit_id.clone(), spec.generation)
                .ok()
                .as_ref()
                != Some(spec)
                || !spec.mounts.contains(mount)
            {
                continue;
            }
            let RuntimeMountSource::Directory { reference } = &mount.source else {
                break;
            };
            if let Some(grant) = record
                .intent
                .directories
                .iter()
                .find(|grant| &grant.reference == reference)
            {
                return Ok(grant.source.clone());
            }
        }
        Err(BoxDirectoryPortError::Rejected(
            "directory is not granted to this exact specification".into(),
        ))
    }

    async fn cleanup_spec(&self, _digest: &str) -> Result<(), BoxDirectoryPortError> {
        // The canonical Box teardown removes its share. Caller data and frozen intent survive.
        Ok(())
    }
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
