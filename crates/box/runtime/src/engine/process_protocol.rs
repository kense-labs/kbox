use std::collections::BTreeMap;

use kbox_core::exec::{ExecExit, ExecRequest, UNLIMITED_STREAM_TIMEOUT_NS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{EngineError, EngineErrorCode, EngineKey, EngineResult};

pub(super) const BOOTSTRAP_PROTOCOL: &str = "process-stream.bootstrap.v1";
pub(super) const MAX_BOOTSTRAP_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnginePrimaryOpen {
    pub schema: String,
    pub engine_id: EngineKey,
    pub instance_id: EngineKey,
    pub request_id: EngineKey,
    pub generation: u64,
    pub command: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_directory: Option<String>,
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bootstrap: bool,
}

impl std::fmt::Debug for EnginePrimaryOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnginePrimaryOpen")
            .field("instance_id", &self.instance_id)
            .field("request_id", &self.request_id)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl EnginePrimaryOpen {
    pub fn validate(&self) -> EngineResult<()> {
        if self.schema != "kbox.engine.primary-open.v1" {
            return Err(EngineError::new(
                EngineErrorCode::UnsupportedVersion,
                "unsupported primary process request",
            ));
        }
        if self.generation == 0
            || self.command.first().is_none_or(String::is_empty)
            || self.command.iter().any(|value| value.contains('\0'))
            || self.environment.iter().any(|(name, value)| {
                name.is_empty() || name.contains(['=', '\0']) || value.contains('\0')
            })
            || self.working_directory.as_ref().is_some_and(|path| {
                !std::path::Path::new(path).is_absolute() || path.contains('\0')
            })
        {
            return Err(EngineError::new(
                EngineErrorCode::InvalidRequest,
                "invalid primary process parameters",
            ));
        }
        self.timeout_ns()?;
        Ok(())
    }

    fn timeout_ns(&self) -> EngineResult<u64> {
        match self.timeout_ms {
            None => Ok(UNLIMITED_STREAM_TIMEOUT_NS),
            Some(ms) => ms
                .checked_mul(1_000_000)
                .filter(|value| *value > 0 && *value != UNLIMITED_STREAM_TIMEOUT_NS)
                .ok_or_else(|| {
                    EngineError::new(EngineErrorCode::InvalidRequest, "invalid process timeout")
                }),
        }
    }

    pub(super) fn fingerprint(&self) -> EngineResult<String> {
        self.validate()?;
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }

    pub(super) fn exec_request(&self) -> EngineResult<ExecRequest> {
        self.validate()?;
        Ok(ExecRequest {
            request_id: None,
            cmd: self.command.clone(),
            timeout_ns: self.timeout_ns()?,
            env: self
                .environment
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect(),
            working_dir: self.working_directory.clone(),
            rootfs: None,
            stdin: None,
            stdin_streaming: true,
            user: None,
            streaming: true,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EngineAttachToken(pub(super) EngineKey);

impl std::fmt::Debug for EngineAttachToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EngineAttachToken([redacted])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineProcessState {
    Reserved,
    Running,
    Exited,
    Lost,
}

impl EngineProcessState {
    pub(super) fn unfinished(self) -> bool {
        matches!(self, Self::Reserved | Self::Running)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnginePrimaryInfo {
    pub schema: String,
    pub engine_id: EngineKey,
    pub instance_id: EngineKey,
    pub request_id: EngineKey,
    pub runtime_generation: u64,
    pub execution_id: Option<String>,
    pub execution_generation: Option<u64>,
    pub state: EngineProcessState,
    pub exit: Option<ExecExit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnginePrimaryTicket {
    pub process: EnginePrimaryInfo,
    pub attach_token: EngineAttachToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EnginePrimaryRecord {
    pub info: EnginePrimaryInfo,
    pub intent_request_id: EngineKey,
    pub fingerprint: String,
}
