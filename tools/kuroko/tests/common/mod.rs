//! Shared E2E test harness. Each integration-test file compiles this module
//! independently, so some helpers are unused per-file but used overall.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use kuroko::persistence::Snapshot;
use kuroko::registry::Registry;
use kuroko::server;
use kuroko::service::ServiceContext;
use kuroko::services;
use tokio::net::TcpListener;

pub struct TestServer {
    pub endpoint: String,
    pub data_dir: Option<std::path::PathBuf>,
    pub registry: Arc<Registry>,
    pub ctx: ServiceContext,
}

impl TestServer {
    /// Trigger every service's persist hook on the registry the spawned server
    /// is actually using. Tests that need to verify on-disk snapshots use this
    /// instead of waiting for graceful shutdown.
    pub fn snapshot_all(&self) {
        for svc in self.registry.all() {
            svc.snapshot(&self.ctx).ok();
        }
    }
}

pub async fn spawn() -> TestServer {
    spawn_inner(None).await
}

pub async fn spawn_with_data_dir(dir: std::path::PathBuf) -> TestServer {
    spawn_inner(Some(dir)).await
}

async fn spawn_inner(data_dir: Option<std::path::PathBuf>) -> TestServer {
    let snapshot = data_dir.as_ref().map(Snapshot::new);
    let ctx = ServiceContext::new(snapshot);
    let registry = Arc::new(Registry::new());
    services::register_all(&registry);

    for svc in registry.all() {
        svc.restore(&ctx).ok();
    }

    let app = server::build_router(registry.clone(), ctx.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    TestServer {
        endpoint: format!("http://{addr}"),
        data_dir,
        registry,
        ctx,
    }
}

pub fn creds() -> Credentials {
    Credentials::new("test", "test", None, None, "kuroko")
}

pub async fn aws_config(endpoint: &str) -> aws_config::SdkConfig {
    aws_config::defaults(BehaviorVersion::latest())
        .endpoint_url(endpoint)
        .region("us-east-1")
        .credentials_provider(creds())
        .load()
        .await
}

pub fn s3_client(cfg: &aws_config::SdkConfig) -> aws_sdk_s3::Client {
    aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::config::Builder::from(cfg)
            .force_path_style(true)
            .build(),
    )
}
