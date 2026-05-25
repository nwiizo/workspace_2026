//! Amazon MSK (kafka) — restJson1, mounted under `/v1/clusters` and `/api/v2/clusters`.

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
    clusters: HashMap<String, Cluster>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Cluster {
    arn: String,
    name: String,
    kafka_version: String,
    number_of_broker_nodes: i32,
    state: String,
    cluster_type: String,
}

pub struct Kafka {
    state: Arc<RwLock<State_>>,
}
impl Kafka {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Kafka {
    fn default() -> Self {
        Self::new()
    }
}

type KState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Kafka {
    fn name(&self) -> &'static str {
        "kafka"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("kafka").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("kafka", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route("/v1/clusters", post(create_v1).get(list_v1))
            .route(
                "/v1/clusters/{arn}",
                get(describe_v1).delete(delete_cluster),
            )
            .route("/api/v2/clusters", post(create_v2).get(list_v2))
            .route(
                "/api/v2/clusters/{arn}",
                get(describe_v2).delete(delete_cluster),
            )
            .with_state(s)
    }
}

async fn create_v1(State(state): State<KState>, body: Bytes) -> Response {
    create_cluster(state, body, "PROVISIONED").await
}

async fn create_v2(State(state): State<KState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let cluster_type = req
        .get("clusterType")
        .and_then(Value::as_str)
        .unwrap_or("PROVISIONED")
        .to_string();
    create_cluster(state, body, &cluster_type).await
}

async fn create_cluster(state: KState, body: Bytes, cluster_type: &str) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let name = match req.get("clusterName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => {
            return rest_error(AwsError::new("BadRequestException", "clusterName required"));
        }
    };
    // v2 wraps provisioned config under .provisioned
    let provisioned = req.get("provisioned");
    let version = req
        .get("kafkaVersion")
        .and_then(Value::as_str)
        .or_else(|| {
            provisioned
                .and_then(|p| p.get("kafkaVersion"))
                .and_then(Value::as_str)
        })
        .unwrap_or("3.5.1")
        .to_string();
    let broker_nodes = req
        .get("numberOfBrokerNodes")
        .and_then(Value::as_i64)
        .or_else(|| {
            provisioned
                .and_then(|p| p.get("numberOfBrokerNodes"))
                .and_then(Value::as_i64)
        })
        .unwrap_or(2) as i32;
    let arn = format!(
        "arn:aws:kafka:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster/{name}/{}",
        Uuid::new_v4()
    );
    let c = Cluster {
        arn: arn.clone(),
        name,
        kafka_version: version,
        number_of_broker_nodes: broker_nodes,
        state: "ACTIVE".into(),
        cluster_type: cluster_type.to_string(),
    };
    state.write().clusters.insert(arn.clone(), c.clone());
    rest_json(
        StatusCode::OK,
        &json!({
            "clusterArn": arn,
            "clusterName": c.name,
            "state": c.state,
        }),
    )
}

async fn describe_v1(State(state): State<KState>, Path(arn): Path<String>) -> Response {
    let arn = decode(&arn);
    let s = state.read();
    match s.clusters.get(&arn) {
        Some(c) => rest_json(
            StatusCode::OK,
            &json!({ "clusterInfo": cluster_json_v1(c) }),
        ),
        None => rest_error(AwsError::new("NotFoundException", "cluster not found")),
    }
}

async fn describe_v2(State(state): State<KState>, Path(arn): Path<String>) -> Response {
    let arn = decode(&arn);
    let s = state.read();
    match s.clusters.get(&arn) {
        Some(c) => rest_json(
            StatusCode::OK,
            &json!({ "clusterInfo": cluster_json_v2(c) }),
        ),
        None => rest_error(AwsError::new("NotFoundException", "cluster not found")),
    }
}

async fn list_v1(State(state): State<KState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s.clusters.values().map(cluster_json_v1).collect();
    rest_json(StatusCode::OK, &json!({ "clusterInfoList": items }))
}

async fn list_v2(State(state): State<KState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s.clusters.values().map(cluster_json_v2).collect();
    rest_json(StatusCode::OK, &json!({ "clusterInfoList": items }))
}

async fn delete_cluster(State(state): State<KState>, Path(arn): Path<String>) -> Response {
    let arn = decode(&arn);
    let mut s = state.write();
    match s.clusters.remove(&arn) {
        Some(c) => rest_json(
            StatusCode::OK,
            &json!({ "clusterArn": c.arn, "state": "DELETING" }),
        ),
        None => rest_error(AwsError::new("NotFoundException", "cluster not found")),
    }
}

fn cluster_json_v1(c: &Cluster) -> Value {
    json!({
        "clusterArn": c.arn,
        "clusterName": c.name,
        "state": c.state,
        "currentBrokerSoftwareInfo": { "kafkaVersion": c.kafka_version },
        "numberOfBrokerNodes": c.number_of_broker_nodes,
    })
}

fn cluster_json_v2(c: &Cluster) -> Value {
    json!({
        "clusterArn": c.arn,
        "clusterName": c.name,
        "state": c.state,
        "clusterType": c.cluster_type,
        "provisioned": {
            "numberOfBrokerNodes": c.number_of_broker_nodes,
            "currentBrokerSoftwareInfo": { "kafkaVersion": c.kafka_version },
        },
    })
}

fn decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
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
    registry.register(Arc::new(Kafka::new()));
}
