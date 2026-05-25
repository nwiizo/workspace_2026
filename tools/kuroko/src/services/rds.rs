//! RDS — AWS Query protocol, sdk_id `rds`.
//!
//! Database instance / cluster / snapshot metadata. Instances and clusters
//! transition to `available` immediately on Create.

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

const SDK_ID: &str = "rds";
const NS: &str = "http://rds.amazonaws.com/doc/2014-10-31/";

const ACTIONS: &[&str] = &[
    "CreateDBInstance",
    "DescribeDBInstances",
    "DeleteDBInstance",
    "ModifyDBInstance",
    "CreateDBCluster",
    "DescribeDBClusters",
    "DeleteDBCluster",
    "ModifyDBCluster",
    "CreateDBSnapshot",
    "DescribeDBSnapshots",
    "DeleteDBSnapshot",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    instances: HashMap<String, DbInstance>,
    clusters: HashMap<String, DbCluster>,
    snapshots: HashMap<String, DbSnapshot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DbInstance {
    identifier: String,
    arn: String,
    class: String,
    engine: String,
    engine_version: String,
    status: String,
    allocated_storage: i32,
    master_username: String,
    endpoint: String,
    port: i32,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DbCluster {
    identifier: String,
    arn: String,
    engine: String,
    engine_version: String,
    status: String,
    master_username: String,
    endpoint: String,
    port: i32,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DbSnapshot {
    identifier: String,
    arn: String,
    db_instance_identifier: String,
    engine: String,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Rds {
    state: Arc<RwLock<State>>,
}

impl Rds {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Rds {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Rds {
    fn name(&self) -> &'static str {
        "rds"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("rds").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("rds", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Rds {
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
            "CreateDBInstance" => self.create_db_instance(params),
            "DescribeDBInstances" => self.describe_db_instances(params),
            "DeleteDBInstance" => self.delete_db_instance(params),
            "ModifyDBInstance" => self.modify_db_instance(params),
            "CreateDBCluster" => self.create_db_cluster(params),
            "DescribeDBClusters" => self.describe_db_clusters(params),
            "DeleteDBCluster" => self.delete_db_cluster(params),
            "ModifyDBCluster" => self.modify_db_cluster(params),
            "CreateDBSnapshot" => self.create_db_snapshot(params),
            "DescribeDBSnapshots" => self.describe_db_snapshots(params),
            "DeleteDBSnapshot" => self.delete_db_snapshot(params),
            other => Err(AwsError::unsupported(format!("RDS::{other}"))),
        }
    }
}

impl Rds {
    fn create_db_instance(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBInstanceIdentifier")?;
        let class = required(p, "DBInstanceClass")?;
        let engine = required(p, "Engine")?;
        let username = p
            .get("MasterUsername")
            .cloned()
            .unwrap_or_else(|| "admin".into());
        let allocated_storage: i32 = p
            .get("AllocatedStorage")
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);
        let port: i32 = p.get("Port").and_then(|v| v.parse().ok()).unwrap_or(
            // sensible default per engine
            if engine.contains("postgres") {
                5432
            } else if engine.contains("mysql") || engine.contains("mariadb") {
                3306
            } else {
                1433
            },
        );
        let mut s = self.state.write();
        if s.instances.contains_key(&id) {
            return Err(AwsError::new(
                "DBInstanceAlreadyExists",
                format!("instance '{id}' already exists"),
            ));
        }
        let endpoint = format!("{id}.kuroko.{EMULATED_REGION}.rds.amazonaws.com");
        let inst = DbInstance {
            arn: instance_arn(&id),
            identifier: id.clone(),
            class,
            engine: engine.clone(),
            engine_version: p
                .get("EngineVersion")
                .cloned()
                .unwrap_or_else(|| "default".into()),
            status: "available".into(),
            allocated_storage,
            master_username: username,
            endpoint,
            port,
            created: chrono::Utc::now(),
        };
        let body = format!("<DBInstance>{}</DBInstance>", db_instance_xml(&inst));
        s.instances.insert(id, inst);
        Ok(wrap("CreateDBInstance", &body))
    }

    fn describe_db_instances(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter = p.get("DBInstanceIdentifier").cloned();
        let s = self.state.read();
        let mut members = String::new();
        for inst in s.instances.values() {
            if let Some(f) = &filter
                && &inst.identifier != f
            {
                continue;
            }
            members.push_str(&format!(
                "<DBInstance>{}</DBInstance>",
                db_instance_xml(inst)
            ));
        }
        Ok(wrap(
            "DescribeDBInstances",
            &format!("<DBInstances>{members}</DBInstances>"),
        ))
    }

    fn delete_db_instance(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBInstanceIdentifier")?;
        let mut s = self.state.write();
        let inst = s
            .instances
            .remove(&id)
            .ok_or_else(|| not_found_instance(&id))?;
        Ok(wrap(
            "DeleteDBInstance",
            &format!("<DBInstance>{}</DBInstance>", db_instance_xml(&inst)),
        ))
    }

    fn modify_db_instance(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBInstanceIdentifier")?;
        let mut s = self.state.write();
        let inst = s
            .instances
            .get_mut(&id)
            .ok_or_else(|| not_found_instance(&id))?;
        if let Some(c) = p.get("DBInstanceClass") {
            inst.class = c.clone();
        }
        if let Some(v) = p.get("EngineVersion") {
            inst.engine_version = v.clone();
        }
        if let Some(s_size) = p.get("AllocatedStorage").and_then(|v| v.parse().ok()) {
            inst.allocated_storage = s_size;
        }
        Ok(wrap(
            "ModifyDBInstance",
            &format!("<DBInstance>{}</DBInstance>", db_instance_xml(inst)),
        ))
    }

    fn create_db_cluster(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBClusterIdentifier")?;
        let engine = required(p, "Engine")?;
        let mut s = self.state.write();
        if s.clusters.contains_key(&id) {
            return Err(AwsError::new(
                "DBClusterAlreadyExistsFault",
                format!("cluster '{id}' already exists"),
            ));
        }
        let port: i32 = p.get("Port").and_then(|v| v.parse().ok()).unwrap_or(5432);
        let cluster = DbCluster {
            arn: cluster_arn(&id),
            identifier: id.clone(),
            engine,
            engine_version: p
                .get("EngineVersion")
                .cloned()
                .unwrap_or_else(|| "default".into()),
            status: "available".into(),
            master_username: p
                .get("MasterUsername")
                .cloned()
                .unwrap_or_else(|| "admin".into()),
            endpoint: format!("{id}.cluster.kuroko.{EMULATED_REGION}.rds.amazonaws.com"),
            port,
            created: chrono::Utc::now(),
        };
        let body = format!("<DBCluster>{}</DBCluster>", db_cluster_xml(&cluster));
        s.clusters.insert(id, cluster);
        Ok(wrap("CreateDBCluster", &body))
    }

    fn describe_db_clusters(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let filter = p.get("DBClusterIdentifier").cloned();
        let s = self.state.read();
        let mut members = String::new();
        for cluster in s.clusters.values() {
            if let Some(f) = &filter
                && &cluster.identifier != f
            {
                continue;
            }
            members.push_str(&format!(
                "<DBCluster>{}</DBCluster>",
                db_cluster_xml(cluster)
            ));
        }
        Ok(wrap(
            "DescribeDBClusters",
            &format!("<DBClusters>{members}</DBClusters>"),
        ))
    }

    fn delete_db_cluster(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBClusterIdentifier")?;
        let mut s = self.state.write();
        let cluster = s
            .clusters
            .remove(&id)
            .ok_or_else(|| not_found_cluster(&id))?;
        Ok(wrap(
            "DeleteDBCluster",
            &format!("<DBCluster>{}</DBCluster>", db_cluster_xml(&cluster)),
        ))
    }

    fn modify_db_cluster(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBClusterIdentifier")?;
        let mut s = self.state.write();
        let cluster = s
            .clusters
            .get_mut(&id)
            .ok_or_else(|| not_found_cluster(&id))?;
        if let Some(v) = p.get("EngineVersion") {
            cluster.engine_version = v.clone();
        }
        Ok(wrap(
            "ModifyDBCluster",
            &format!("<DBCluster>{}</DBCluster>", db_cluster_xml(cluster)),
        ))
    }

    fn create_db_snapshot(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let snap_id = required(p, "DBSnapshotIdentifier")?;
        let inst_id = required(p, "DBInstanceIdentifier")?;
        let s_state = self.state.read();
        let inst = s_state
            .instances
            .get(&inst_id)
            .ok_or_else(|| not_found_instance(&inst_id))?;
        let engine = inst.engine.clone();
        drop(s_state);
        let snap = DbSnapshot {
            arn: snapshot_arn(&snap_id),
            identifier: snap_id.clone(),
            db_instance_identifier: inst_id,
            engine,
            status: "available".into(),
            created: chrono::Utc::now(),
        };
        let body = format!("<DBSnapshot>{}</DBSnapshot>", db_snapshot_xml(&snap));
        self.state.write().snapshots.insert(snap_id, snap);
        Ok(wrap("CreateDBSnapshot", &body))
    }

    fn describe_db_snapshots(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let inst_filter = p.get("DBInstanceIdentifier").cloned();
        let snap_filter = p.get("DBSnapshotIdentifier").cloned();
        let s = self.state.read();
        let mut members = String::new();
        for snap in s.snapshots.values() {
            if let Some(f) = &inst_filter
                && &snap.db_instance_identifier != f
            {
                continue;
            }
            if let Some(f) = &snap_filter
                && &snap.identifier != f
            {
                continue;
            }
            members.push_str(&format!(
                "<DBSnapshot>{}</DBSnapshot>",
                db_snapshot_xml(snap)
            ));
        }
        Ok(wrap(
            "DescribeDBSnapshots",
            &format!("<DBSnapshots>{members}</DBSnapshots>"),
        ))
    }

    fn delete_db_snapshot(&self, p: &HashMap<String, String>) -> Result<String, AwsError> {
        let id = required(p, "DBSnapshotIdentifier")?;
        let mut s = self.state.write();
        let snap = s.snapshots.remove(&id).ok_or_else(|| {
            AwsError::new("DBSnapshotNotFound", format!("snapshot '{id}' not found"))
        })?;
        Ok(wrap(
            "DeleteDBSnapshot",
            &format!("<DBSnapshot>{}</DBSnapshot>", db_snapshot_xml(&snap)),
        ))
    }
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("MissingParameter", format!("{key} required")))
}

fn not_found_instance(id: &str) -> AwsError {
    AwsError::new(
        "DBInstanceNotFound",
        format!("DB instance '{id}' not found"),
    )
}

fn not_found_cluster(id: &str) -> AwsError {
    AwsError::new(
        "DBClusterNotFoundFault",
        format!("DB cluster '{id}' not found"),
    )
}

fn instance_arn(id: &str) -> String {
    format!("arn:aws:rds:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:db:{id}")
}

fn cluster_arn(id: &str) -> String {
    format!("arn:aws:rds:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster:{id}")
}

fn snapshot_arn(id: &str) -> String {
    format!("arn:aws:rds:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:snapshot:{id}")
}

fn db_instance_xml(i: &DbInstance) -> String {
    format!(
        "<DBInstanceIdentifier>{id}</DBInstanceIdentifier><DBInstanceArn>{arn}</DBInstanceArn><DBInstanceClass>{class}</DBInstanceClass><Engine>{engine}</Engine><EngineVersion>{ver}</EngineVersion><DBInstanceStatus>{status}</DBInstanceStatus><AllocatedStorage>{storage}</AllocatedStorage><MasterUsername>{user}</MasterUsername><Endpoint><Address>{ep}</Address><Port>{port}</Port></Endpoint><InstanceCreateTime>{ts}</InstanceCreateTime>",
        id = xml_escape(&i.identifier),
        arn = xml_escape(&i.arn),
        class = xml_escape(&i.class),
        engine = xml_escape(&i.engine),
        ver = xml_escape(&i.engine_version),
        status = xml_escape(&i.status),
        storage = i.allocated_storage,
        user = xml_escape(&i.master_username),
        ep = xml_escape(&i.endpoint),
        port = i.port,
        ts = i.created.to_rfc3339(),
    )
}

fn db_cluster_xml(c: &DbCluster) -> String {
    format!(
        "<DBClusterIdentifier>{id}</DBClusterIdentifier><DBClusterArn>{arn}</DBClusterArn><Engine>{engine}</Engine><EngineVersion>{ver}</EngineVersion><Status>{status}</Status><MasterUsername>{user}</MasterUsername><Endpoint>{ep}</Endpoint><Port>{port}</Port><ClusterCreateTime>{ts}</ClusterCreateTime>",
        id = xml_escape(&c.identifier),
        arn = xml_escape(&c.arn),
        engine = xml_escape(&c.engine),
        ver = xml_escape(&c.engine_version),
        status = xml_escape(&c.status),
        user = xml_escape(&c.master_username),
        ep = xml_escape(&c.endpoint),
        port = c.port,
        ts = c.created.to_rfc3339(),
    )
}

fn db_snapshot_xml(s: &DbSnapshot) -> String {
    format!(
        "<DBSnapshotIdentifier>{id}</DBSnapshotIdentifier><DBSnapshotArn>{arn}</DBSnapshotArn><DBInstanceIdentifier>{inst}</DBInstanceIdentifier><Engine>{engine}</Engine><Status>{status}</Status><SnapshotCreateTime>{ts}</SnapshotCreateTime>",
        id = xml_escape(&s.identifier),
        arn = xml_escape(&s.arn),
        inst = xml_escape(&s.db_instance_identifier),
        engine = xml_escape(&s.engine),
        status = xml_escape(&s.status),
        ts = s.created.to_rfc3339(),
    )
}

fn wrap(action: &str, body: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><{action}Result>{body}</{action}Result><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

pub fn register(registry: &Arc<Registry>) {
    registry.register_query(Arc::new(Rds::new()));
}
