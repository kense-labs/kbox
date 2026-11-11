use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct EngineArgs {
    #[command(subcommand)]
    command: EngineCommand,
}

#[derive(Subcommand)]
enum EngineCommand {
    /// Report the home-scoped engine endpoint as versioned JSON without starting it.
    Endpoint,
    /// Serve versioned same-user runtime control requests.
    Serve {
        /// Private Unix control socket. Defaults to the home-scoped engine endpoint.
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    #[command(hide = true)]
    StartReservedEnvironment {
        execution_id: String,
        generation: u64,
    },
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub async fn execute(args: EngineArgs) -> Result<(), Box<dyn std::error::Error>> {
    use kbox_runtime::engine::{default_engine_endpoint, serve_engine, LocalEngine};
    use kbox_runtime::BoxRuntimeDriverConfig;
    use std::sync::Arc;

    let config = BoxRuntimeDriverConfig::default();
    let socket = match args.command {
        EngineCommand::Endpoint => {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "schema":"kbox.engine.endpoint.v1", "transport":"unix",
                    "path":default_engine_endpoint(&config.home_dir)?,
                }))?
            );
            return Ok(());
        }
        EngineCommand::Serve { socket } => match socket {
            Some(socket) => socket,
            None => default_engine_endpoint(&config.home_dir)?,
        },
        EngineCommand::StartReservedEnvironment {
            execution_id,
            generation,
        } => {
            kbox_runtime::engine::start_reserved_environment(config, &execution_id, generation)
                .await?;
            return Ok(());
        }
    };
    let engine = Arc::new(tokio::task::spawn_blocking(move || LocalEngine::open(config)).await??);
    serve_engine(socket, engine, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub async fn execute(_args: EngineArgs) -> Result<(), Box<dyn std::error::Error>> {
    Err("the local engine transport requires a qualified Unix host".into())
}
