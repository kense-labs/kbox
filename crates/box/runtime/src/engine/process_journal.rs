use kbox_core::exec::ExecExit;

use super::process_protocol::EnginePrimaryRecord;
use super::{
    DesiredState, EngineError, EngineErrorCode, EngineJournal, EngineKey, EnginePrimaryInfo,
    EnginePrimaryOpen, EngineProcessState, EngineResult,
};

impl EngineJournal {
    pub(super) fn primary(&self, id: &EngineKey) -> EngineResult<&EnginePrimaryRecord> {
        self.ensure_usable()?;
        self.data.primary_processes.get(id).ok_or_else(|| {
            EngineError::new(
                EngineErrorCode::NotFound,
                "primary process request not found",
            )
        })
    }

    pub(super) fn reserve_primary(
        &mut self,
        request: &EnginePrimaryOpen,
    ) -> EngineResult<EnginePrimaryInfo> {
        self.ensure_usable()?;
        let fingerprint = request.fingerprint()?;
        if let Some(previous) = self.data.primary_processes.get(&request.request_id) {
            return Err(EngineError::new(
                if previous.fingerprint == fingerprint {
                    if previous.info.state.unfinished() {
                        EngineErrorCode::AlreadyConnected
                    } else {
                        EngineErrorCode::StreamLost
                    }
                } else {
                    EngineErrorCode::RequestConflict
                },
                "primary process requests cannot replay or resume a byte stream",
            ));
        }
        let intent = self.current(&request.instance_id)?;
        if request.engine_id != *self.engine_id() {
            return Err(EngineError::new(
                EngineErrorCode::Forbidden,
                "engine identity changed",
            ));
        }
        if intent.intent.desired_state != DesiredState::Up
            || request.generation != intent.generation
        {
            return Err(EngineError::new(
                EngineErrorCode::GenerationMismatch,
                "primary process generation changed",
            ));
        }
        if self.data.primary_processes.values().any(|entry| {
            entry.info.instance_id == request.instance_id
                && entry.info.runtime_generation == request.generation
                && entry.info.state.unfinished()
        }) {
            return Err(EngineError::new(
                EngineErrorCode::AlreadyConnected,
                "primary process already reserved",
            ));
        }
        let info = EnginePrimaryInfo {
            schema: "kbox.engine.primary.v1".into(),
            engine_id: request.engine_id.clone(),
            instance_id: request.instance_id.clone(),
            request_id: request.request_id.clone(),
            runtime_generation: request.generation,
            execution_id: None,
            execution_generation: None,
            state: EngineProcessState::Reserved,
            exit: None,
        };
        let record = EnginePrimaryRecord {
            info: info.clone(),
            fingerprint,
            intent_request_id: intent.intent.request_id.clone(),
        };
        let mut next = self.data.clone();
        next.primary_processes
            .insert(request.request_id.clone(), record);
        self.publish(next)?;
        Ok(info)
    }

    pub(super) fn require_current_primary(
        &self,
        id: &EngineKey,
    ) -> EngineResult<&EnginePrimaryRecord> {
        let record = self.primary(id)?;
        let current = self.current(&record.info.instance_id)?;
        if current.intent.desired_state != DesiredState::Up
            || current.intent.request_id != record.intent_request_id
            || current.generation != record.info.runtime_generation
            || record.info.state != EngineProcessState::Reserved
        {
            return Err(EngineError::new(
                EngineErrorCode::GenerationMismatch,
                "primary process reservation is stale",
            ));
        }
        Ok(record)
    }

    pub(super) fn primary_started(
        &mut self,
        id: &EngineKey,
        execution_id: String,
        generation: u64,
    ) -> EngineResult<EnginePrimaryInfo> {
        let mut record = self.require_current_primary(id)?.clone();
        if execution_id.is_empty() || generation == 0 {
            return Err(EngineError::new(
                EngineErrorCode::Internal,
                "provider returned invalid process identity",
            ));
        }
        record.info.execution_id = Some(execution_id);
        record.info.execution_generation = Some(generation);
        record.info.state = EngineProcessState::Running;
        let info = record.info.clone();
        let mut next = self.data.clone();
        next.primary_processes.insert(id.clone(), record);
        self.publish(next)?;
        Ok(info)
    }

    pub(super) fn finish_primary(
        &mut self,
        id: &EngineKey,
        exit: Option<ExecExit>,
    ) -> EngineResult<()> {
        let mut record = self.primary(id)?.clone();
        if !record.info.state.unfinished() {
            return Ok(());
        }
        if exit.is_some() && record.info.state != EngineProcessState::Running {
            return Err(EngineError::new(
                EngineErrorCode::Internal,
                "reserved process cannot report an exit",
            ));
        }
        record.info.state = if exit.is_some() {
            EngineProcessState::Exited
        } else {
            EngineProcessState::Lost
        };
        record.info.exit = exit;
        let mut next = self.data.clone();
        if next.instances.get(&record.info.instance_id) == Some(&record.intent_request_id) {
            let current = next
                .requests
                .get_mut(&record.intent_request_id)
                .ok_or_else(|| {
                    EngineError::new(EngineErrorCode::Internal, "primary process intent missing")
                })?;
            if current.intent.desired_state == DesiredState::Up
                && current.generation == record.info.runtime_generation
            {
                current.generation = current.generation.checked_add(1).ok_or_else(|| {
                    EngineError::new(EngineErrorCode::Internal, "execution generation overflow")
                })?;
                current.attempt = current.attempt.checked_add(1).ok_or_else(|| {
                    EngineError::new(EngineErrorCode::Internal, "environment attempt overflow")
                })?;
                current.runtime_request_id = format!(
                    "engine-{}-g{}-a{}",
                    current.intent.request_id.as_str(),
                    current.generation,
                    current.attempt
                );
            }
        }
        next.primary_processes.insert(id.clone(), record);
        self.publish(next)
    }

    pub(super) fn unfinished_primaries(&self) -> EngineResult<Vec<EnginePrimaryInfo>> {
        self.ensure_usable()?;
        Ok(self
            .data
            .primary_processes
            .values()
            .filter(|p| p.info.state.unfinished())
            .map(|p| p.info.clone())
            .collect())
    }

    pub(super) fn validate_primary_records(&self) -> EngineResult<()> {
        let mut unfinished = std::collections::BTreeSet::new();
        for (id, record) in &self.data.primary_processes {
            let info = &record.info;
            let intent = self.request(&record.intent_request_id)?;
            if id != &info.request_id
                || info.engine_id != *self.engine_id()
                || info.schema != "kbox.engine.primary.v1"
                || info.runtime_generation == 0
                || info.runtime_generation > intent.generation
                || intent.intent.instance_id != info.instance_id
                || record.fingerprint.len() != 64
                || !record.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                || info.execution_id.is_some() != info.execution_generation.is_some()
                || info.execution_id.as_ref().is_some_and(String::is_empty)
                || info.execution_generation == Some(0)
                || (matches!(
                    info.state,
                    EngineProcessState::Running | EngineProcessState::Exited
                ) && info.execution_id.is_none())
                || ((info.state == EngineProcessState::Exited) != info.exit.is_some())
                || (info.state == EngineProcessState::Reserved && info.execution_id.is_some())
                || (info.state.unfinished()
                    && !unfinished.insert((info.instance_id.clone(), info.runtime_generation)))
            {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "inconsistent primary process journal",
                ));
            }
        }
        Ok(())
    }
}
