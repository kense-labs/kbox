use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use kbox_contract::{RuntimeAdmissionGuard, RuntimeError, RuntimeResult};
use tokio::sync::Mutex;

use crate::file_lock::FileLock;

use super::protocol::{AcceptedIntent, EngineJournalData};
use super::{
    AcceptedOperation, DesiredState, EngineError, EngineErrorCode, EngineIntent, EngineKey,
    EngineResult,
};

const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct EngineJournal {
    path: PathBuf,
    pub(super) data: EngineJournalData,
    poisoned: bool,
    _lock: FileLock,
}

impl std::fmt::Debug for EngineJournal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EngineJournal")
            .field("path", &self.path)
            .field("engine_id", &self.data.engine_id)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl EngineJournal {
    pub fn open(path: PathBuf) -> EngineResult<Self> {
        private_parent(&path)?;
        let lock = FileLock::try_acquire(&path)?.ok_or_else(|| {
            EngineError::new(
                EngineErrorCode::RequestConflict,
                "engine journal has a live owner",
            )
        })?;
        let data = if path.try_exists()? {
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.len() > MAX_JOURNAL_BYTES {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "invalid engine journal file",
                ));
            }
            let mut bytes = Vec::new();
            std::fs::File::open(&path)?
                .take(MAX_JOURNAL_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_JOURNAL_BYTES {
                return Err(EngineError::new(
                    EngineErrorCode::FrameTooLarge,
                    "engine journal limit",
                ));
            }
            serde_json::from_slice::<EngineJournalData>(&bytes)?
        } else {
            EngineJournalData {
                schema: "kbox.engine.journal.v1".into(),
                engine_id: EngineKey::parse(uuid::Uuid::new_v4().to_string())?,
                requests: BTreeMap::new(),
                instances: BTreeMap::new(),
                primary_processes: BTreeMap::new(),
                assemblies: BTreeMap::new(),
            }
        };
        if data.schema != "kbox.engine.journal.v1" {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedVersion,
                "unsupported engine journal",
            ));
        }
        let journal = Self {
            path,
            data,
            poisoned: false,
            _lock: lock,
        };
        journal.validate()?;
        if !journal.path.try_exists()? {
            journal.persist()?;
        }
        Ok(journal)
    }

    pub fn engine_id(&self) -> &EngineKey {
        &self.data.engine_id
    }

    pub fn accept(&mut self, intent: EngineIntent) -> EngineResult<AcceptedOperation> {
        self.ensure_usable()?;
        intent.validate()?;
        if intent.engine_id != self.data.engine_id {
            return Err(EngineError::new(
                EngineErrorCode::Forbidden,
                "engine identity changed",
            ));
        }
        let fingerprint = intent.fingerprint()?;
        if let Some(previous) = self.data.requests.get(&intent.request_id) {
            if previous.fingerprint != fingerprint {
                return Err(EngineError::new(
                    EngineErrorCode::RequestConflict,
                    "request identity has different content",
                ));
            }
            return Ok(previous.accepted.clone());
        }
        let previous = self.current(&intent.instance_id).ok();
        if let Some(previous) = &previous {
            if previous.intent.revision > intent.revision {
                return Err(EngineError::new(
                    EngineErrorCode::StaleIntent,
                    "superseded intent revision",
                ));
            }
            if previous.intent.revision == intent.revision {
                return Err(EngineError::new(
                    EngineErrorCode::RequestConflict,
                    "revision already belongs to another request",
                ));
            }
        }
        let generation = match previous {
            Some(previous) if intent.desired_state == DesiredState::Up => {
                previous.generation.checked_add(1).ok_or_else(|| {
                    EngineError::new(EngineErrorCode::InvalidRequest, "generation overflow")
                })?
            }
            Some(previous) => previous.generation,
            None => 1,
        };
        let accepted = AcceptedOperation {
            schema: "kbox.engine.accepted.v1".into(),
            engine_id: self.data.engine_id.clone(),
            instance_id: intent.instance_id.clone(),
            request_id: intent.request_id.clone(),
            operation_id: EngineKey::parse(uuid::Uuid::new_v4().to_string())?,
            intent_revision: intent.revision,
            plan_digest: intent.plan_digest.clone(),
        };
        let record = AcceptedIntent {
            fingerprint,
            accepted: accepted.clone(),
            generation,
            attempt: 1,
            runtime_request_id: format!("engine-{}-g{}-a1", intent.request_id.as_str(), generation),
            intent,
        };
        let mut next = self.data.clone();
        next.instances.insert(
            record.intent.instance_id.clone(),
            record.intent.request_id.clone(),
        );
        next.requests
            .insert(record.intent.request_id.clone(), record);
        self.publish(next)?;
        Ok(accepted)
    }

    pub(crate) fn current(&self, instance: &EngineKey) -> EngineResult<&AcceptedIntent> {
        self.ensure_usable()?;
        self.data
            .instances
            .get(instance)
            .and_then(|id| self.data.requests.get(id))
            .ok_or_else(|| EngineError::new(EngineErrorCode::NotFound, "instance not found"))
    }

    pub(crate) fn request(&self, id: &EngineKey) -> EngineResult<&AcceptedIntent> {
        self.ensure_usable()?;
        self.data
            .requests
            .get(id)
            .ok_or_else(|| EngineError::new(EngineErrorCode::NotFound, "operation not found"))
    }

    pub(crate) fn current_requests(&self) -> EngineResult<Vec<AcceptedIntent>> {
        self.ensure_usable()?;
        Ok(self
            .data
            .instances
            .values()
            .filter_map(|id| self.data.requests.get(id).cloned())
            .collect())
    }

    pub(crate) fn unit_id(&self, instance: &EngineKey) -> String {
        format!(
            "engine.{}.{}",
            self.data.engine_id.as_str(),
            instance.as_str()
        )
    }

    pub(super) fn publish(&mut self, next: EngineJournalData) -> EngineResult<()> {
        // A failed directory sync may follow a successful rename. Reopen before retrying.
        if let Err(error) = persist(&self.path, &next) {
            self.poisoned = true;
            return Err(error);
        }
        self.data = next;
        Ok(())
    }

    pub(super) fn ensure_usable(&self) -> EngineResult<()> {
        if self.poisoned {
            return Err(EngineError::new(
                EngineErrorCode::Unknown,
                "reopen engine journal after an uncertain persistence result",
            ));
        }
        Ok(())
    }

    fn persist(&self) -> EngineResult<()> {
        persist(&self.path, &self.data)
    }

    fn validate(&self) -> EngineResult<()> {
        self.validate_primary_records()?;
        self.validate_assemblies()?;
        let mut latest = BTreeMap::new();
        let mut operations = std::collections::BTreeSet::new();
        let mut revisions = std::collections::BTreeSet::new();
        for (id, record) in &self.data.requests {
            record.intent.validate()?;
            if id != &record.intent.request_id
                || record.fingerprint != record.intent.fingerprint()?
                || record.accepted.engine_id != self.data.engine_id
                || record.intent.engine_id != self.data.engine_id
                || record.accepted.schema != "kbox.engine.accepted.v1"
                || record.accepted.request_id != *id
                || record.accepted.instance_id != record.intent.instance_id
                || record.generation == 0
                || record.attempt == 0
                || record.accepted.intent_revision != record.intent.revision
                || record.accepted.plan_digest != record.intent.plan_digest
                || record.runtime_request_id
                    != format!(
                        "engine-{}-g{}-a{}",
                        id.as_str(),
                        record.generation,
                        record.attempt
                    )
                || !operations.insert(record.accepted.operation_id.clone())
                || !revisions.insert((record.intent.instance_id.clone(), record.intent.revision))
            {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "inconsistent engine journal",
                ));
            }
            let current = latest
                .entry(record.intent.instance_id.clone())
                .or_insert((0, id));
            if current.0 == record.intent.revision {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "duplicate intent revision",
                ));
            }
            if current.0 < record.intent.revision {
                *current = (record.intent.revision, id);
            }
        }
        if latest.len() != self.data.instances.len() {
            return Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "incomplete instance index",
            ));
        }
        for (instance, request) in &self.data.instances {
            if self.request(request)?.intent.instance_id != *instance
                || latest.get(instance).map(|(_, id)| *id) != Some(request)
            {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "inconsistent instance index",
                ));
            }
        }
        Ok(())
    }
}

fn private_parent(path: &Path) -> EngineResult<&Path> {
    let parent = path.parent().ok_or_else(|| {
        EngineError::new(
            EngineErrorCode::InvalidRequest,
            "journal path has no parent",
        )
    })?;
    let mut directory = std::fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(parent)?;
    Ok(parent)
}

fn persist(path: &Path, value: &EngineJournalData) -> EngineResult<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err(EngineError::new(
            EngineErrorCode::FrameTooLarge,
            "engine journal limit",
        ));
    }
    let parent = private_parent(path)?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    kbox_core::fs_atomic::write_durable(temporary.path(), path, &bytes)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub(crate) struct EngineRevisionGuard {
    journal: Arc<Mutex<EngineJournal>>,
}

impl EngineRevisionGuard {
    pub fn new(journal: Arc<Mutex<EngineJournal>>) -> Self {
        Self { journal }
    }
}

#[async_trait]
impl RuntimeAdmissionGuard for EngineRevisionGuard {
    async fn check_apply(
        &self,
        request: &kbox_contract::contract::RuntimeApplyRequest,
    ) -> RuntimeResult<()> {
        let journal = self.journal.lock().await;
        let prefix = format!("engine.{}.", journal.engine_id().as_str());
        let current = request
            .spec
            .unit_id
            .strip_prefix(&prefix)
            .and_then(|id| EngineKey::parse(id).ok())
            .and_then(|id| journal.current(&id).ok());
        match current {
            Some(intent)
                if intent.intent.desired_state == DesiredState::Up
                    && intent.runtime_request_id == request.request_id
                    && intent.generation == request.spec.generation
                    && intent
                        .intent
                        .bind_service(request.spec.unit_id.clone(), intent.generation)
                        .is_ok_and(|spec| spec == request.spec) =>
            {
                Ok(())
            }
            _ => Err(RuntimeError::RequestConflict {
                request_id: request.request_id.clone(),
            }),
        }
    }
}
