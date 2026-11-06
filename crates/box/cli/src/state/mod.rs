//! State management for box instances.
//!
//! Persists box metadata to `~/.kbox/boxes.json` with atomic writes.
//! On every load, dead active PIDs are reconciled to mark boxes as dead.

mod file;
pub(crate) mod policy;
#[cfg(test)]
mod tests;

pub use file::StateFile;
pub use kbox_runtime::{BoxRecord, HealthCheck};
pub use policy::{generate_name, parse_restart_policy, validate_restart_policy};
