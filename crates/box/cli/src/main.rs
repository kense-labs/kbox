//! Kbox Box CLI entry point.

use clap::Parser;
use tracing_subscriber::EnvFilter;

use kbox_cli::commands::{dispatch, Cli};

const TOKIO_WORKER_STACK_BYTES: usize = 8 * 1024 * 1024;

fn main() {
    // Initialize tracing (write to stderr so stdout is clean for JSON output)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    // Deep OCI pull and VM boot futures overflow the default worker stack on macOS.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(TOKIO_WORKER_STACK_BYTES)
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Error: failed to initialize Tokio runtime: {error}");
            std::process::exit(1);
        }
    };
    let cli = Cli::parse();

    if let Err(e) = runtime.block_on(dispatch(cli)) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
