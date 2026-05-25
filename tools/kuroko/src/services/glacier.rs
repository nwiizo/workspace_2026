//! Glacier — REST under `/-/vaults/*`. Glacier APIs are scoped under
//! `/{account_id}/vaults/*`; AWS SDK uses `-` as a shorthand for the
//! current account.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, put};
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, Service, ServiceContext, persistence_error,
};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    vaults: HashMap<String, Vault>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Vault {
    name: String,
    arn: String,
    created: chrono::DateTime<chrono::Utc>,
    size_in_bytes: i64,
    number_of_archives: i64,
}

pub struct Glacier {
    state: Arc<RwLock<State_>>,
}
impl Glacier {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Glacier {
    fn default() -> Self {
        Self::new()
    }
}

type GlacierState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Glacier {
    fn name(&self) -> &'static str {
        "glacier"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("glacier").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("glacier", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route(
                "/{account_id}/vaults/{name}",
                put(create_vault).get(describe_vault).delete(delete_vault),
            )
            .route("/{account_id}/vaults", get(list_vaults))
            .with_state(state)
    }
}

async fn create_vault(
    State(state): State<GlacierState>,
    Path((_acct, name)): Path<(String, String)>,
) -> Response {
    let mut s = state.write();
    if s.vaults.contains_key(&name) {
        return rest_error(
            StatusCode::CONFLICT,
            "ResourceInUseException",
            "vault exists",
        );
    }
    let arn = format!("arn:aws:glacier:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:vaults/{name}");
    s.vaults.insert(
        name.clone(),
        Vault {
            name,
            arn: arn.clone(),
            created: chrono::Utc::now(),
            size_in_bytes: 0,
            number_of_archives: 0,
        },
    );
    Response::builder()
        .status(StatusCode::CREATED)
        .header(
            "Location",
            format!("/-/vaults/{}", arn.rsplit('/').next().unwrap_or("")),
        )
        .body(Body::empty())
        .unwrap()
}

async fn describe_vault(
    State(state): State<GlacierState>,
    Path((_acct, name)): Path<(String, String)>,
) -> Response {
    let s = state.read();
    match s.vaults.get(&name) {
        Some(v) => rest_json(
            StatusCode::OK,
            &json!({
                "VaultName": v.name,
                "VaultARN": v.arn,
                "CreationDate": v.created.to_rfc3339(),
                "SizeInBytes": v.size_in_bytes,
                "NumberOfArchives": v.number_of_archives,
            }),
        ),
        None => rest_error(
            StatusCode::NOT_FOUND,
            "ResourceNotFoundException",
            "vault not found",
        ),
    }
}

async fn delete_vault(
    State(state): State<GlacierState>,
    Path((_acct, name)): Path<(String, String)>,
) -> Response {
    state.write().vaults.remove(&name);
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn list_vaults(State(state): State<GlacierState>, Path(_acct): Path<String>) -> Response {
    let s = state.read();
    let vaults: Vec<_> = s
        .vaults
        .values()
        .map(|v| {
            json!({
                "VaultName": v.name,
                "VaultARN": v.arn,
                "CreationDate": v.created.to_rfc3339(),
                "SizeInBytes": v.size_in_bytes,
                "NumberOfArchives": v.number_of_archives,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "VaultList": vaults }))
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_error(status: StatusCode, code: &str, msg: &str) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", code)
        .body(Body::from(
            serde_json::to_vec(&json!({ "code": code, "message": msg, "type": "Client" }))
                .unwrap_or_default(),
        ))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Glacier::new()));
}
