//! AWS Batch — REST protocol under `/v1/*`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::post;
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
    job_queues: HashMap<String, JobQueue>,
    job_definitions: HashMap<String, JobDefinition>,
    jobs: HashMap<String, Job>,
    compute_environments: HashMap<String, ComputeEnvironment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JobQueue {
    name: String,
    arn: String,
    state: String,
    priority: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JobDefinition {
    name: String,
    revision: i32,
    arn: String,
    type_: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Job {
    id: String,
    name: String,
    queue: String,
    definition: String,
    status: String,
    created: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ComputeEnvironment {
    name: String,
    arn: String,
    type_: String,
    state: String,
    status: String,
}

pub struct Batch {
    state: Arc<RwLock<State_>>,
}

impl Batch {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

type BatchState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Batch {
    fn name(&self) -> &'static str {
        "batch"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("batch").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("batch", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route("/v1/createjobqueue", post(create_job_queue))
            .route("/v1/describejobqueues", post(describe_job_queues))
            .route("/v1/deletejobqueue", post(delete_job_queue))
            .route("/v1/registerjobdefinition", post(register_job_definition))
            .route("/v1/describejobdefinitions", post(describe_job_definitions))
            .route(
                "/v1/deregisterjobdefinition",
                post(deregister_job_definition),
            )
            .route("/v1/submitjob", post(submit_job))
            .route("/v1/describejobs", post(describe_jobs))
            .route("/v1/listjobs", post(list_jobs))
            .route("/v1/terminatejob", post(terminate_job))
            .route(
                "/v1/createcomputeenvironment",
                post(create_compute_environment),
            )
            .route(
                "/v1/describecomputeenvironments",
                post(describe_compute_environments),
            )
            .with_state(state)
    }
}

async fn create_job_queue(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let name = match req.get("jobQueueName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_err("jobQueueName required"),
    };
    let mut s = state.write();
    if s.job_queues.contains_key(&name) {
        return rest_err("jobQueueName already exists");
    }
    let arn = format!("arn:aws:batch:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:job-queue/{name}");
    s.job_queues.insert(
        name.clone(),
        JobQueue {
            name: name.clone(),
            arn: arn.clone(),
            state: req
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("ENABLED")
                .to_string(),
            priority: req.get("priority").and_then(Value::as_i64).unwrap_or(1) as i32,
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({ "jobQueueName": name, "jobQueueArn": arn }),
    )
}

async fn describe_job_queues(State(state): State<BatchState>, _body: Bytes) -> Response {
    let s = state.read();
    let queues: Vec<_> = s
        .job_queues
        .values()
        .map(|q| {
            json!({
                "jobQueueName": q.name,
                "jobQueueArn": q.arn,
                "state": q.state,
                "status": "VALID",
                "priority": q.priority,
                "computeEnvironmentOrder": [],
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "jobQueues": queues }))
}

async fn delete_job_queue(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    if let Some(name) = req.get("jobQueue").and_then(Value::as_str) {
        state.write().job_queues.remove(name);
    }
    rest_json(StatusCode::OK, &json!({}))
}

async fn register_job_definition(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let name = match req.get("jobDefinitionName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_err("jobDefinitionName required"),
    };
    let type_ = req
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("container")
        .to_string();
    let mut s = state.write();
    let revision = s
        .job_definitions
        .get(&name)
        .map(|d| d.revision + 1)
        .unwrap_or(1);
    let arn = format!(
        "arn:aws:batch:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:job-definition/{name}:{revision}"
    );
    s.job_definitions.insert(
        name.clone(),
        JobDefinition {
            name: name.clone(),
            revision,
            arn: arn.clone(),
            type_,
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({
            "jobDefinitionName": name,
            "jobDefinitionArn": arn,
            "revision": revision,
        }),
    )
}

async fn describe_job_definitions(State(state): State<BatchState>, _body: Bytes) -> Response {
    let s = state.read();
    let defs: Vec<_> = s
        .job_definitions
        .values()
        .map(|d| {
            json!({
                "jobDefinitionName": d.name,
                "jobDefinitionArn": d.arn,
                "revision": d.revision,
                "type": d.type_,
                "status": "ACTIVE",
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "jobDefinitions": defs }))
}

async fn deregister_job_definition(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    if let Some(arn) = req.get("jobDefinition").and_then(Value::as_str) {
        let mut s = state.write();
        let key = arn
            .rsplit('/')
            .next()
            .and_then(|p| p.split(':').next())
            .unwrap_or(arn)
            .to_string();
        s.job_definitions.remove(&key);
    }
    rest_json(StatusCode::OK, &json!({}))
}

async fn submit_job(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let id = Uuid::new_v4().to_string();
    let name = req
        .get("jobName")
        .and_then(Value::as_str)
        .unwrap_or("kuroko-job")
        .to_string();
    let job = Job {
        id: id.clone(),
        name: name.clone(),
        queue: req
            .get("jobQueue")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        definition: req
            .get("jobDefinition")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        status: "SUBMITTED".into(),
        created: chrono::Utc::now().timestamp_millis(),
    };
    state.write().jobs.insert(id.clone(), job);
    rest_json(StatusCode::OK, &json!({ "jobId": id, "jobName": name }))
}

async fn describe_jobs(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let want: Vec<String> = req
        .get("jobs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let s = state.read();
    let jobs: Vec<_> = want
        .iter()
        .filter_map(|id| s.jobs.get(id))
        .map(|j| {
            json!({
                "jobId": j.id,
                "jobName": j.name,
                "jobQueue": j.queue,
                "jobDefinition": j.definition,
                "status": j.status,
                "createdAt": j.created,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "jobs": jobs }))
}

async fn list_jobs(State(state): State<BatchState>, _body: Bytes) -> Response {
    let s = state.read();
    let summaries: Vec<_> = s
        .jobs
        .values()
        .map(|j| {
            json!({
                "jobId": j.id,
                "jobName": j.name,
                "status": j.status,
                "createdAt": j.created,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "jobSummaryList": summaries }))
}

async fn terminate_job(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    if let Some(id) = req.get("jobId").and_then(Value::as_str)
        && let Some(j) = state.write().jobs.get_mut(id)
    {
        j.status = "FAILED".into();
    }
    rest_json(StatusCode::OK, &json!({}))
}

async fn create_compute_environment(State(state): State<BatchState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let name = match req.get("computeEnvironmentName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_err("computeEnvironmentName required"),
    };
    let arn =
        format!("arn:aws:batch:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:compute-environment/{name}");
    state.write().compute_environments.insert(
        name.clone(),
        ComputeEnvironment {
            name: name.clone(),
            arn: arn.clone(),
            type_: req
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("MANAGED")
                .to_string(),
            state: "ENABLED".into(),
            status: "VALID".into(),
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({
            "computeEnvironmentName": name,
            "computeEnvironmentArn": arn,
        }),
    )
}

async fn describe_compute_environments(State(state): State<BatchState>, _body: Bytes) -> Response {
    let s = state.read();
    let envs: Vec<_> = s
        .compute_environments
        .values()
        .map(|e| {
            json!({
                "computeEnvironmentName": e.name,
                "computeEnvironmentArn": e.arn,
                "type": e.type_,
                "state": e.state,
                "status": e.status,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "computeEnvironments": envs }))
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_err(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", "ClientException")
        .body(Body::from(
            serde_json::to_vec(&json!({ "message": msg, "Type": "ClientException" }))
                .unwrap_or_default(),
        ))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Batch::new()));
}
