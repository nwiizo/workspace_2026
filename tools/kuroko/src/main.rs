use std::process::ExitCode;

use clap::Parser;
use kuroko::config::Config;
use kuroko::server;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "kuroko",
    version,
    about = "A lightweight AWS service emulator in Rust",
    long_about = "kuroko runs as a local AWS-compatible endpoint for CI and local \
                  development. No authentication required, single binary, fast startup."
)]
struct Cli {
    /// Host to bind (overrides KUROKO_HOST)
    #[arg(long, env = "KUROKO_HOST")]
    host: Option<String>,

    /// Port to bind (overrides KUROKO_PORT)
    #[arg(long, env = "KUROKO_PORT")]
    port: Option<u16>,

    /// Directory for JSON-snapshot persistence (overrides KUROKO_DATA_DIR)
    #[arg(long, env = "KUROKO_DATA_DIR")]
    data_dir: Option<String>,

    /// Log filter (e.g. info, debug, kuroko=debug)
    #[arg(long, env = "KUROKO_LOG", default_value = "info")]
    log: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(&cli.log).unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(true)
        .init();

    let mut config = Config::from_env();
    if let Some(h) = cli.host {
        config.host = h;
    }
    if let Some(p) = cli.port {
        config.port = p;
    }
    if let Some(d) = cli.data_dir {
        config.data_dir = Some(d);
    }

    match server::run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "server exited with error");
            ExitCode::FAILURE
        }
    }
}
