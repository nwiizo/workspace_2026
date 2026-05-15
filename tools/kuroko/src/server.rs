//! HTTP server: builds the axum router from the registry, listens, and dispatches
//! to the protocol layer based on Content-Type.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use tokio::net::TcpListener;
use tokio::signal;

use crate::config::Config;
use crate::persistence::Snapshot;
use crate::protocol::{awsjson, cbor, query};
use crate::registry::Registry;
use crate::service::ServiceContext;
use crate::services;

pub type SharedState = (Arc<Registry>, ServiceContext);

pub async fn run(config: Config) -> anyhow::Result<()> {
    let snapshot = config.data_dir_path().map(Snapshot::new);
    let ctx = ServiceContext::new(snapshot);

    let registry = Arc::new(Registry::new());
    services::register_all(&registry);

    // Restore persisted state for every service before opening the socket.
    for svc in registry.all() {
        if let Err(err) = svc.restore(&ctx) {
            tracing::warn!(service = svc.name(), error = %err.message, "restore failed");
        }
    }

    let app = build_router(registry.clone(), ctx.clone());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(
        addr = %addr,
        services = registry.names().len(),
        data_dir = ?config.data_dir,
        "kuroko listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Persist on graceful shutdown so a restart picks up where we left off.
    if ctx.snapshot.is_some() {
        for svc in registry.all() {
            if let Err(err) = svc.snapshot(&ctx) {
                tracing::warn!(service = svc.name(), error = %err.message, "snapshot failed");
            }
        }
    }
    Ok(())
}

pub fn build_router(registry: Arc<Registry>, ctx: ServiceContext) -> Router {
    let state: SharedState = (registry.clone(), ctx.clone());

    // Routes that need shared state are built first, then state-ified into a
    // `Router<()>`, then merged with each service's own router. This avoids
    // the "mixing state types" error from axum when adding state-bearing
    // handlers after a stateless merge.
    let stateful = Router::new()
        .route("/", post(unified_dispatcher))
        .route("/_kuroko/info", get(info))
        .route("/_kuroko/health", get(health))
        .route("/_kuroko/services", get(list_services))
        .route("/_kuroko/reset", post(reset_all))
        .route(
            "/service/{service}/operation/{operation}",
            post(cbor::handler),
        )
        // 32 MiB cap on the unified dispatcher (code #13). DynamoDB and SQS
        // payloads top out well below this; S3 has its own larger limit on
        // the REST router.
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .with_state(state);

    let mut app = stateful;
    for svc in registry.all() {
        app = app.merge(svc.router(ctx.clone()));
    }
    app
}

async fn unified_dispatcher(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let media = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let (registry, ctx) = state;

    match media.as_str() {
        "application/x-www-form-urlencoded" => query::dispatch(registry, ctx, headers, body).await,
        _ => awsjson::dispatch(registry, ctx, headers, body).await,
    }
}

async fn info() -> Response {
    let body = serde_json::json!({
        "name": "kuroko",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

async fn reset_all(State((registry, ctx)): State<SharedState>) -> Response {
    let services = registry.all();
    for svc in &services {
        svc.reset();
    }
    // If persistence is enabled, write empty snapshots so a restart matches.
    if ctx.snapshot.is_some() {
        for svc in &services {
            let _ = svc.snapshot(&ctx);
        }
    }
    let body = serde_json::json!({ "reset": services.len() });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

async fn health() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap()
}

async fn list_services(State((registry, _)): State<SharedState>) -> Response {
    let names = registry.names();
    let body = serde_json::to_vec(&serde_json::json!({ "services": names, "count": names.len() }))
        .unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
