//! AWS Backup — REST protocol.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, put};
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
    vaults: HashMap<String, Vault>,
    plans: HashMap<String, Plan>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Vault {
    name: String,
    arn: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Plan {
    id: String,
    name: String,
    arn: String,
    version: String,
    created: chrono::DateTime<chrono::Utc>,
    plan: Value,
}

pub struct Backup {
    state: Arc<RwLock<State_>>,
}

impl Backup {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Backup {
    fn default() -> Self {
        Self::new()
    }
}

type BackupState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Backup {
    fn name(&self) -> &'static str {
        "backup"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("backup").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("backup", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route(
                "/backup-vaults/{name}",
                put(create_vault).get(describe_vault).delete(delete_vault),
            )
            .route("/backup-vaults", get(list_vaults))
            .route("/backup/plans", get(list_plans).put(create_plan))
            .route("/backup/plans/{id}", get(get_plan).delete(delete_plan))
            .with_state(state)
    }
}

async fn create_vault(
    State(state): State<BackupState>,
    Path(name): Path<String>,
    _body: Bytes,
) -> Response {
    let mut s = state.write();
    if s.vaults.contains_key(&name) {
        return rest_error(AwsError::new(
            "AlreadyExistsException",
            format!("vault '{name}' already exists"),
        ));
    }
    let arn = format!("arn:aws:backup:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:backup-vault:{name}");
    let v = Vault {
        name: name.clone(),
        arn: arn.clone(),
        created: chrono::Utc::now(),
    };
    s.vaults.insert(name.clone(), v);
    rest_json(
        StatusCode::OK,
        &json!({
            "BackupVaultName": name,
            "BackupVaultArn": arn,
            "CreationDate": chrono::Utc::now().timestamp(),
        }),
    )
}

async fn describe_vault(State(state): State<BackupState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    match s.vaults.get(&name) {
        Some(v) => rest_json(
            StatusCode::OK,
            &json!({
                "BackupVaultName": v.name,
                "BackupVaultArn": v.arn,
                "CreationDate": v.created.timestamp(),
                "NumberOfRecoveryPoints": 0,
            }),
        ),
        None => rest_error(not_found(&name)),
    }
}

async fn delete_vault(State(state): State<BackupState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    if s.vaults.remove(&name).is_none() {
        return rest_error(not_found(&name));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn list_vaults(State(state): State<BackupState>) -> Response {
    let s = state.read();
    let vaults: Vec<_> = s
        .vaults
        .values()
        .map(|v| {
            json!({
                "BackupVaultName": v.name,
                "BackupVaultArn": v.arn,
                "CreationDate": v.created.timestamp(),
                "NumberOfRecoveryPoints": 0,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "BackupVaultList": vaults }))
}

async fn create_plan(State(state): State<BackupState>, body: Bytes) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let plan_body = req.get("BackupPlan").cloned().unwrap_or(Value::Null);
    let plan_name = plan_body
        .get("BackupPlanName")
        .and_then(Value::as_str)
        .unwrap_or("kuroko-plan")
        .to_string();
    let id = Uuid::new_v4().to_string();
    let arn = format!("arn:aws:backup:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:backup-plan:{id}");
    let version = Uuid::new_v4().simple().to_string();
    let plan = Plan {
        id: id.clone(),
        name: plan_name,
        arn: arn.clone(),
        version: version.clone(),
        created: chrono::Utc::now(),
        plan: plan_body,
    };
    state.write().plans.insert(id.clone(), plan);
    rest_json(
        StatusCode::OK,
        &json!({
            "BackupPlanId": id,
            "BackupPlanArn": arn,
            "CreationDate": chrono::Utc::now().timestamp(),
            "VersionId": version,
        }),
    )
}

async fn get_plan(State(state): State<BackupState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.plans.get(&id) {
        Some(p) => rest_json(
            StatusCode::OK,
            &json!({
                "BackupPlan": p.plan,
                "BackupPlanId": p.id,
                "BackupPlanArn": p.arn,
                "VersionId": p.version,
                "CreationDate": p.created.timestamp(),
            }),
        ),
        None => rest_error(not_found(&id)),
    }
}

async fn delete_plan(State(state): State<BackupState>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    if s.plans.remove(&id).is_none() {
        return rest_error(not_found(&id));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn list_plans(State(state): State<BackupState>) -> Response {
    let s = state.read();
    let plans: Vec<_> = s
        .plans
        .values()
        .map(|p| {
            json!({
                "BackupPlanId": p.id,
                "BackupPlanArn": p.arn,
                "BackupPlanName": p.name,
                "CreationDate": p.created.timestamp(),
                "VersionId": p.version,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "BackupPlansList": plans }))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("resource '{name}' not found"),
    )
    .status(StatusCode::NOT_FOUND)
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_error(err: AwsError) -> Response {
    let body = json!({ "Type": err.code, "message": err.message });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Backup::new()));
}
