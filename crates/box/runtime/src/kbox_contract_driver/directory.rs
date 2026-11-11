//! Caller-authorized live directory grants. Box owns actual mount lifecycle.
use std::path::PathBuf;

use async_trait::async_trait;
use kbox_contract::contract::{RuntimeMount, RuntimeUnitSpec};

#[async_trait]
pub trait BoxDirectoryPort: Send + Sync {
    /// Resolve the exact Directory reference granted to this unit and generation.
    /// Keep this binding stable through teardown, even if its original path disappears.
    async fn mount_path(
        &self,
        spec: &RuntimeUnitSpec,
        mount: &RuntimeMount,
    ) -> Result<PathBuf, BoxDirectoryPortError>;

    /// Release materialization views, retaining the caller's original directory/data.
    async fn cleanup_spec(&self, spec_digest: &str) -> Result<(), BoxDirectoryPortError>;
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;

#[cfg(all(test, unix, feature = "vm"))]
#[path = "directory_native.rs"]
mod native;

#[derive(Debug, thiserror::Error)]
pub enum BoxDirectoryPortError {
    #[error("Directory grant was rejected: {0}")]
    Rejected(String),
    #[error("Directory grant is unavailable: {0}")]
    Unavailable(String),
}

pub(super) fn map_error(error: BoxDirectoryPortError) -> kbox_contract::RuntimeError {
    match error {
        BoxDirectoryPortError::Rejected(_) => kbox_contract::RuntimeError::InvalidRequest(
            "Directory grant rejected by the caller boundary".into(),
        ),
        BoxDirectoryPortError::Unavailable(_) => kbox_contract::RuntimeError::ProviderUnavailable(
            "Directory grant boundary is unavailable".into(),
        ),
    }
}
