//! ElastiCache — AWS Query protocol, sdk_id `elasticache`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, QueryProtocolService, Service, ServiceContext,
    persistence_error,
};

const SDK_ID: &str = "elasticache";
const NS: &str = "http://elasticache.amazonaws.com/doc/2015-02-02/";

const ACTIONS: &[&str] = &[
    "CreateCacheCluster",
    "DescribeCacheClusters",
    "DeleteCacheCluster",
    "ModifyCacheCluster",
    "CreateReplicationGroup",
    "DescribeReplicationGroups",
    "DeleteReplicationGroup",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    clusters: HashMap<String, Cluster>,
    replication_groups: HashMap<String, ReplicationGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Cluster {
    id: String,
    engine: String,
    status: String,
    node_type: String,
    num_nodes: i32,
    endpoint: String,
    port: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReplicationGroup {
    id: String,
    description: String,
    status: String,
}

pub struct ElastiCache {
    state: Arc<RwLock<State>>,
}

impl ElastiCache {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for ElastiCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ElastiCache {
    fn name(&self) -> &'static str {
        "elasticache"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("elasticache")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("elasticache", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for ElastiCache {
    fn sdk_id(&self) -> &'static str {
        SDK_ID
    }
    fn actions(&self) -> &'static [&'static str] {
        ACTIONS
    }
    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        match action {
            "CreateCacheCluster" => {
                let id = required(params, "CacheClusterId")?;
                let engine = params
                    .get("Engine")
                    .cloned()
                    .unwrap_or_else(|| "redis".into());
                let node_type = params
                    .get("CacheNodeType")
                    .cloned()
                    .unwrap_or_else(|| "cache.t3.micro".into());
                let num_nodes: i32 = params
                    .get("NumCacheNodes")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let port: i32 = if engine.contains("memcached") {
                    11211
                } else {
                    6379
                };
                let mut s = self.state.write();
                if s.clusters.contains_key(&id) {
                    return Err(AwsError::new(
                        "CacheClusterAlreadyExistsFault",
                        format!("cluster '{id}' already exists"),
                    ));
                }
                let endpoint = format!("{id}.kuroko.cache.amazonaws.com");
                let c = Cluster {
                    id: id.clone(),
                    engine,
                    status: "available".into(),
                    node_type,
                    num_nodes,
                    endpoint,
                    port,
                };
                let body = format!("<CacheCluster>{}</CacheCluster>", cluster_xml(&c));
                s.clusters.insert(id, c);
                Ok(wrap("CreateCacheCluster", &body))
            }
            "DescribeCacheClusters" => {
                let filter = params.get("CacheClusterId").cloned();
                let s = self.state.read();
                let mut members = String::new();
                for c in s.clusters.values() {
                    if let Some(f) = &filter
                        && &c.id != f
                    {
                        continue;
                    }
                    members.push_str(&format!("<CacheCluster>{}</CacheCluster>", cluster_xml(c)));
                }
                Ok(wrap(
                    "DescribeCacheClusters",
                    &format!("<CacheClusters>{members}</CacheClusters>"),
                ))
            }
            "DeleteCacheCluster" => {
                let id = required(params, "CacheClusterId")?;
                let mut s = self.state.write();
                let c = s
                    .clusters
                    .remove(&id)
                    .ok_or_else(|| not_found_cluster(&id))?;
                Ok(wrap(
                    "DeleteCacheCluster",
                    &format!("<CacheCluster>{}</CacheCluster>", cluster_xml(&c)),
                ))
            }
            "ModifyCacheCluster" => {
                let id = required(params, "CacheClusterId")?;
                let s = self.state.read();
                let c = s.clusters.get(&id).ok_or_else(|| not_found_cluster(&id))?;
                Ok(wrap(
                    "ModifyCacheCluster",
                    &format!("<CacheCluster>{}</CacheCluster>", cluster_xml(c)),
                ))
            }
            "CreateReplicationGroup" => {
                let id = required(params, "ReplicationGroupId")?;
                let description = params
                    .get("ReplicationGroupDescription")
                    .cloned()
                    .unwrap_or_default();
                let mut s = self.state.write();
                s.replication_groups.insert(
                    id.clone(),
                    ReplicationGroup {
                        id: id.clone(),
                        description: description.clone(),
                        status: "available".into(),
                    },
                );
                Ok(wrap(
                    "CreateReplicationGroup",
                    &format!(
                        "<ReplicationGroup><ReplicationGroupId>{id}</ReplicationGroupId><Description>{desc}</Description><Status>available</Status></ReplicationGroup>",
                        id = xml_escape(&id),
                        desc = xml_escape(&description),
                    ),
                ))
            }
            "DescribeReplicationGroups" => {
                let s = self.state.read();
                let mut members = String::new();
                for r in s.replication_groups.values() {
                    members.push_str(&format!(
                        "<ReplicationGroup><ReplicationGroupId>{id}</ReplicationGroupId><Description>{desc}</Description><Status>{status}</Status></ReplicationGroup>",
                        id = xml_escape(&r.id),
                        desc = xml_escape(&r.description),
                        status = r.status,
                    ));
                }
                Ok(wrap(
                    "DescribeReplicationGroups",
                    &format!("<ReplicationGroups>{members}</ReplicationGroups>"),
                ))
            }
            "DeleteReplicationGroup" => {
                let id = required(params, "ReplicationGroupId")?;
                self.state.write().replication_groups.remove(&id);
                Ok(wrap(
                    "DeleteReplicationGroup",
                    &format!(
                        "<ReplicationGroup><ReplicationGroupId>{id}</ReplicationGroupId><Status>deleting</Status></ReplicationGroup>",
                        id = xml_escape(&id),
                    ),
                ))
            }
            other => Err(AwsError::unsupported(format!("ElastiCache::{other}"))),
        }
    }
}

fn cluster_xml(c: &Cluster) -> String {
    format!(
        "<CacheClusterId>{id}</CacheClusterId><Engine>{engine}</Engine><CacheClusterStatus>{status}</CacheClusterStatus><CacheNodeType>{nt}</CacheNodeType><NumCacheNodes>{n}</NumCacheNodes><ConfigurationEndpoint><Address>{ep}</Address><Port>{port}</Port></ConfigurationEndpoint><ARN>arn:aws:elasticache:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster:{id}</ARN>",
        id = xml_escape(&c.id),
        engine = xml_escape(&c.engine),
        status = xml_escape(&c.status),
        nt = xml_escape(&c.node_type),
        n = c.num_nodes,
        ep = xml_escape(&c.endpoint),
        port = c.port,
    )
}

fn not_found_cluster(id: &str) -> AwsError {
    AwsError::new(
        "CacheClusterNotFoundFault",
        format!("cluster '{id}' not found"),
    )
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("MissingParameter", format!("{key} required")))
}

fn wrap(action: &str, body: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><{action}Result>{body}</{action}Result><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

pub fn register(registry: &Arc<Registry>) {
    registry.register_query(Arc::new(ElastiCache::new()));
}
