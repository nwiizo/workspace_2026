use axum::Json;
use axum::http::header::LOCATION;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use openraft::error::{ClientWriteError, ForwardToLeader, RaftError};
use raft_proxy_core::{Node, NodeId};
use raft_proxy_network::normalize_base_url;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("raft init: {0}")]
    RaftBuild(String),

    #[error("raft op: {0}")]
    RaftOp(String),

    #[error("forward to leader at {0}")]
    ForwardToLeader(String),

    #[error("no leader yet")]
    NoLeader,

    #[error("bad request: {0}")]
    BadRequest(String),
}

impl ControlError {
    pub(crate) fn from_client_write(
        err: RaftError<NodeId, ClientWriteError<NodeId, Node>>,
        path: &str,
    ) -> Self {
        match err {
            RaftError::APIError(ClientWriteError::ForwardToLeader(forward)) => {
                redirect_from_forward(forward, path)
            }
            other => Self::RaftOp(other.to_string()),
        }
    }

    pub(crate) fn from_raft_error<E>(err: RaftError<NodeId, E>) -> Self
    where
        E: std::error::Error,
    {
        Self::RaftOp(err.to_string())
    }
}

impl IntoResponse for ControlError {
    fn into_response(self) -> Response {
        match self {
            Self::ForwardToLeader(location) => match HeaderValue::from_str(&location) {
                Ok(value) => {
                    let mut response = StatusCode::TEMPORARY_REDIRECT.into_response();
                    response.headers_mut().insert(LOCATION, value);
                    response
                }
                Err(err) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("invalid redirect location: {err}"),
                ),
            },
            Self::NoLeader => error_response(StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            Self::BadRequest(_) => error_response(StatusCode::BAD_REQUEST, self.to_string()),
            Self::RaftBuild(_) | Self::RaftOp(_) => {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        }
    }
}

impl From<openraft::error::Fatal<NodeId>> for ControlError {
    fn from(err: openraft::error::Fatal<NodeId>) -> Self {
        Self::RaftOp(err.to_string())
    }
}

impl From<RaftError<NodeId>> for ControlError {
    fn from(err: RaftError<NodeId>) -> Self {
        Self::from_raft_error(err)
    }
}

fn redirect_from_forward(forward: ForwardToLeader<NodeId, Node>, path: &str) -> ControlError {
    match (forward.leader_id, forward.leader_node) {
        (Some(_leader_id), Some(node)) if !node.rpc_addr.trim().is_empty() => {
            ControlError::ForwardToLeader(format!("{}{}", normalize_base_url(&node.rpc_addr), path))
        }
        _ => ControlError::NoLeader,
    }
}

fn error_response(status: StatusCode, message: String) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}
