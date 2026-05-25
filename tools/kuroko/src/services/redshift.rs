//! Redshift — AWS Query protocol, sdk_id `redshift`.

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

const SDK_ID: &str = "redshift";
const NS: &str = "http://redshift.amazonaws.com/doc/2012-12-01/";
const ACTIONS: &[&str] = &[
    "CreateCluster",
    "DescribeClusters",
    "DeleteCluster",
    "ModifyCluster",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    clusters: HashMap<String, Cluster>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Cluster {
    id: String,
    node_type: String,
    status: String,
    master_username: String,
    db_name: String,
    endpoint: String,
    port: i32,
    num_nodes: i32,
}

pub struct Redshift {
    state: Arc<RwLock<State>>,
}

impl Redshift {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Redshift {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Redshift {
    fn name(&self) -> &'static str {
        "redshift"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("redshift").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("redshift", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Redshift {
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
            "CreateCluster" => {
                let id = required(params, "ClusterIdentifier")?;
                let node_type = required(params, "NodeType")?;
                let master = params.get("MasterUsername").cloned().unwrap_or_default();
                let db_name = params
                    .get("DBName")
                    .cloned()
                    .unwrap_or_else(|| "dev".into());
                let num_nodes: i32 = params
                    .get("NumberOfNodes")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let mut s = self.state.write();
                if s.clusters.contains_key(&id) {
                    return Err(AwsError::new(
                        "ClusterAlreadyExistsFault",
                        format!("cluster '{id}' already exists"),
                    ));
                }
                let endpoint = format!("{id}.kuroko.redshift.amazonaws.com");
                let c = Cluster {
                    id: id.clone(),
                    node_type,
                    status: "available".into(),
                    master_username: master,
                    db_name,
                    endpoint,
                    port: 5439,
                    num_nodes,
                };
                let body = format!("<Cluster>{}</Cluster>", cluster_xml(&c));
                s.clusters.insert(id, c);
                Ok(wrap("CreateCluster", &body))
            }
            "DescribeClusters" => {
                let filter = params.get("ClusterIdentifier").cloned();
                let s = self.state.read();
                let mut members = String::new();
                for c in s.clusters.values() {
                    if let Some(f) = &filter
                        && &c.id != f
                    {
                        continue;
                    }
                    members.push_str(&format!("<Cluster>{}</Cluster>", cluster_xml(c)));
                }
                Ok(wrap(
                    "DescribeClusters",
                    &format!("<Clusters>{members}</Clusters>"),
                ))
            }
            "DeleteCluster" => {
                let id = required(params, "ClusterIdentifier")?;
                let mut s = self.state.write();
                let c = s.clusters.remove(&id).ok_or_else(|| {
                    AwsError::new("ClusterNotFoundFault", format!("cluster '{id}' not found"))
                })?;
                Ok(wrap(
                    "DeleteCluster",
                    &format!("<Cluster>{}</Cluster>", cluster_xml(&c)),
                ))
            }
            "ModifyCluster" => {
                let id = required(params, "ClusterIdentifier")?;
                let s = self.state.read();
                let c = s.clusters.get(&id).ok_or_else(|| {
                    AwsError::new("ClusterNotFoundFault", format!("cluster '{id}' not found"))
                })?;
                Ok(wrap(
                    "ModifyCluster",
                    &format!("<Cluster>{}</Cluster>", cluster_xml(c)),
                ))
            }
            other => Err(AwsError::unsupported(format!("Redshift::{other}"))),
        }
    }
}

fn cluster_xml(c: &Cluster) -> String {
    format!(
        "<ClusterIdentifier>{id}</ClusterIdentifier><NodeType>{nt}</NodeType><ClusterStatus>{status}</ClusterStatus><MasterUsername>{user}</MasterUsername><DBName>{db}</DBName><Endpoint><Address>{ep}</Address><Port>{port}</Port></Endpoint><NumberOfNodes>{nn}</NumberOfNodes><ClusterCreateTime>{ts}</ClusterCreateTime><ClusterNamespaceArn>arn:aws:redshift:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:namespace:{id}</ClusterNamespaceArn>",
        id = xml_escape(&c.id),
        nt = xml_escape(&c.node_type),
        status = xml_escape(&c.status),
        user = xml_escape(&c.master_username),
        db = xml_escape(&c.db_name),
        ep = xml_escape(&c.endpoint),
        port = c.port,
        nn = c.num_nodes,
        ts = chrono::Utc::now().to_rfc3339(),
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
    registry.register_query(Arc::new(Redshift::new()));
}
