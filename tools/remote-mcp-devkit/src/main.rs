use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use remote_mcp_devkit::{
    cleanup,
    cli::{Cli, Cmd, DanceArgs, DoctorArgs, DownArgs, ListArgs, OauthCodeArgs, SmokeArgs, UpArgs},
    client_dance,
    config::Config,
    doctor,
    mock_as::MockAsState,
    oauth_code, proxy, smoke,
    state::{CleanupStatus, SessionState},
    tls,
};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        // already installed — fine
    }

    let cli = Cli::parse();
    match cli.command {
        Cmd::Up(args) => cmd_up(args).await,
        Cmd::Down(args) => cmd_down(args),
        Cmd::Smoke(args) => cmd_smoke(args).await,
        Cmd::ClientDance(args) => cmd_dance(args).await,
        Cmd::OauthCode(args) => cmd_oauth_code(args).await,
        Cmd::Doctor(args) => cmd_doctor(args).await,
        Cmd::InitConfig => {
            let yaml = serde_yaml::to_string(&Config::sample())?;
            println!("{yaml}");
            Ok(())
        }
        Cmd::List(args) => cmd_list(args),
    }
}

async fn cmd_up(args: UpArgs) -> anyhow::Result<()> {
    let mut cfg = Config::load_or_default(args.config.as_deref())?;
    if let Some(p) = args.port {
        cfg.server.port = p;
    }
    if let Some(u) = args.upstream {
        cfg.upstreams.mcp = Some(remote_mcp_devkit::config::Upstream { url: u });
    }
    if let Some(u) = args.upstream_oauth {
        cfg.upstreams.oauth = Some(remote_mcp_devkit::config::Upstream { url: u });
    }

    std::fs::create_dir_all(&cfg.workspace.state_dir)?;
    std::fs::create_dir_all(&cfg.workspace.artifact_dir)?;

    let cert_paths = tls::ensure_self_signed(&cfg.workspace.state_dir, &cfg.server.host)
        .context("generate self-signed TLS")?;

    let base_url = cfg.server.base_url();
    let mock_as = MockAsState::new(base_url.clone(), cfg.profile.mcp_path.clone());
    let app = proxy::router(&cfg, mock_as);

    let now = chrono::Utc::now();
    let session_id = SessionState::new_id(now);
    let artifact_dir = cfg.workspace.artifact_dir.join(&session_id);
    std::fs::create_dir_all(&artifact_dir)?;

    let addr: SocketAddr = resolve_bind_addr(&cfg.server.host, cfg.server.port).await?;

    let session = SessionState {
        session_id: session_id.clone(),
        started_at: now,
        public_base_url: base_url.clone(),
        listen_addr: addr.to_string(),
        upstream_mcp_url: cfg.upstreams.mcp.as_ref().map(|u| u.url.clone()),
        pid: std::process::id(),
        artifact_dir: artifact_dir.clone(),
        cert_path: cert_paths.cert.clone(),
        key_path: cert_paths.key.clone(),
        cleanup_status: CleanupStatus::Running,
    };
    let state_path = session.save(&cfg.workspace.state_dir)?;

    // Machine-readable session line goes to stdout — one JSON object so an agent
    // can `read line, parse JSON` to get the session id, base_url, and paths.
    let session_json = serde_json::json!({
        "event": "session_started",
        "session_id": session.session_id,
        "base_url": base_url,
        "mcp_url": format!("{base_url}{}", cfg.profile.mcp_path),
        "prm_url": format!("{base_url}/.well-known/oauth-protected-resource"),
        "as_metadata_url": format!("{base_url}/.well-known/oauth-authorization-server"),
        "upstream_mcp_url": session.upstream_mcp_url,
        "cert_path": cert_paths.cert,
        "artifact_dir": artifact_dir,
        "state_path": state_path,
    });
    println!("{session_json}");

    // Human-friendly banner stays on stderr so it never mixes with the JSON.
    eprintln!("┌─ remote-mcp-devkit session started");
    eprintln!("│  session     : {}", session.session_id);
    eprintln!("│  base_url    : {}", base_url);
    eprintln!("│  mcp_url     : {}{}", base_url, cfg.profile.mcp_path);
    eprintln!(
        "│  prm         : {}/.well-known/oauth-protected-resource",
        base_url
    );
    eprintln!(
        "│  as_metadata : {}/.well-known/oauth-authorization-server",
        base_url
    );
    eprintln!(
        "│  upstream    : {}",
        session
            .upstream_mcp_url
            .clone()
            .unwrap_or_else(|| "(none — authorized-echo)".to_string())
    );
    eprintln!("│  cert        : {}", cert_paths.cert.display());
    eprintln!("│  artifacts   : {}", artifact_dir.display());
    eprintln!("│  state       : {}", state_path.display());
    eprintln!(
        "└─ Stop with: remote-mcp-devkit down --session {}",
        session.session_id
    );
    eprintln!();

    let tls_config = RustlsConfig::from_pem_file(&cert_paths.cert, &cert_paths.key)
        .await
        .context("load TLS cert/key")?;

    // Run server with a cancellation watcher for Ctrl-C cleanup.
    let server_handle = axum_server::Handle::new();
    let shutdown_handle = server_handle.clone();
    let state_dir_for_signal = cfg.workspace.state_dir.clone();
    let artifact_dir_for_signal = cfg.workspace.artifact_dir.clone();
    let session_id_for_signal = session.session_id.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("\n[remote-mcp-devkit] SIGINT received, shutting down…");
        shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(3)));
        // force=false: mark stopped and remove the state JSON, but keep the
        // artifact dir for the operator / agent to inspect.
        let _ = cleanup::run(
            &state_dir_for_signal,
            &artifact_dir_for_signal,
            &session_id_for_signal,
            false,
        );
    });

    if !args.no_smoke {
        let smoke_base = base_url.clone();
        let smoke_path = cfg.profile.mcp_path.clone();
        let smoke_dir = artifact_dir.join("startup-smoke");
        tokio::spawn(async move {
            // small grace period for the listener
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            match smoke::run(&smoke_base, &smoke_path, &smoke_dir).await {
                Ok(r) => {
                    let pass = r.passed();
                    eprintln!(
                        "[remote-mcp-devkit] startup smoke: {} ({} checks)",
                        if pass { "PASS" } else { "FAIL" },
                        r.checks.len()
                    );
                    if !pass {
                        for c in &r.checks {
                            if !c.passed {
                                eprintln!("  ! {}: {:?}", c.name, c.messages);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[remote-mcp-devkit] startup smoke error: {e}"),
            }
        });
    }

    axum_server::bind_rustls(addr, tls_config)
        .handle(server_handle)
        .serve(app.into_make_service())
        .await
        .context("serve")?;

    // Mark stopped on graceful exit. force=false so artifacts persist for review.
    let _ = cleanup::run(
        &cfg.workspace.state_dir,
        &cfg.workspace.artifact_dir,
        &session.session_id,
        false,
    );

    Ok(())
}

async fn resolve_bind_addr(host: &str, port: u16) -> anyhow::Result<SocketAddr> {
    // Fast path: numeric ip.
    if let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() {
        return Ok(addr);
    }
    // Special case localhost so we don't need DNS.
    if host == "localhost" {
        return Ok(SocketAddr::from(([127, 0, 0, 1], port)));
    }
    // Fallback to DNS.
    let mut iter = tokio::net::lookup_host((host, port))
        .await
        .with_context(|| format!("resolve {host}:{port}"))?;
    iter.next()
        .ok_or_else(|| anyhow::anyhow!("no address for {host}:{port}"))
}

fn cmd_down(args: DownArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let session_id = match args.session {
        Some(s) => s,
        None => SessionState::list(&cfg.workspace.state_dir)?
            .into_iter()
            .next()
            .map(|s| s.session_id)
            .ok_or_else(|| anyhow::anyhow!("no sessions on disk"))?,
    };
    let report = cleanup::run(
        &cfg.workspace.state_dir,
        &cfg.workspace.artifact_dir,
        &session_id,
        args.force,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.ok {
        std::process::exit(1);
    }
    Ok(())
}

async fn cmd_smoke(args: SmokeArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let base_url = args.base_url.unwrap_or_else(|| cfg.server.base_url());
    let mcp_path = args.mcp_path.unwrap_or(cfg.profile.mcp_path);
    let client_profile = args
        .client_profile
        .as_deref()
        .unwrap_or(&cfg.profile.oauth.client);
    let mut options = smoke::SmokeOptions::for_profile(client_profile);
    if let Some(client_id) = cfg.profile.oauth.client_id_metadata_document {
        options.client_id = client_id;
    }
    if !cfg.profile.oauth.scopes.is_empty() {
        options.scopes = cfg.profile.oauth.scopes;
    }
    if let Some(client_id) = args.client_id {
        options.client_id = client_id;
    }
    if let Some(redirect_uri) = args.redirect_uri {
        options.redirect_uri = redirect_uri;
    }
    if !args.scope.is_empty() {
        options.scopes = args.scope;
    }
    if let Some(resource) = args.resource {
        options.resource = match resource.as_str() {
            "auto" => smoke::ResourceParam::Auto,
            "omit" | "none" => smoke::ResourceParam::Omit,
            _ => smoke::ResourceParam::Value(resource),
        };
    }
    options.expected_upstream_client_id = args.expected_upstream_client_id;
    let report = smoke::run_with_options(&base_url, &mcp_path, &args.out, options).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.passed() {
        std::process::exit(1);
    }
    Ok(())
}

async fn cmd_dance(args: DanceArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let base_url = args.base_url.unwrap_or_else(|| cfg.server.base_url());
    let mcp_path = args.mcp_path.unwrap_or(cfg.profile.mcp_path.clone());
    let cimd =
        args.client_id_metadata_document
            .or(cfg.profile.oauth.client_id_metadata_document.clone());
    let cid = args.client_id.or(Some(cfg.profile.oauth.client.clone()));
    let report = client_dance::run(
        &base_url,
        &mcp_path,
        cid.as_deref(),
        cimd.as_deref(),
        &args.out,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.passed() {
        std::process::exit(1);
    }
    Ok(())
}

async fn cmd_oauth_code(args: OauthCodeArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let base_url = args.base_url.unwrap_or_else(|| cfg.server.base_url());
    let mcp_path = args.mcp_path.unwrap_or(cfg.profile.mcp_path.clone());

    let profile = args
        .client_profile
        .as_deref()
        .unwrap_or(&cfg.profile.oauth.client);
    // Reuse the same client-profile defaults that `smoke` uses, so an agent only
    // has to set one flag for both commands.
    let mut options = smoke::SmokeOptions::for_profile(profile);
    if let Some(client_id) = cfg.profile.oauth.client_id_metadata_document {
        options.client_id = client_id;
    }
    if !cfg.profile.oauth.scopes.is_empty() {
        options.scopes = cfg.profile.oauth.scopes;
    }
    if let Some(client_id) = args.client_id {
        options.client_id = client_id;
    }
    if !args.scope.is_empty() {
        options.scopes = args.scope;
    }
    let resource_value = match args.resource.as_deref() {
        Some("auto") => Some("auto".to_string()),
        Some("omit") | Some("none") => None,
        Some(other) => Some(other.to_string()),
        None => Some("auto".to_string()),
    };

    let opts = oauth_code::OAuthCodeOptions {
        base_url,
        mcp_path,
        client_id: options.client_id,
        redirect_uri: args.redirect_uri,
        callback_mode: args.callback_mode.parse()?,
        scopes: options.scopes,
        resource: resource_value,
        timeout: std::time::Duration::from_secs(args.timeout_secs),
        open_browser: args.open_browser,
    };
    let report = oauth_code::run(opts, &args.out).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.passed() {
        std::process::exit(1);
    }
    Ok(())
}

async fn cmd_doctor(args: DoctorArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let report = doctor::run(&cfg, &cfg.workspace.state_dir).await;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.passed() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_list(args: ListArgs) -> anyhow::Result<()> {
    let cfg = Config::load_or_default(args.config.as_deref())?;
    let sessions = SessionState::list(&cfg.workspace.state_dir)?;
    println!("{}", serde_json::to_string_pretty(&sessions)?);
    Ok(())
}
