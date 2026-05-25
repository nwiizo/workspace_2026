//! AWS Amplify — restJson1 under `/apps`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, Service, ServiceContext, persistence_error,
};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    apps: HashMap<String, App>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct App {
    id: String,
    arn: String,
    name: String,
    description: String,
    platform: String,
    repository: String,
}

pub struct Amplify {
    state: Arc<RwLock<State_>>,
}
impl Amplify {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Amplify {
    fn default() -> Self {
        Self::new()
    }
}

type AState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Amplify {
    fn name(&self) -> &'static str {
        "amplify"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("amplify").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("amplify", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route("/apps", post(create_app).get(list_apps))
            .route("/apps/{id}", get(get_app).delete(delete_app))
            .with_state(s)
    }
}

async fn create_app(State(state): State<AState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let name = match req.get("name").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_error(AwsError::new("BadRequestException", "name required")),
    };
    let id = format!("d{}", Uuid::new_v4().simple());
    let arn = format!("arn:aws:amplify:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:apps/{id}");
    let app = App {
        id: id.clone(),
        arn,
        name,
        description: req
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        platform: req
            .get("platform")
            .and_then(Value::as_str)
            .unwrap_or("WEB")
            .to_string(),
        repository: req
            .get("repository")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    };
    let body = app_json(&app);
    state.write().apps.insert(id, app);
    rest_json(StatusCode::OK, &json!({ "app": body }))
}

async fn list_apps(State(state): State<AState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s.apps.values().map(app_json).collect();
    rest_json(StatusCode::OK, &json!({ "apps": items }))
}

async fn get_app(State(state): State<AState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.apps.get(&id) {
        Some(a) => rest_json(StatusCode::OK, &json!({ "app": app_json(a) })),
        None => rest_error(AwsError::new("NotFoundException", "app not found")),
    }
}

async fn delete_app(State(state): State<AState>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    match s.apps.remove(&id) {
        Some(a) => rest_json(StatusCode::OK, &json!({ "app": app_json(&a) })),
        None => rest_error(AwsError::new("NotFoundException", "app not found")),
    }
}

fn app_json(a: &App) -> Value {
    json!({
        "appId": a.id,
        "appArn": a.arn,
        "name": a.name,
        "description": a.description,
        "platform": a.platform,
        "repository": a.repository,
        "createTime": chrono::Utc::now().timestamp(),
        "updateTime": chrono::Utc::now().timestamp(),
        "defaultDomain": format!("{}.amplifyapp.kuroko.test", a.id),
        "enableBranchAutoBuild": false,
        "enableBasicAuth": false,
        "environmentVariables": {},
    })
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_error(err: AwsError) -> Response {
    let body = json!({ "Message": err.message });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Amplify::new()));
}
