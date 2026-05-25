//! MemoryDB for Redis — AWS JSON 1.1, target prefix `AmazonMemoryDB`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "AmazonMemoryDB";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    clusters: HashMap<String, Cluster>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Cluster {
    name: String,
    arn: String,
    status: String,
    node_type: String,
    num_shards: i32,
    engine: String,
    endpoint: String,
}

pub struct MemoryDb {
    state: Arc<RwLock<State>>,
}
impl MemoryDb {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for MemoryDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for MemoryDb {
    fn name(&self) -> &'static str {
        "memorydb"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("memorydb").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("memorydb", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for MemoryDb {
    fn target_prefix(&self) -> &'static str {
        TARGET_PREFIX
    }
    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        body: Bytes,
    ) -> Result<Value, AwsError> {
        let req: Value = if body.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&body)
                .map_err(|e| AwsError::new("InvalidParameterValueException", e.to_string()))?
        };
        match action {
            "CreateCluster" => {
                let name = req
                    .get("ClusterName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AwsError::new("InvalidParameterValueException", "ClusterName required")
                    })?
                    .to_string();
                let node_type = req
                    .get("NodeType")
                    .and_then(Value::as_str)
                    .unwrap_or("db.t4g.small")
                    .to_string();
                let num_shards = req.get("NumShards").and_then(Value::as_i64).unwrap_or(1) as i32;
                let mut s = self.state.write();
                if s.clusters.contains_key(&name) {
                    return Err(AwsError::new(
                        "ClusterAlreadyExistsFault",
                        format!("cluster '{name}' exists"),
                    ));
                }
                let c = Cluster {
                    arn: format!(
                        "arn:aws:memorydb:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster/{name}"
                    ),
                    endpoint: format!("{name}.kuroko.memorydb.amazonaws.com"),
                    name: name.clone(),
                    status: "available".into(),
                    node_type,
                    num_shards,
                    engine: "redis".into(),
                };
                let resp = cluster_json(&c);
                s.clusters.insert(name, c);
                Ok(json!({ "Cluster": resp }))
            }
            "DescribeClusters" => {
                let filter = req
                    .get("ClusterName")
                    .and_then(Value::as_str)
                    .map(String::from);
                let s = self.state.read();
                let clusters: Vec<_> = s
                    .clusters
                    .values()
                    .filter(|c| filter.as_deref().is_none_or(|f| f == c.name))
                    .map(cluster_json)
                    .collect();
                Ok(json!({ "Clusters": clusters }))
            }
            "DeleteCluster" => {
                let name = req
                    .get("ClusterName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AwsError::new("InvalidParameterValueException", "ClusterName required")
                    })?
                    .to_string();
                let mut s = self.state.write();
                let c = s.clusters.remove(&name).ok_or_else(|| {
                    AwsError::new(
                        "ClusterNotFoundFault",
                        format!("cluster '{name}' not found"),
                    )
                })?;
                Ok(json!({ "Cluster": cluster_json(&c) }))
            }
            other => Err(AwsError::unsupported(format!("MemoryDB::{other}"))),
        }
    }
}

fn cluster_json(c: &Cluster) -> Value {
    json!({
        "Name": c.name,
        "ARN": c.arn,
        "Status": c.status,
        "NodeType": c.node_type,
        "NumberOfShards": c.num_shards,
        "Engine": c.engine,
        "ClusterEndpoint": { "Address": c.endpoint, "Port": 6379 },
    })
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(MemoryDb::new()));
}
