//! Native window UI is out of Spec050 scope.
//!
//! Upstream shipped a WebKit window; kbox keeps preference hooks for diag
//! only. No window binary is linked from this stub.

use serde_json::{Map, Value};

/// Run the (absent) window UI. Always reports unsupported.
///
/// Arguments match the historical CLI (`--start`, `--view`) so `main` keeps
/// compiling; they are ignored.
pub fn run(_start: bool, _view: Option<String>) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "sandbox window UI is not part of kbox",
    ))
}

/// Preference lookup stub used by diagnostics (`ui.json` shape).
pub fn prefs() -> Map<String, Value> {
    Map::new()
}
