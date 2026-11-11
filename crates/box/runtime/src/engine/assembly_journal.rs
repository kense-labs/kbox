use super::protocol::AcceptedIntent;
use super::{
    DesiredState, EngineAssemblyReceipt, EngineError, EngineErrorCode, EngineJournal, EngineResult,
};
use kbox_contract::RuntimeUnitRecord;

impl EngineJournal {
    pub(super) fn require_assembly_intent(&self, record: &AcceptedIntent) -> EngineResult<()> {
        let current = self.current(&record.intent.instance_id)?;
        if current.runtime_request_id != record.runtime_request_id
            || current.intent.desired_state != DesiredState::Up
        {
            return Err(EngineError::new(
                EngineErrorCode::StaleIntent,
                "assembly intent changed",
            ));
        }
        Ok(())
    }

    pub(super) fn assembly_matches(
        &self,
        unit: &RuntimeUnitRecord,
        record: &AcceptedIntent,
    ) -> EngineResult<bool> {
        self.ensure_usable()?;
        let Some(plan) = &record.intent.assembly else {
            return Ok(true);
        };
        Ok(self
            .data
            .assemblies
            .get(&record.runtime_request_id)
            .is_some_and(|receipt| {
                receipt.generation == unit.spec.generation
                    && receipt.package_digest == plan.digest
                    && receipt.spec_digest == unit.observation.spec_digest
                    && unit.observation.provider_resource_id.as_deref()
                        == Some(receipt.execution_id.as_str())
            }))
    }

    pub(super) fn require_assembly(
        &self,
        unit: &RuntimeUnitRecord,
        record: &AcceptedIntent,
    ) -> EngineResult<()> {
        self.require_assembly_intent(record)?;
        if !self.assembly_matches(unit, record)? {
            return Err(EngineError::new(
                EngineErrorCode::CleanupPending,
                "Harness package assembly is not ready for this execution",
            ));
        }
        Ok(())
    }

    pub(super) fn publish_assembly(
        &mut self,
        record: &AcceptedIntent,
        receipt: EngineAssemblyReceipt,
    ) -> EngineResult<()> {
        self.require_assembly_intent(record)?;
        self.validate_assembly(&receipt)?;
        if let Some(previous) = self.data.assemblies.get(&record.runtime_request_id) {
            if previous != &receipt {
                return Err(invalid());
            }
            return Ok(());
        }
        let mut next = self.data.clone();
        next.assemblies
            .insert(record.runtime_request_id.clone(), receipt);
        self.publish(next)
    }

    pub(super) fn validate_assemblies(&self) -> EngineResult<()> {
        for (key, receipt) in &self.data.assemblies {
            if key != &receipt.runtime_request_id {
                return Err(invalid());
            }
            self.validate_assembly(receipt)?;
        }
        Ok(())
    }

    fn validate_assembly(&self, receipt: &EngineAssemblyReceipt) -> EngineResult<()> {
        let record = self.request(&receipt.accepted.request_id)?;
        let plan = record.intent.assembly.as_ref().ok_or_else(invalid)?;
        let first = record
            .generation
            .checked_sub(record.attempt.saturating_sub(1))
            .ok_or_else(invalid)?;
        if receipt.schema != "kbox.engine.assembly.v1"
            || receipt.accepted != record.accepted
            || receipt.generation < first
            || receipt.generation > record.generation
            || receipt.execution_id.is_empty()
            || receipt.execution_generation == 0
            || receipt.package_digest != plan.digest
            || receipt.root != plan.root()
            || receipt.spec_digest
                != record
                    .intent
                    .bind_service(self.unit_id(&record.intent.instance_id), receipt.generation)?
                    .digest()
                    .map_err(|_| invalid())?
            || receipt.runtime_request_id
                != format!(
                    "engine-{}-g{}-a{}",
                    record.intent.request_id.as_str(),
                    receipt.generation,
                    receipt.generation - first + 1
                )
        {
            return Err(invalid());
        }
        Ok(())
    }
}

fn invalid() -> EngineError {
    EngineError::new(
        EngineErrorCode::InvalidRequest,
        "invalid package assembly receipt binding",
    )
}
