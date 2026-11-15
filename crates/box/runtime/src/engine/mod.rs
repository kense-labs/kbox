//! Versioned local control plane over the existing Runtime and Box authorities.

mod controller;
mod directory;
mod journal;
mod prewarm;
mod process;
mod process_journal;
mod process_protocol;
mod process_transport;
mod protocol;
mod transport;

pub use controller::LocalEngine;
pub use directory::EngineDirectoryGrant;
pub(crate) use journal::{EngineJournal, EngineRevisionGuard};
pub use prewarm::start_reserved_environment;
pub use process_protocol::{
    EngineAttachToken, EnginePrimaryInfo, EnginePrimaryOpen, EnginePrimaryTicket,
    EngineProcessState,
};
pub use protocol::{
    AcceptedOperation, DesiredState, EngineCommand, EngineDiagnostics, EngineDiscovery,
    EngineError, EngineErrorCode, EngineExecution, EngineIntent, EngineKey, EngineOperation,
    EngineReply, EngineReplyBody, EngineReplyValue, EngineRequest, EngineResult,
    EngineServiceTemplate, EngineVirtualization,
};
pub use transport::{default_engine_endpoint, serve_engine};

#[cfg(test)]
mod tests;
