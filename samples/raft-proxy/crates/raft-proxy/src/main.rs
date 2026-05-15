use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use raft_proxy_control::ProxyApp;
use raft_proxy_core::NodeId;
use raft_proxy_network::{PeerRegistry, normalize_base_url};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "raft-proxy", about = "Raft-coordinated reverse proxy")]
struct Cli {
    /// Unique node id (1..=u64::MAX)
    #[arg(long)]
    id: NodeId,

    /// Data plane (pingora) listen address
    #[arg(long, default_value = "127.0.0.1:8080")]
    proxy_addr: String,

    /// Control plane (axum) listen address - this is also THIS node's rpc_addr
    #[arg(long, default_value = "127.0.0.1:9080")]
    admin_addr: String,

    /// Peer list as `id=base_url` pairs, comma-separated.
    /// Example: --peers 1=http://127.0.0.1:9080,2=http://127.0.0.1:9081
    /// The current node MUST be present in this list (and its base_url must match --admin-addr).
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,
}

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    validate_node_id(cli.id)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    runtime.block_on(run(cli))?;

    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
    let peers = build_peer_registry(&cli.peers)?;
    let my_rpc_addr = normalize_base_url(&cli.admin_addr);
    validate_self_peer(&peers, cli.id, &my_rpc_addr)?;

    tracing::info!(node_id = cli.id, "node starting");

    let app = Arc::new(
        ProxyApp::bootstrap(cli.id, my_rpc_addr.clone(), peers.clone())
            .await
            .context("bootstrap proxy app")?,
    );
    let routing = Arc::clone(&app.routing);
    let route_side_state = Arc::clone(&app.route_side_state);

    let proxy_addr = cli.proxy_addr.clone();
    let pingora_handle = std::thread::Builder::new()
        .name("pingora".into())
        .spawn(move || {
            let server = raft_proxy_data::build_server(routing, route_side_state, &proxy_addr);
            server.run_forever();
        })
        .context("spawn pingora thread")?;

    let listener = tokio::net::TcpListener::bind(&cli.admin_addr)
        .await
        .with_context(|| format!("bind control plane listener at {}", cli.admin_addr))?;
    tracing::info!("control plane listening on {}", cli.admin_addr);
    tracing::info!("data plane listening on {}", cli.proxy_addr);

    let router = Arc::clone(&app).router();
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve control plane")?;

    tracing::info!("axum stopped; leaving pingora thread detached until process exit");
    let _ = pingora_handle;

    Ok(())
}

fn build_peer_registry(entries: &[String]) -> Result<PeerRegistry> {
    let peers = PeerRegistry::new();

    for entry in entries {
        let (id, url) = parse_peer(entry)?;
        peers.insert(id, url);
    }

    Ok(peers)
}

fn parse_peer(entry: &str) -> Result<(NodeId, String)> {
    let (id, url) = entry
        .split_once('=')
        .ok_or_else(|| anyhow!("peer entry must be id=base_url: {entry}"))?;
    let id = id
        .trim()
        .parse::<NodeId>()
        .with_context(|| format!("parse peer id in entry {entry}"))?;
    validate_node_id(id)?;

    let url = normalize_base_url(url);
    if url == "http://" || url == "https://" {
        bail!("peer URL must not be empty: {entry}");
    }

    Ok((id, url))
}

fn validate_node_id(id: NodeId) -> Result<()> {
    if id == 0 {
        bail!("node id must be in 1..=u64::MAX");
    }

    Ok(())
}

fn validate_self_peer(peers: &PeerRegistry, node_id: NodeId, my_rpc_addr: &str) -> Result<()> {
    match peers.get(node_id) {
        Some(url) if url == my_rpc_addr => Ok(()),
        Some(url) => bail!(
            "peer entry for node {node_id} must match --admin-addr: got {url}, expected {my_rpc_addr}"
        ),
        None => bail!("peer list must include current node {node_id} with {my_rpc_addr}"),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let sigterm = signal(SignalKind::terminate());
        let sigint = signal(SignalKind::interrupt());

        match (sigterm, sigint) {
            (Ok(mut sigterm), Ok(mut sigint)) => {
                tokio::select! {
                    _ = sigterm.recv() => {},
                    _ = sigint.recv() => {},
                }
            }
            (Ok(mut sigterm), Err(err)) => {
                tracing::warn!(%err, "failed to install SIGINT handler; waiting for SIGTERM");
                let _ = sigterm.recv().await;
            }
            (Err(err), Ok(mut sigint)) => {
                tracing::warn!(%err, "failed to install SIGTERM handler; waiting for SIGINT");
                let _ = sigint.recv().await;
            }
            (Err(term_err), Err(int_err)) => {
                tracing::warn!(
                    %term_err,
                    %int_err,
                    "failed to install unix signal handlers; waiting for Ctrl-C"
                );
                wait_for_ctrl_c().await;
            }
        }
    }

    #[cfg(not(unix))]
    wait_for_ctrl_c().await;
}

async fn wait_for_ctrl_c() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!(%err, "failed to wait for Ctrl-C");
    }
}

fn init_tracing() {
    let env_filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => EnvFilter::new("raft_proxy=info,openraft=warn,pingora=warn"),
    };
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}
