//! Neptune — AWS Query protocol, sdk_id `neptune`. Shares the RDS protocol
//! shape; kuroko stores Neptune clusters in its own table.

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

const SDK_ID: &str = "neptune";
const NS: &str = "http://rds.amazonaws.com/doc/2014-10-31/";
const ACTIONS: &[&str] = &[
    "CreateDBCluster",
    "DescribeDBClusters",
    "DeleteDBCluster",
    "ModifyDBCluster",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    clusters: HashMap<String, NeptuneCluster>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NeptuneCluster {
    id: String,
    arn: String,
    status: String,
    endpoint: String,
    engine: String,
}

pub struct Neptune {
    state: Arc<RwLock<State>>,
}
impl Neptune {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Neptune {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Neptune {
    fn name(&self) -> &'static str {
        "neptune"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("neptune").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("neptune", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Neptune {
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
            "CreateDBCluster" => {
                let id = required(params, "DBClusterIdentifier")?;
                let engine = params
                    .get("Engine")
                    .cloned()
                    .unwrap_or_else(|| "neptune".into());
                let mut s = self.state.write();
                if s.clusters.contains_key(&id) {
                    return Err(AwsError::new(
                        "DBClusterAlreadyExistsFault",
                        format!("cluster '{id}' exists"),
                    ));
                }
                let c = NeptuneCluster {
                    arn: format!(
                        "arn:aws:neptune:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster:{id}"
                    ),
                    endpoint: format!("{id}.kuroko.neptune.amazonaws.com"),
                    id: id.clone(),
                    status: "available".into(),
                    engine,
                };
                let body = format!("<DBCluster>{}</DBCluster>", cluster_xml(&c));
                s.clusters.insert(id, c);
                Ok(wrap("CreateDBCluster", &body))
            }
            "DescribeDBClusters" => {
                let filter = params.get("DBClusterIdentifier").cloned();
                let s = self.state.read();
                let mut members = String::new();
                for c in s.clusters.values() {
                    if let Some(f) = &filter
                        && &c.id != f
                    {
                        continue;
                    }
                    members.push_str(&format!("<DBCluster>{}</DBCluster>", cluster_xml(c)));
                }
                Ok(wrap(
                    "DescribeDBClusters",
                    &format!("<DBClusters>{members}</DBClusters>"),
                ))
            }
            "DeleteDBCluster" => {
                let id = required(params, "DBClusterIdentifier")?;
                let mut s = self.state.write();
                let c = s.clusters.remove(&id).ok_or_else(|| {
                    AwsError::new(
                        "DBClusterNotFoundFault",
                        format!("cluster '{id}' not found"),
                    )
                })?;
                Ok(wrap(
                    "DeleteDBCluster",
                    &format!("<DBCluster>{}</DBCluster>", cluster_xml(&c)),
                ))
            }
            "ModifyDBCluster" => {
                let id = required(params, "DBClusterIdentifier")?;
                let s = self.state.read();
                let c = s.clusters.get(&id).ok_or_else(|| {
                    AwsError::new(
                        "DBClusterNotFoundFault",
                        format!("cluster '{id}' not found"),
                    )
                })?;
                Ok(wrap(
                    "ModifyDBCluster",
                    &format!("<DBCluster>{}</DBCluster>", cluster_xml(c)),
                ))
            }
            other => Err(AwsError::unsupported(format!("Neptune::{other}"))),
        }
    }
}

fn cluster_xml(c: &NeptuneCluster) -> String {
    format!(
        "<DBClusterIdentifier>{id}</DBClusterIdentifier><DBClusterArn>{arn}</DBClusterArn><Engine>{engine}</Engine><Status>{status}</Status><Endpoint>{ep}</Endpoint><Port>8182</Port>",
        id = xml_escape(&c.id),
        arn = xml_escape(&c.arn),
        engine = xml_escape(&c.engine),
        status = xml_escape(&c.status),
        ep = xml_escape(&c.endpoint),
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
    registry.register_query(Arc::new(Neptune::new()));
}
