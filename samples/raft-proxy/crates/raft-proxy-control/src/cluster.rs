use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::{OriginalUri, State};
use openraft::RaftMetrics;
use raft_proxy_core::{Node, NodeId};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::ProxyApp;
use crate::error::ControlError;

#[derive(Debug, Deserialize)]
pub struct InitRequest {
    members: Option<Vec<MemberRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct MemberRequest {
    id: NodeId,
    rpc_addr: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeMembershipRequest {
    members: Vec<NodeId>,
    #[serde(default)]
    retain: bool,
}

pub async fn init(
    State(app): State<Arc<ProxyApp>>,
    body: Option<Json<InitRequest>>,
) -> Result<Json<Value>, ControlError> {
    let members = match body {
        Some(Json(req)) => members_from_request(&app, req.members)?,
        None => BTreeMap::from([(
            app.node_id,
            Node {
                rpc_addr: app.self_rpc_addr.clone(),
            },
        )]),
    };

    app.raft
        .initialize(members)
        .await
        .map_err(|err| ControlError::BadRequest(err.to_string()))?;

    Ok(Json(json!({ "status": "ok" })))
}

pub async fn add_learner(
    State(app): State<Arc<ProxyApp>>,
    OriginalUri(uri): OriginalUri,
    Json(req): Json<MemberRequest>,
) -> Result<Json<Value>, ControlError> {
    app.peers.insert(req.id, req.rpc_addr.clone());
    let response = app
        .raft
        .add_learner(
            req.id,
            Node {
                rpc_addr: req.rpc_addr,
            },
            true,
        )
        .await
        .map_err(|err| ControlError::from_client_write(err, uri.path()))?;

    Ok(Json(json!({
        "status": "ok",
        "log_id": response.log_id,
    })))
}

pub async fn change_membership(
    State(app): State<Arc<ProxyApp>>,
    OriginalUri(uri): OriginalUri,
    Json(req): Json<ChangeMembershipRequest>,
) -> Result<Json<Value>, ControlError> {
    let members = BTreeSet::from_iter(req.members);
    let response = app
        .raft
        .change_membership(members, req.retain)
        .await
        .map_err(|err| ControlError::from_client_write(err, uri.path()))?;

    Ok(Json(json!({
        "status": "ok",
        "log_id": response.log_id,
    })))
}

pub async fn metrics(State(app): State<Arc<ProxyApp>>) -> Json<RaftMetrics<NodeId, Node>> {
    Json(app.raft.metrics().borrow().clone())
}

fn members_from_request(
    app: &ProxyApp,
    members: Option<Vec<MemberRequest>>,
) -> Result<BTreeMap<NodeId, Node>, ControlError> {
    let members = members.unwrap_or_else(|| {
        vec![MemberRequest {
            id: app.node_id,
            rpc_addr: app.self_rpc_addr.clone(),
        }]
    });

    if members.is_empty() {
        return Err(ControlError::BadRequest(
            "members must not be empty".to_owned(),
        ));
    }

    Ok(members
        .into_iter()
        .map(|member| {
            app.peers.insert(member.id, member.rpc_addr.clone());
            (
                member.id,
                Node {
                    rpc_addr: member.rpc_addr,
                },
            )
        })
        .collect())
}
