//! Amazon MQ — restJson1 under `/v1/brokers`.

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
    brokers: HashMap<String, Broker>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Broker {
    id: String,
    arn: String,
    name: String,
    engine: String,
    engine_version: String,
    deployment_mode: String,
    state: String,
    instance_type: String,
}

pub struct Mq {
    state: Arc<RwLock<State_>>,
}
impl Mq {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Mq {
    fn default() -> Self {
        Self::new()
    }
}

type MqState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Mq {
    fn name(&self) -> &'static str {
        "mq"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("mq").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("mq", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route("/v1/brokers", post(create_broker).get(list_brokers))
            .route(
                "/v1/brokers/{id}",
                get(describe_broker).delete(delete_broker),
            )
            .with_state(s)
    }
}

async fn create_broker(State(state): State<MqState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let name = match req.get("brokerName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_error(AwsError::new("BadRequestException", "brokerName required")),
    };
    let engine = req
        .get("engineType")
        .and_then(Value::as_str)
        .unwrap_or("ACTIVEMQ")
        .to_string();
    let engine_version = req
        .get("engineVersion")
        .and_then(Value::as_str)
        .unwrap_or("5.17.6")
        .to_string();
    let deployment_mode = req
        .get("deploymentMode")
        .and_then(Value::as_str)
        .unwrap_or("SINGLE_INSTANCE")
        .to_string();
    let instance_type = req
        .get("hostInstanceType")
        .and_then(Value::as_str)
        .unwrap_or("mq.t3.micro")
        .to_string();
    let id = format!("b-{}", Uuid::new_v4().simple());
    let arn = format!("arn:aws:mq:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:broker:{name}:{id}");
    let b = Broker {
        id: id.clone(),
        arn: arn.clone(),
        name,
        engine,
        engine_version,
        deployment_mode,
        state: "RUNNING".into(),
        instance_type,
    };
    state.write().brokers.insert(id.clone(), b);
    rest_json(StatusCode::OK, &json!({ "brokerId": id, "brokerArn": arn }))
}

async fn describe_broker(State(state): State<MqState>, Path(id): Path<String>) -> Response {
    let s = state.read();
    match s.brokers.get(&id) {
        Some(b) => rest_json(StatusCode::OK, &broker_full(b)),
        None => rest_error(AwsError::new("NotFoundException", "broker not found")),
    }
}

async fn list_brokers(State(state): State<MqState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s.brokers.values().map(broker_summary).collect();
    rest_json(StatusCode::OK, &json!({ "brokerSummaries": items }))
}

async fn delete_broker(State(state): State<MqState>, Path(id): Path<String>) -> Response {
    let mut s = state.write();
    match s.brokers.remove(&id) {
        Some(b) => rest_json(
            StatusCode::OK,
            &json!({ "brokerId": b.id, "brokerArn": b.arn }),
        ),
        None => rest_error(AwsError::new("NotFoundException", "broker not found")),
    }
}

fn broker_summary(b: &Broker) -> Value {
    json!({
        "brokerId": b.id,
        "brokerArn": b.arn,
        "brokerName": b.name,
        "brokerState": b.state,
        "deploymentMode": b.deployment_mode,
        "engineType": b.engine,
        "hostInstanceType": b.instance_type,
    })
}

fn broker_full(b: &Broker) -> Value {
    json!({
        "brokerId": b.id,
        "brokerArn": b.arn,
        "brokerName": b.name,
        "brokerState": b.state,
        "deploymentMode": b.deployment_mode,
        "engineType": b.engine,
        "engineVersion": b.engine_version,
        "hostInstanceType": b.instance_type,
        "publiclyAccessible": false,
        "autoMinorVersionUpgrade": true,
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
    registry.register(Arc::new(Mq::new()));
}
