use std::collections::BTreeMap;

use kbox_contract::contract::{
    ArtifactRef, IsolationLevel, ResourceLimits, RestartPolicy, RuntimeHealthCheck,
    RuntimeInspection, RuntimeLogChunk, RuntimeMount, RuntimeNetworkSpec, RuntimeProcessSpec,
    RuntimeServiceLifecycle, RuntimeUnitClass, RuntimeUnitSpec, SecretReference,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EngineErrorCode {
    UnsupportedVersion,
    UnsupportedCapability,
    Forbidden,
    InvalidRequest,
    RequestConflict,
    StaleIntent,
    GenerationMismatch,
    NotFound,
    AlreadyConnected,
    StreamLost,
    FrameTooLarge,
    CleanupPending,
    AssemblyFailed,
    RuntimeRequirementsUnmet,
    Unknown,
    Transport,
    Internal,
}

#[derive(thiserror::Error, Serialize, Deserialize)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct EngineError {
    pub code: EngineErrorCode,
    pub message: String,
    #[source]
    #[serde(skip)]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl std::fmt::Debug for EngineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EngineError")
            .field("code", &self.code)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

impl EngineError {
    pub fn new(code: EngineErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(error: std::io::Error) -> Self {
        Self {
            source: Some(Box::new(error)),
            ..Self::new(EngineErrorCode::Transport, "engine state I/O failed")
        }
    }
}

impl From<serde_json::Error> for EngineError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            source: Some(Box::new(error)),
            ..Self::new(EngineErrorCode::InvalidRequest, "invalid engine data")
        }
    }
}

impl From<kbox_contract::RuntimeError> for EngineError {
    fn from(error: kbox_contract::RuntimeError) -> Self {
        use kbox_contract::RuntimeError as R;
        let code = match &error {
            R::InvalidRequest(_) => EngineErrorCode::InvalidRequest,
            R::NotFound { .. } | R::RequestNotFound { .. } => EngineErrorCode::NotFound,
            R::RequestConflict { .. } | R::GenerationConflict { .. } => {
                EngineErrorCode::RequestConflict
            }
            R::StaleGeneration { .. } => EngineErrorCode::GenerationMismatch,
            R::UnsupportedCapabilities(_) => EngineErrorCode::UnsupportedCapability,
            R::DeadlineExceeded(_) | R::ProviderUnavailable(_) | R::Transport(_) => {
                EngineErrorCode::Unknown
            }
            R::LogDiscontinuity { .. } | R::Protocol(_) => EngineErrorCode::Internal,
        };
        Self {
            source: Some(Box::new(error)),
            ..Self::new(code, "Runtime operation did not satisfy its contract")
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EngineKey(String);

impl EngineKey {
    pub fn parse(value: impl Into<String>) -> EngineResult<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.as_bytes()[0].is_ascii_alphanumeric()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        {
            return Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid engine identifier",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EngineKey {
    type Error = EngineError;
    fn try_from(value: String) -> EngineResult<Self> {
        Self::parse(value)
    }
}
impl From<EngineKey> for String {
    fn from(value: EngineKey) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DesiredState {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineWorkload {
    OciService,
    AcpHarness,
}

/// Identity-free Service definition. The engine binds its unit and generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineServiceTemplate {
    pub artifact: ArtifactRef,
    pub process: RuntimeProcessSpec,
    pub mounts: Vec<RuntimeMount>,
    pub secrets: Vec<SecretReference>,
    pub network: RuntimeNetworkSpec,
    pub resources: ResourceLimits,
    pub isolation: IsolationLevel,
    pub health: Option<RuntimeHealthCheck>,
    pub service_lifecycle: Option<RuntimeServiceLifecycle>,
    pub restart: RestartPolicy,
}

impl EngineServiceTemplate {
    pub fn bind(&self, unit_id: String, generation: u64) -> EngineResult<RuntimeUnitSpec> {
        let spec = RuntimeUnitSpec {
            schema: RuntimeUnitSpec::SCHEMA.into(),
            unit_id,
            generation,
            class: RuntimeUnitClass::Service,
            artifact: self.artifact.clone(),
            process: self.process.clone(),
            mounts: self.mounts.clone(),
            secrets: self.secrets.clone(),
            network: self.network.clone(),
            resources: self.resources.clone(),
            isolation: self.isolation,
            health: self.health.clone(),
            service_lifecycle: self.service_lifecycle.clone(),
            restart: self.restart.clone(),
            outputs: Vec::new(),
            semantics_profile_digest: None,
            identity_attachment_digest: None,
        };
        spec.validate().map_err(|_| {
            EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid engine Service template",
            )
        })?;
        Ok(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineIntent {
    pub schema: String,
    pub engine_id: EngineKey,
    pub instance_id: EngineKey,
    pub request_id: EngineKey,
    pub revision: u64,
    pub desired_state: DesiredState,
    pub workload: EngineWorkload,
    pub plan_digest: String,
    pub service: Option<EngineServiceTemplate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<super::EngineDirectoryGrant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assembly: Option<super::EngineAssemblyPlan>,
}

impl EngineIntent {
    pub fn bind_service(&self, unit_id: String, generation: u64) -> EngineResult<RuntimeUnitSpec> {
        self.validate()?;
        let template = self.service.as_ref().ok_or_else(|| {
            EngineError::new(
                EngineErrorCode::InvalidRequest,
                "down has no Service to bind",
            )
        })?;
        let mut spec = template.bind(unit_id, generation)?;
        if self.workload == EngineWorkload::AcpHarness {
            spec.semantics_profile_digest =
                Some(crate::kbox_contract_driver::NATIVE_GUEST_PROFILE_DIGEST.into());
        }
        Ok(spec)
    }

    pub fn validate(&self) -> EngineResult<()> {
        if self.schema != "kbox.engine.intent.v1" {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedVersion,
                "unsupported engine intent schema",
            ));
        }
        if self.revision == 0 || self.revision > i64::MAX as u64 {
            return Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid intent revision",
            ));
        }
        let digest = self.plan_digest.strip_prefix("sha256:").unwrap_or("");
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid plan digest",
            ));
        }
        super::directory::validate_grants(self)?;
        if let Some(assembly) = &self.assembly {
            assembly.validate(self)?;
        }
        match (&self.desired_state, &self.service) {
            (DesiredState::Up, Some(template)) => {
                template.bind("engine-validation".into(), 1)?;
                if self.workload == EngineWorkload::AcpHarness
                    && template.isolation != IsolationLevel::Sandbox
                {
                    return Err(EngineError::new(
                        EngineErrorCode::UnsupportedCapability,
                        "the ACP Harness profile requires independent-kernel isolation",
                    ));
                }
            }
            (DesiredState::Down, None) => {}
            _ => {
                return Err(EngineError::new(
                    EngineErrorCode::InvalidRequest,
                    "up requires a Service template; down has none",
                ))
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> EngineResult<String> {
        self.validate()?;
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(self)?)
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptedOperation {
    pub schema: String,
    pub engine_id: EngineKey,
    pub instance_id: EngineKey,
    pub request_id: EngineKey,
    pub operation_id: EngineKey,
    pub intent_revision: u64,
    pub plan_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineCommand {
    Discover,
    Diagnose {},
    ApplyIntent {
        intent: Box<EngineIntent>,
    },
    InspectExecution {
        instance_id: EngineKey,
    },
    InspectOperation {
        request_id: EngineKey,
    },
    InspectAssembly {
        instance_id: EngineKey,
    },
    ReadLogs {
        instance_id: EngineKey,
        cursor: Option<String>,
    },
    OpenPrimaryProcess {
        process: Box<super::EnginePrimaryOpen>,
    },
    AttachPrimaryProcess {
        token: super::EngineAttachToken,
    },
    InspectPrimaryProcess {
        request_id: EngineKey,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineRequest {
    pub schema: String,
    pub command: EngineCommand,
}

impl EngineRequest {
    pub fn validate(&self) -> EngineResult<()> {
        if self.schema != "kbox.engine.request.v1" {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedVersion,
                "unsupported engine request schema",
            ));
        }
        if let EngineCommand::ApplyIntent { intent } = &self.command {
            intent.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineDiscovery {
    pub schema: String,
    pub engine_id: EngineKey,
    pub protocols: Vec<String>,
    pub platform: String,
    pub max_control_frame_bytes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineDiagnostics {
    pub schema: String,
    pub engine_id: EngineKey,
    pub version: String,
    pub platform: String,
    pub virtualization: EngineVirtualization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineVirtualization {
    Available { backend: String },
    Unavailable {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineExecution {
    pub accepted: AcceptedOperation,
    pub target_generation: u64,
    pub desired_state: DesiredState,
    pub runtime: Option<RuntimeInspection>,
    pub last_error: Option<EngineErrorCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineOperation {
    pub accepted: AcceptedOperation,
    pub target_generation: u64,
    pub superseded: bool,
    pub runtime: Option<kbox_contract::RuntimeRequestReceipt>,
    pub last_error: Option<EngineErrorCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EngineReplyValue {
    Discovery(EngineDiscovery),
    Diagnostics(EngineDiagnostics),
    Accepted(AcceptedOperation),
    Execution(EngineExecution),
    Operation(Box<EngineOperation>),
    Logs(Vec<RuntimeLogChunk>),
    PrimaryTicket(super::EnginePrimaryTicket),
    PrimaryProcess(super::EnginePrimaryInfo),
    Assembly(super::EngineAssembly),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineReplyBody {
    Ok { result: Box<EngineReplyValue> },
    Error { error: EngineError },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineReply {
    pub schema: String,
    #[serde(flatten)]
    pub body: EngineReplyBody,
}

impl From<EngineResult<EngineReplyValue>> for EngineReply {
    fn from(result: EngineResult<EngineReplyValue>) -> Self {
        Self {
            schema: "kbox.engine.response.v1".into(),
            body: match result {
                Ok(result) => EngineReplyBody::Ok {
                    result: Box::new(result),
                },
                Err(error) => EngineReplyBody::Error { error },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineJournalData {
    pub schema: String,
    pub engine_id: EngineKey,
    pub requests: BTreeMap<EngineKey, AcceptedIntent>,
    pub instances: BTreeMap<EngineKey, EngineKey>,
    #[serde(default)]
    pub(super) primary_processes: BTreeMap<EngineKey, super::process_protocol::EnginePrimaryRecord>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) assemblies: BTreeMap<String, super::EngineAssemblyReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptedIntent {
    pub intent: EngineIntent,
    pub fingerprint: String,
    pub accepted: AcceptedOperation,
    pub generation: u64,
    pub attempt: u64,
    pub runtime_request_id: String,
}
