use std::sync::Arc;

use axum::Json;
use axum::extract::{OriginalUri, Path, State};
use raft_proxy_core::{AppRequest, AppResponse, RoutingTable, Upstream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::app::ProxyApp;
use crate::error::ControlError;

#[derive(Debug, Deserialize)]
pub struct PutRouteRequest {
    host: String,
    upstreams: Vec<Upstream>,
}

#[derive(Debug, Serialize)]
pub struct DeleteRouteResponse {
    found: bool,
}

pub async fn put(
    State(app): State<Arc<ProxyApp>>,
    OriginalUri(uri): OriginalUri,
    Json(req): Json<PutRouteRequest>,
) -> Result<Json<Value>, ControlError> {
    let response = app
        .raft
        .client_write(AppRequest::PutRoute {
            host: req.host,
            upstreams: req.upstreams,
        })
        .await
        .map_err(|err| ControlError::from_client_write(err, uri.path()))?;

    Ok(Json(json!({
        "status": "ok",
        "log_id": response.log_id,
    })))
}

pub async fn delete(
    State(app): State<Arc<ProxyApp>>,
    OriginalUri(uri): OriginalUri,
    Path(host): Path<String>,
) -> Result<Json<DeleteRouteResponse>, ControlError> {
    let response = app
        .raft
        .client_write(AppRequest::DeleteRoute { host })
        .await
        .map_err(|err| ControlError::from_client_write(err, uri.path()))?;

    let found = match response.data {
        AppResponse::Ok => true,
        AppResponse::NotFound => false,
    };

    Ok(Json(DeleteRouteResponse { found }))
}

pub async fn list(State(app): State<Arc<ProxyApp>>) -> Json<Arc<RoutingTable>> {
    Json(app.routing.load_full())
}
