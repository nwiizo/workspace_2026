//! EKS — REST protocol mounted under `/clusters/*`.
//!
//! Cluster + Nodegroup metadata. No actual Kubernetes control plane.

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
    name: String,
    arn: String,
    version: String,
    role_arn: String,
    status: String,
    endpoint: String,
    created: chrono::DateTime<chrono::Utc>,
    nodegroups: HashMap<String, Nodegroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Nodegroup {
    name: String,
    arn: String,
    cluster_name: String,
    status: String,
    instance_types: Vec<String>,
    scaling: Value,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Eks {
    state: Arc<RwLock<State_>>,
}

impl Eks {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}

impl Default for Eks {
    fn default() -> Self {
        Self::new()
    }
}

type EksState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Eks {
    fn name(&self) -> &'static str {
        "eks"
    }
    fn reset(&self) {
        self.state.write().clusters.clear();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("eks").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("eks", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route("/clusters", post(create_cluster).get(list_clusters))
            .route(
                "/clusters/{name}",
                get(describe_cluster).delete(delete_cluster),
            )
            .route(
                "/clusters/{name}/node-groups",
                post(create_nodegroup).get(list_nodegroups),
            )
            .route(
                "/clusters/{name}/node-groups/{ng}",
                get(describe_nodegroup).delete(delete_nodegroup),
            )
            .with_state(state)
    }
}

async fn create_cluster(State(state): State<EksState>, body: Bytes) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let name = match req.get("name").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => {
            return rest_error(AwsError::new("InvalidParameterException", "name required"));
        }
    };
    let mut s = state.write();
    if s.clusters.contains_key(&name) {
        return rest_error(
            AwsError::new(
                "ResourceInUseException",
                format!("cluster '{name}' already exists"),
            )
            .status(StatusCode::CONFLICT),
        );
    }
    let cluster = Cluster {
        arn: format!("arn:aws:eks:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster/{name}"),
        name: name.clone(),
        version: req
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("1.31")
            .to_string(),
        role_arn: req
            .get("roleArn")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        status: "ACTIVE".into(),
        endpoint: format!("https://{name}.kuroko.eks.amazonaws.com"),
        created: chrono::Utc::now(),
        nodegroups: HashMap::new(),
    };
    let resp = cluster_json(&cluster);
    s.clusters.insert(name, cluster);
    rest_json(StatusCode::CREATED, &json!({ "cluster": resp }))
}

async fn list_clusters(State(state): State<EksState>) -> Response {
    let s = state.read();
    let names: Vec<_> = s.clusters.keys().cloned().collect();
    rest_json(StatusCode::OK, &json!({ "clusters": names }))
}

async fn describe_cluster(State(state): State<EksState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    match s.clusters.get(&name) {
        Some(c) => rest_json(StatusCode::OK, &json!({ "cluster": cluster_json(c) })),
        None => rest_error(not_found("cluster", &name)),
    }
}

async fn delete_cluster(State(state): State<EksState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    match s.clusters.remove(&name) {
        Some(c) => rest_json(StatusCode::OK, &json!({ "cluster": cluster_json(&c) })),
        None => rest_error(not_found("cluster", &name)),
    }
}

async fn create_nodegroup(
    State(state): State<EksState>,
    Path(cluster_name): Path<String>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let ng_name = match req.get("nodegroupName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => {
            return rest_error(AwsError::new(
                "InvalidParameterException",
                "nodegroupName required",
            ));
        }
    };
    let mut s = state.write();
    let cluster = match s.clusters.get_mut(&cluster_name) {
        Some(c) => c,
        None => return rest_error(not_found("cluster", &cluster_name)),
    };
    if cluster.nodegroups.contains_key(&ng_name) {
        return rest_error(
            AwsError::new(
                "ResourceInUseException",
                format!("nodegroup '{ng_name}' already exists"),
            )
            .status(StatusCode::CONFLICT),
        );
    }
    let instance_types: Vec<String> = req
        .get("instanceTypes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_else(|| vec!["t3.medium".into()]);
    let ng = Nodegroup {
        arn: format!(
            "arn:aws:eks:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:nodegroup/{cluster_name}/{ng_name}/{}",
            Uuid::new_v4().simple()
        ),
        name: ng_name.clone(),
        cluster_name: cluster_name.clone(),
        status: "ACTIVE".into(),
        instance_types,
        scaling: req.get("scalingConfig").cloned().unwrap_or(Value::Null),
        created: chrono::Utc::now(),
    };
    let resp = nodegroup_json(&ng);
    cluster.nodegroups.insert(ng_name, ng);
    rest_json(StatusCode::CREATED, &json!({ "nodegroup": resp }))
}

async fn list_nodegroups(
    State(state): State<EksState>,
    Path(cluster_name): Path<String>,
) -> Response {
    let s = state.read();
    let cluster = match s.clusters.get(&cluster_name) {
        Some(c) => c,
        None => return rest_error(not_found("cluster", &cluster_name)),
    };
    let names: Vec<_> = cluster.nodegroups.keys().cloned().collect();
    rest_json(StatusCode::OK, &json!({ "nodegroups": names }))
}

async fn describe_nodegroup(
    State(state): State<EksState>,
    Path((cluster_name, ng_name)): Path<(String, String)>,
) -> Response {
    let s = state.read();
    let cluster = match s.clusters.get(&cluster_name) {
        Some(c) => c,
        None => return rest_error(not_found("cluster", &cluster_name)),
    };
    match cluster.nodegroups.get(&ng_name) {
        Some(ng) => rest_json(StatusCode::OK, &json!({ "nodegroup": nodegroup_json(ng) })),
        None => rest_error(not_found("nodegroup", &ng_name)),
    }
}

async fn delete_nodegroup(
    State(state): State<EksState>,
    Path((cluster_name, ng_name)): Path<(String, String)>,
) -> Response {
    let mut s = state.write();
    let cluster = match s.clusters.get_mut(&cluster_name) {
        Some(c) => c,
        None => return rest_error(not_found("cluster", &cluster_name)),
    };
    match cluster.nodegroups.remove(&ng_name) {
        Some(ng) => rest_json(StatusCode::OK, &json!({ "nodegroup": nodegroup_json(&ng) })),
        None => rest_error(not_found("nodegroup", &ng_name)),
    }
}

fn cluster_json(c: &Cluster) -> Value {
    json!({
        "name": c.name,
        "arn": c.arn,
        "version": c.version,
        "roleArn": c.role_arn,
        "status": c.status,
        "endpoint": c.endpoint,
        "createdAt": c.created.timestamp(),
        "platformVersion": "eks.1",
    })
}

fn nodegroup_json(n: &Nodegroup) -> Value {
    json!({
        "nodegroupName": n.name,
        "nodegroupArn": n.arn,
        "clusterName": n.cluster_name,
        "status": n.status,
        "instanceTypes": n.instance_types,
        "scalingConfig": n.scaling,
        "createdAt": n.created.timestamp(),
    })
}

fn not_found(kind: &str, name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("{kind} '{name}' not found"),
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
    let body = json!({
        "Type": err.code,
        "message": err.message,
    });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Eks::new()));
}
