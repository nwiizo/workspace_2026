//! A minimal MCP-shaped upstream server used to exercise remote-mcp-devkit end-to-end.
//!
//! It accepts any request on `/mcp` and replies with a JSON envelope that echoes the
//! body. remote-mcp-devkit handles auth/401 in front of it.
//!
//!     cargo run --example simple-upstream -- --port 18080
//!     remote-mcp-devkit up --upstream http://127.0.0.1:18080

use axum::{Json, Router, extract::Request, response::IntoResponse, routing::any};
use clap::Parser;
use serde_json::json;
use std::net::SocketAddr;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 18080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let app = Router::new()
        .route("/mcp", any(handle))
        .route("/mcp/{*rest}", any(handle))
        .route("/health", any(health));
    let addr: SocketAddr = format!("127.0.0.1:{}", args.port).parse()?;
    println!("simple-upstream listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(json!({"ok": true}))
}

async fn handle(req: Request) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let forwarded_proto = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let forwarded_host = req
        .headers()
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .ok()
        .and_then(|b| String::from_utf8(b.to_vec()).ok())
        .unwrap_or_default();
    Json(json!({
        "upstream": "simple-upstream",
        "method": method,
        "path": path,
        "headers": {
            "x-forwarded-proto": forwarded_proto,
            "x-forwarded-host": forwarded_host,
            "authorization_present": auth.is_some(),
        },
        "body": body,
    }))
}
