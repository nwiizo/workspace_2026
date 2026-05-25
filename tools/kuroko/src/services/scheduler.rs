//! EventBridge Scheduler — REST protocol under `/schedules/*` and `/schedule-groups/*`.

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

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, Service, ServiceContext, persistence_error,
};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    schedules: HashMap<String, Schedule>,
    groups: HashMap<String, ScheduleGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Schedule {
    name: String,
    arn: String,
    group_name: String,
    schedule_expression: String,
    target: Value,
    state: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ScheduleGroup {
    name: String,
    arn: String,
    state: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Scheduler {
    state: Arc<RwLock<State_>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut s = State_::default();
        s.groups.insert(
            "default".into(),
            ScheduleGroup {
                name: "default".into(),
                arn: group_arn("default"),
                state: "ACTIVE".into(),
                created: chrono::Utc::now(),
            },
        );
        Self {
            state: Arc::new(RwLock::new(s)),
        }
    }
}
impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

type SchedState = Arc<RwLock<State_>>;

fn schedule_arn(group: &str, name: &str) -> String {
    format!("arn:aws:scheduler:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:schedule/{group}/{name}")
}

fn group_arn(name: &str) -> String {
    format!("arn:aws:scheduler:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:schedule-group/{name}")
}

#[async_trait]
impl Service for Scheduler {
    fn name(&self) -> &'static str {
        "scheduler"
    }
    fn reset(&self) {
        let mut s = self.state.write();
        s.schedules.clear();
        s.groups.clear();
        s.groups.insert(
            "default".into(),
            ScheduleGroup {
                name: "default".into(),
                arn: group_arn("default"),
                state: "ACTIVE".into(),
                created: chrono::Utc::now(),
            },
        );
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State_>("scheduler")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("scheduler", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route("/schedules", get(list_schedules))
            .route(
                "/schedules/{name}",
                post(create_schedule)
                    .get(get_schedule)
                    .delete(delete_schedule),
            )
            .route("/schedule-groups", get(list_groups))
            .route(
                "/schedule-groups/{name}",
                post(create_group).get(get_group).delete(delete_group),
            )
            .with_state(state)
    }
}

async fn create_schedule(
    State(state): State<SchedState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let group_name = req
        .get("GroupName")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    let expr = match req.get("ScheduleExpression").and_then(Value::as_str) {
        Some(e) => e.to_string(),
        None => {
            return rest_err(
                StatusCode::BAD_REQUEST,
                "ValidationException",
                "ScheduleExpression required",
            );
        }
    };
    let mut s = state.write();
    let key = format!("{group_name}/{name}");
    if s.schedules.contains_key(&key) {
        return rest_err(
            StatusCode::CONFLICT,
            "ConflictException",
            "schedule already exists",
        );
    }
    let arn = schedule_arn(&group_name, &name);
    s.schedules.insert(
        key,
        Schedule {
            name: name.clone(),
            arn: arn.clone(),
            group_name,
            schedule_expression: expr,
            target: req.get("Target").cloned().unwrap_or(Value::Null),
            state: req
                .get("State")
                .and_then(Value::as_str)
                .unwrap_or("ENABLED")
                .to_string(),
            created: chrono::Utc::now(),
        },
    );
    rest_json(StatusCode::OK, &json!({ "ScheduleArn": arn }))
}

async fn get_schedule(State(state): State<SchedState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    let sched = s.schedules.values().find(|sc| sc.name == name);
    match sched {
        Some(sc) => rest_json(
            StatusCode::OK,
            &json!({
                "Name": sc.name,
                "Arn": sc.arn,
                "GroupName": sc.group_name,
                "ScheduleExpression": sc.schedule_expression,
                "State": sc.state,
                "Target": sc.target,
                "CreationDate": sc.created.timestamp(),
            }),
        ),
        None => rest_err(
            StatusCode::NOT_FOUND,
            "ResourceNotFoundException",
            "schedule not found",
        ),
    }
}

async fn delete_schedule(State(state): State<SchedState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    let key = s
        .schedules
        .iter()
        .find(|(_, sc)| sc.name == name)
        .map(|(k, _)| k.clone());
    if let Some(k) = key {
        s.schedules.remove(&k);
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("{}"))
            .unwrap()
    } else {
        rest_err(
            StatusCode::NOT_FOUND,
            "ResourceNotFoundException",
            "schedule not found",
        )
    }
}

async fn list_schedules(State(state): State<SchedState>) -> Response {
    let s = state.read();
    let schedules: Vec<_> = s
        .schedules
        .values()
        .map(|sc| {
            json!({
                "Name": sc.name,
                "Arn": sc.arn,
                "GroupName": sc.group_name,
                "State": sc.state,
                "CreationDate": sc.created.timestamp(),
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "Schedules": schedules }))
}

async fn create_group(
    State(state): State<SchedState>,
    Path(name): Path<String>,
    _body: Bytes,
) -> Response {
    let mut s = state.write();
    if s.groups.contains_key(&name) {
        return rest_err(StatusCode::CONFLICT, "ConflictException", "group exists");
    }
    let arn = group_arn(&name);
    s.groups.insert(
        name.clone(),
        ScheduleGroup {
            name,
            arn: arn.clone(),
            state: "ACTIVE".into(),
            created: chrono::Utc::now(),
        },
    );
    rest_json(StatusCode::OK, &json!({ "ScheduleGroupArn": arn }))
}

async fn get_group(State(state): State<SchedState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    match s.groups.get(&name) {
        Some(g) => rest_json(
            StatusCode::OK,
            &json!({
                "Name": g.name,
                "Arn": g.arn,
                "State": g.state,
                "CreationDate": g.created.timestamp(),
            }),
        ),
        None => rest_err(
            StatusCode::NOT_FOUND,
            "ResourceNotFoundException",
            "group not found",
        ),
    }
}

async fn delete_group(State(state): State<SchedState>, Path(name): Path<String>) -> Response {
    if name == "default" {
        return rest_err(
            StatusCode::BAD_REQUEST,
            "ValidationException",
            "cannot delete default group",
        );
    }
    state.write().groups.remove(&name);
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("{}"))
        .unwrap()
}

async fn list_groups(State(state): State<SchedState>) -> Response {
    let s = state.read();
    let groups: Vec<_> = s
        .groups
        .values()
        .map(|g| {
            json!({
                "Name": g.name,
                "Arn": g.arn,
                "State": g.state,
                "CreationDate": g.created.timestamp(),
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "ScheduleGroups": groups }))
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_err(status: StatusCode, code: &str, msg: &str) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", code)
        .body(Body::from(
            serde_json::to_vec(&json!({ "Type": code, "message": msg })).unwrap_or_default(),
        ))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Scheduler::new()));
}
