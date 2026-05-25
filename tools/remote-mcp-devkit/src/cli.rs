use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "remote-mcp-devkit",
    version,
    about = "Local-only Remote MCP + OAuth conformance harness"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Start the local HTTPS proxy + mock OAuth 2.1 AS.
    Up(UpArgs),
    /// Stop a running session and clean up.
    Down(DownArgs),
    /// Run OAuth/PRM/MCP conformance smoke checks against a base URL.
    Smoke(SmokeArgs),
    /// Drive the full OAuth dance from a fake AI client end-to-end.
    ClientDance(DanceArgs),
    /// Run the real AS authorization-code flow via a local callback listener.
    OauthCode(OauthCodeArgs),
    /// Check the local environment for known problems.
    Doctor(DoctorArgs),
    /// Print a sample config to stdout.
    InitConfig,
    /// List session state on disk.
    List(ListArgs),
}

#[derive(Debug, Parser)]
pub struct UpArgs {
    /// Path to remote-mcp-devkit.yaml.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Override the listen port from config.
    #[arg(long)]
    pub port: Option<u16>,
    /// Override the upstream MCP URL.
    #[arg(long)]
    pub upstream: Option<String>,
    /// Pass-through to a real OAuth AS instead of the built-in mock.
    /// `/oauth/*` and `/.well-known/oauth-authorization-server` are proxied there;
    /// PRM and 401 challenge stay in devkit.
    #[arg(long)]
    pub upstream_oauth: Option<String>,
    /// Skip the post-start smoke checks.
    #[arg(long)]
    pub no_smoke: bool,
}

#[derive(Debug, Parser)]
pub struct DownArgs {
    /// Session id (default: latest).
    #[arg(long)]
    pub session: Option<String>,
    /// Path to remote-mcp-devkit.yaml.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Force cleanup even if state file is missing.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Parser)]
pub struct SmokeArgs {
    /// Public base URL (e.g. https://localhost:8443).
    #[arg(long)]
    pub base_url: Option<String>,
    /// MCP path (default from config: /mcp).
    #[arg(long)]
    pub mcp_path: Option<String>,
    /// Output artifact directory.
    #[arg(long, default_value = "artifacts/smoke")]
    pub out: PathBuf,
    /// Path to remote-mcp-devkit.yaml (used to derive defaults).
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// OAuth client profile used for the authorize smoke request.
    ///
    /// Supported profiles: generic, claude, chatgpt.
    #[arg(long)]
    pub client_profile: Option<String>,
    /// Override the authorize request client_id.
    #[arg(long)]
    pub client_id: Option<String>,
    /// Override the authorize request redirect_uri.
    #[arg(long)]
    pub redirect_uri: Option<String>,
    /// Scope values for the authorize request. Can be repeated or comma separated.
    #[arg(long, value_delimiter = ',')]
    pub scope: Vec<String>,
    /// OAuth resource parameter. Use "auto" to send <base-url><mcp-path>.
    #[arg(long)]
    pub resource: Option<String>,
    /// Expected client_id after an upstream CIMD translation proxy rewrites it.
    #[arg(long)]
    pub expected_upstream_client_id: Option<String>,
}

#[derive(Debug, Parser)]
pub struct DanceArgs {
    #[arg(long)]
    pub base_url: Option<String>,
    #[arg(long)]
    pub mcp_path: Option<String>,
    #[arg(long, default_value = "artifacts/dance")]
    pub out: PathBuf,
    #[arg(long)]
    pub client_id: Option<String>,
    #[arg(long)]
    pub client_id_metadata_document: Option<String>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct OauthCodeArgs {
    /// Base URL of the MCP resource (e.g. https://localhost:8443).
    #[arg(long)]
    pub base_url: Option<String>,
    /// MCP path (default from config: /mcp).
    #[arg(long)]
    pub mcp_path: Option<String>,
    /// Output artifact directory.
    #[arg(long, default_value = "artifacts/oauth-code")]
    pub out: PathBuf,
    /// Config file path.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// OAuth client profile (generic / claude / chatgpt).
    #[arg(long)]
    pub client_profile: Option<String>,
    /// Override the authorize request client_id.
    #[arg(long)]
    pub client_id: Option<String>,
    /// Redirect URI. Must point at 127.0.0.1 or localhost; this tool listens on
    /// the URI's port for the auth-code callback. If the URI omits a port, the
    /// default scheme port is used (http=80, https=443).
    #[arg(long, default_value = "http://127.0.0.1:18454/callback")]
    pub redirect_uri: String,
    /// Callback capture mode: `listener` binds a local HTTP listener; `manual`
    /// reads the full callback URL from stdin after the browser redirects.
    #[arg(long, default_value = "listener")]
    pub callback_mode: String,
    /// Scope values (repeatable or comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub scope: Vec<String>,
    /// OAuth `resource` parameter. `auto` = `<base-url><mcp-path>`.
    #[arg(long)]
    pub resource: Option<String>,
    /// Seconds to wait for the browser callback before failing.
    #[arg(long, default_value_t = 300)]
    pub timeout_secs: u64,
    /// Open the system browser at the authorize URL. Default off for agent use;
    /// pass to enable interactive runs.
    #[arg(long, default_value_t = false)]
    pub open_browser: bool,
}

#[derive(Debug, Parser)]
pub struct DoctorArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ListArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,
}
