//! Amazon Data Lifecycle Manager — restJson1.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{Service, ServiceContext, persistence_error};
use crate::services::rest_helpers::{rest_error, rest_json};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    policies: HashMap<String, Value>,
}

pub struct Dlm {
    state: Arc<RwLock<State_>>,
}
impl Dlm {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Dlm {
    fn default() -> Self {
        Self::new()
    }
}
type DState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Dlm {
    fn name(&self) -> &'static str {
        "dlm"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(d) = snap.load::<State_>("dlm").map_err(persistence_error)?
        {
            *self.state.write() = d;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("dlm", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route("/policies", post(create_policy).get(list_policies))
            .route("/policies/{id}", get(get_policy).delete(delete_policy))
            .with_state(s)
    }
}

async fn create_policy(State(state): State<DState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let id = format!("policy-{}", &Uuid::new_v4().simple().to_string()[..17]);
    let mut p = req;
    if let Some(o) = p.as_object_mut() {
        o.insert("PolicyId".into(), json!(id.clone()));
        o.insert("State".into(), json!("ENABLED"));
    }
    state.write().policies.insert(id.clone(), p);
    rest_json(StatusCode::OK, &json!({ "PolicyId": id }))
}

async fn list_policies(State(state): State<DState>) -> Response {
    let s = state.read();
    let summaries: Vec<Value> = s
        .policies
        .iter()
        .map(|(id, p)| {
            json!({
                "PolicyId": id,
                "Description": p.get("Description").cloned().unwrap_or(Value::Null),
                "State": "ENABLED",
                "PolicyType": p.get("PolicyType").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "Policies": summaries }))
}

async fn get_policy(State(state): State<DState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.policies.get(&id) {
        Some(p) => rest_json(
            StatusCode::OK,
            &json!({
                "Policy": {
                    "PolicyId": id,
                    "Description": p.get("Description").cloned().unwrap_or(Value::Null),
                    "State": "ENABLED",
                    "PolicyDetails": p.get("PolicyDetails").cloned().unwrap_or(Value::Null),
                    "ExecutionRoleArn": p.get("ExecutionRoleArn").cloned().unwrap_or(Value::Null),
                }
            }),
        ),
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn delete_policy(State(state): State<DState>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    if s.policies.remove(&id).is_some() {
        rest_json(StatusCode::OK, &json!({}))
    } else {
        rest_error(AwsError::new("ResourceNotFoundException", "not found"))
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Dlm::new()));
}
