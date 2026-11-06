//! Warm VM pool for cold start optimization.
//!
//! Pre-boots MicroVMs so that `acquire()` returns an already-ready VM
//! instead of waiting for the full boot sequence.

pub mod attach;
pub mod client;
pub mod prewarm;
pub mod scaler;
pub mod warm_pool;

#[cfg(not(windows))]
pub use attach::AttachFrameReader;
pub use attach::{
    attach_client, AttachInput, AttachOutput, PoolAttachRequest, PoolAttachResponse,
    PoolClientAttach,
};
pub use client::{
    PoolClientLease, PoolClientOutput, PoolClientRun, PoolImageStat, PoolLeaseClient,
    PoolLeaseExec, PoolLeaseExecRequest, PoolLeaseReleaseRequest, PoolLeaseReleaseResponse,
    PoolLeaseRequest, PoolLeaseResponse, PoolRequest, PoolRunRequest, PoolRunResponse,
    PoolStatusResponse, PoolStopResponse,
};
pub use prewarm::PrewarmedExec;
pub use scaler::{PoolScaler, ScaleDecision};
pub use warm_pool::{PoolStats, WarmPool};
