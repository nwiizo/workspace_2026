//! Amazon EBS direct APIs — restJson1.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post, put};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{Service, ServiceContext, persistence_error};
use crate::services::rest_helpers::{rest_error, rest_json};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    snapshots: HashMap<String, Snapshot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Snapshot {
    id: String,
    parent_id: Option<String>,
    volume_size: i64,
    blocks: HashMap<i64, String>,
    status: String,
}

pub struct Ebs {
    state: Arc<RwLock<State_>>,
}
impl Ebs {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Ebs {
    fn default() -> Self {
        Self::new()
    }
}
type EState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Ebs {
    fn name(&self) -> &'static str {
        "ebs"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(d) = snap.load::<State_>("ebs").map_err(persistence_error)?
        {
            *self.state.write() = d;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("ebs", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route("/snapshots", post(start_snapshot))
            .route("/snapshots/completion/{id}", post(complete_snapshot))
            .route("/snapshots/{id}/blocks", get(list_snapshot_blocks))
            .route(
                "/snapshots/{id}/blocks/{block_index}",
                put(put_snapshot_block).get(get_snapshot_block),
            )
            .with_state(s)
    }
}

async fn start_snapshot(State(state): State<EState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let volume_size = req.get("VolumeSize").and_then(Value::as_i64).unwrap_or(1);
    let parent_id = req
        .get("ParentSnapshotId")
        .and_then(Value::as_str)
        .map(String::from);
    let id = format!("snap-{}", &Uuid::new_v4().simple().to_string()[..17]);
    let snap = Snapshot {
        id: id.clone(),
        parent_id,
        volume_size,
        blocks: HashMap::new(),
        status: "pending".into(),
    };
    state.write().snapshots.insert(id.clone(), snap);
    rest_json(
        StatusCode::ACCEPTED,
        &json!({
            "SnapshotId": id,
            "VolumeSize": volume_size,
            "BlockSize": 524288,
            "Status": "pending",
        }),
    )
}

async fn complete_snapshot(State(state): State<EState>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    match s.snapshots.get_mut(&id) {
        Some(snap) => {
            snap.status = "completed".into();
            rest_json(StatusCode::ACCEPTED, &json!({ "Status": "completed" }))
        }
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn put_snapshot_block(
    State(state): State<EState>,
    Path((id, block_index)): Path<(String, i64)>,
    body: Bytes,
) -> Response {
    use base64::Engine;
    let mut s = state.write();
    match s.snapshots.get_mut(&id) {
        Some(snap) => {
            snap.blocks.insert(
                block_index,
                base64::engine::general_purpose::STANDARD.encode(body),
            );
            rest_json(
                StatusCode::CREATED,
                &json!({ "Checksum": "x", "ChecksumAlgorithm": "SHA256" }),
            )
        }
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn get_snapshot_block(
    State(state): State<EState>,
    Path((id, block_index)): Path<(String, i64)>,
) -> Response {
    use base64::Engine;
    let s = state.read();
    match s
        .snapshots
        .get(&id)
        .and_then(|s| s.blocks.get(&block_index))
    {
        Some(b64) => {
            let data = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .unwrap_or_default();
            Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
                .body(axum::body::Body::from(data))
                .unwrap()
        }
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn list_snapshot_blocks(State(state): State<EState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.snapshots.get(&id) {
        Some(snap) => {
            let blocks: Vec<Value> = snap
                .blocks
                .keys()
                .map(|idx| json!({ "BlockIndex": idx, "BlockToken": format!("tok-{idx}") }))
                .collect();
            rest_json(
                StatusCode::OK,
                &json!({
                    "Blocks": blocks,
                    "ExpiryTime": chrono::Utc::now().to_rfc3339(),
                    "VolumeSize": snap.volume_size,
                    "BlockSize": 524288,
                }),
            )
        }
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Ebs::new()));
}
