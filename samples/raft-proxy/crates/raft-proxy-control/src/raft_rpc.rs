use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use raft_proxy_core::{NodeId, TypeConfig};

use crate::app::ProxyApp;
use crate::error::ControlError;

pub async fn vote(
    State(app): State<Arc<ProxyApp>>,
    Json(req): Json<VoteRequest<NodeId>>,
) -> Result<Json<VoteResponse<NodeId>>, ControlError> {
    Ok(Json(app.raft.vote(req).await?))
}

pub async fn append(
    State(app): State<Arc<ProxyApp>>,
    Json(req): Json<AppendEntriesRequest<TypeConfig>>,
) -> Result<Json<AppendEntriesResponse<NodeId>>, ControlError> {
    Ok(Json(app.raft.append_entries(req).await?))
}

pub async fn snapshot(
    State(app): State<Arc<ProxyApp>>,
    Json(req): Json<InstallSnapshotRequest<TypeConfig>>,
) -> Result<Json<InstallSnapshotResponse<NodeId>>, ControlError> {
    app.raft
        .install_snapshot(req)
        .await
        .map(Json)
        .map_err(ControlError::from_raft_error)
}
