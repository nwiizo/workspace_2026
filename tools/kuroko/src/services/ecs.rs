//! ECS — AWS JSON 1.1, target prefix `AmazonEC2ContainerServiceV20141113`.
//!
//! Cluster / TaskDefinition / Service / Task metadata. No orchestration —
//! tasks transition to RUNNING immediately.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "AmazonEC2ContainerServiceV20141113";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    clusters: HashMap<String, Cluster>,
    task_definitions: HashMap<String, Vec<TaskDefinition>>,
    services: HashMap<String, EcsService>,
    tasks: HashMap<String, Task>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Cluster {
    name: String,
    arn: String,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TaskDefinition {
    family: String,
    revision: i32,
    arn: String,
    container_definitions: Value,
    status: String,
    network_mode: Option<String>,
    cpu: Option<String>,
    memory: Option<String>,
    registered: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EcsService {
    name: String,
    arn: String,
    cluster_arn: String,
    task_definition: String,
    desired_count: i32,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Task {
    arn: String,
    cluster_arn: String,
    task_definition_arn: String,
    last_status: String,
    desired_status: String,
    started: chrono::DateTime<chrono::Utc>,
}

pub struct Ecs {
    state: Arc<RwLock<State>>,
}

impl Ecs {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Ecs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ecs {
    fn name(&self) -> &'static str {
        "ecs"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ecs").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("ecs", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Ecs {
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
                .map_err(|e| AwsError::new("InvalidParameterException", e.to_string()))?
        };
        match action {
            "CreateCluster" => self.create_cluster(&req),
            "ListClusters" => self.list_clusters(),
            "DescribeClusters" => self.describe_clusters(&req),
            "DeleteCluster" => self.delete_cluster(&req),
            "RegisterTaskDefinition" => self.register_task_definition(&req),
            "ListTaskDefinitions" => self.list_task_definitions(&req),
            "DescribeTaskDefinition" => self.describe_task_definition(&req),
            "DeregisterTaskDefinition" => self.deregister_task_definition(&req),
            "CreateService" => self.create_service(&req),
            "ListServices" => self.list_services(&req),
            "DescribeServices" => self.describe_services(&req),
            "DeleteService" => self.delete_service(&req),
            "RunTask" => self.run_task(&req),
            "ListTasks" => self.list_tasks(&req),
            "DescribeTasks" => self.describe_tasks(&req),
            "StopTask" => self.stop_task(&req),
            other => Err(AwsError::unsupported(format!("ECS::{other}"))),
        }
    }
}

impl Ecs {
    fn create_cluster(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("clusterName")
            .and_then(Value::as_str)
            .unwrap_or("default")
            .to_string();
        let cluster = Cluster {
            arn: cluster_arn(&name),
            name: name.clone(),
            status: "ACTIVE".into(),
            created: chrono::Utc::now(),
        };
        let resp = cluster_json(&cluster);
        self.state.write().clusters.insert(name, cluster);
        Ok(json!({ "cluster": resp }))
    }

    fn list_clusters(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let arns: Vec<_> = s.clusters.values().map(|c| c.arn.clone()).collect();
        Ok(json!({ "clusterArns": arns }))
    }

    fn describe_clusters(&self, req: &Value) -> Result<Value, AwsError> {
        let want = req
            .get("clusters")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let want_keys: Vec<String> = if want.is_empty() {
            vec!["default".to_string()]
        } else {
            want.iter()
                .filter_map(|v| v.as_str().map(cluster_lookup_key))
                .collect()
        };
        let s = self.state.read();
        let clusters: Vec<_> = want_keys
            .iter()
            .filter_map(|name| s.clusters.get(name))
            .map(cluster_json)
            .collect();
        Ok(json!({ "clusters": clusters, "failures": [] }))
    }

    fn delete_cluster(&self, req: &Value) -> Result<Value, AwsError> {
        let name = cluster_param(req, "cluster")?;
        self.state
            .write()
            .clusters
            .remove(&name)
            .ok_or_else(|| not_found_cluster(&name))?;
        Ok(json!({}))
    }

    fn register_task_definition(&self, req: &Value) -> Result<Value, AwsError> {
        let family = req
            .get("family")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "family required"))?
            .to_string();
        let container_defs = req
            .get("containerDefinitions")
            .cloned()
            .unwrap_or(Value::Null);
        let network_mode = req
            .get("networkMode")
            .and_then(Value::as_str)
            .map(String::from);
        let cpu = req.get("cpu").and_then(Value::as_str).map(String::from);
        let memory = req.get("memory").and_then(Value::as_str).map(String::from);
        let mut s = self.state.write();
        let entry = s.task_definitions.entry(family.clone()).or_default();
        let revision = entry.last().map_or(1, |t| t.revision + 1);
        let td = TaskDefinition {
            arn: task_def_arn(&family, revision),
            family,
            revision,
            container_definitions: container_defs,
            status: "ACTIVE".into(),
            network_mode,
            cpu,
            memory,
            registered: chrono::Utc::now(),
        };
        let resp = task_def_json(&td);
        entry.push(td);
        Ok(json!({ "taskDefinition": resp }))
    }

    fn list_task_definitions(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let mut arns = Vec::new();
        for td_list in s.task_definitions.values() {
            for td in td_list {
                if td.status == "ACTIVE" {
                    arns.push(td.arn.clone());
                }
            }
        }
        Ok(json!({ "taskDefinitionArns": arns }))
    }

    fn describe_task_definition(&self, req: &Value) -> Result<Value, AwsError> {
        let target = required(req, "taskDefinition")?;
        let s = self.state.read();
        let td = lookup_task_def(&s, &target).ok_or_else(|| not_found(&target))?;
        Ok(json!({ "taskDefinition": task_def_json(td) }))
    }

    fn deregister_task_definition(&self, req: &Value) -> Result<Value, AwsError> {
        let target = required(req, "taskDefinition")?;
        let mut s = self.state.write();
        for td_list in s.task_definitions.values_mut() {
            for td in td_list.iter_mut() {
                if td.arn == target || format!("{}:{}", td.family, td.revision) == target {
                    td.status = "INACTIVE".into();
                    return Ok(json!({ "taskDefinition": task_def_json(td) }));
                }
            }
        }
        Err(not_found(&target))
    }

    fn create_service(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "serviceName")?;
        let cluster = cluster_param(req, "cluster").unwrap_or_else(|_| "default".into());
        let task_definition = required(req, "taskDefinition")?;
        let desired = req.get("desiredCount").and_then(Value::as_i64).unwrap_or(1) as i32;
        let svc = EcsService {
            arn: service_arn(&cluster, &name),
            name: name.clone(),
            cluster_arn: cluster_arn(&cluster),
            task_definition,
            desired_count: desired,
            status: "ACTIVE".into(),
            created: chrono::Utc::now(),
        };
        let resp = service_json(&svc);
        self.state.write().services.insert(name, svc);
        Ok(json!({ "service": resp }))
    }

    fn list_services(&self, req: &Value) -> Result<Value, AwsError> {
        let cluster = cluster_param(req, "cluster").unwrap_or_else(|_| "default".into());
        let cluster_arn_str = cluster_arn(&cluster);
        let s = self.state.read();
        let arns: Vec<_> = s
            .services
            .values()
            .filter(|svc| svc.cluster_arn == cluster_arn_str)
            .map(|svc| svc.arn.clone())
            .collect();
        Ok(json!({ "serviceArns": arns }))
    }

    fn describe_services(&self, req: &Value) -> Result<Value, AwsError> {
        let want = req
            .get("services")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let s = self.state.read();
        let services: Vec<_> = want
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|name_or_arn| {
                s.services
                    .values()
                    .find(|svc| svc.name == name_or_arn || svc.arn == name_or_arn)
            })
            .map(service_json)
            .collect();
        Ok(json!({ "services": services, "failures": [] }))
    }

    fn delete_service(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "service")?;
        self.state.write().services.remove(&name).ok_or_else(|| {
            AwsError::new(
                "ServiceNotFoundException",
                format!("service '{name}' not found"),
            )
        })?;
        Ok(json!({}))
    }

    fn run_task(&self, req: &Value) -> Result<Value, AwsError> {
        let cluster = cluster_param(req, "cluster").unwrap_or_else(|_| "default".into());
        let cluster_arn_str = cluster_arn(&cluster);
        let task_def = required(req, "taskDefinition")?;
        let count = req.get("count").and_then(Value::as_i64).unwrap_or(1) as usize;
        let s = self.state.read();
        let td = lookup_task_def(&s, &task_def).ok_or_else(|| not_found(&task_def))?;
        let td_arn = td.arn.clone();
        drop(s);

        let mut tasks = Vec::with_capacity(count);
        let mut s = self.state.write();
        for _ in 0..count {
            let id = Uuid::new_v4().simple().to_string();
            let arn =
                format!("arn:aws:ecs:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:task/{cluster}/{id}");
            let task = Task {
                cluster_arn: cluster_arn_str.clone(),
                task_definition_arn: td_arn.clone(),
                last_status: "RUNNING".into(),
                desired_status: "RUNNING".into(),
                started: chrono::Utc::now(),
                arn: arn.clone(),
            };
            tasks.push(task_json(&task));
            s.tasks.insert(arn, task);
        }
        Ok(json!({ "tasks": tasks, "failures": [] }))
    }

    fn list_tasks(&self, req: &Value) -> Result<Value, AwsError> {
        let cluster = cluster_param(req, "cluster").unwrap_or_else(|_| "default".into());
        let cluster_arn_str = cluster_arn(&cluster);
        let s = self.state.read();
        let arns: Vec<_> = s
            .tasks
            .values()
            .filter(|t| t.cluster_arn == cluster_arn_str)
            .map(|t| t.arn.clone())
            .collect();
        Ok(json!({ "taskArns": arns }))
    }

    fn describe_tasks(&self, req: &Value) -> Result<Value, AwsError> {
        let want = req
            .get("tasks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let s = self.state.read();
        let tasks: Vec<_> = want
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|arn| s.tasks.get(arn))
            .map(task_json)
            .collect();
        Ok(json!({ "tasks": tasks, "failures": [] }))
    }

    fn stop_task(&self, req: &Value) -> Result<Value, AwsError> {
        let task = required(req, "task")?;
        let mut s = self.state.write();
        let t = s.tasks.get_mut(&task).ok_or_else(|| not_found(&task))?;
        t.last_status = "STOPPED".into();
        t.desired_status = "STOPPED".into();
        Ok(json!({ "task": task_json(t) }))
    }
}

fn cluster_lookup_key(name_or_arn: &str) -> String {
    name_or_arn
        .rsplit('/')
        .next()
        .unwrap_or(name_or_arn)
        .to_string()
}

fn cluster_param(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(cluster_lookup_key)
        .ok_or_else(|| AwsError::new("InvalidParameterException", format!("{key} required")))
}

fn cluster_arn(name: &str) -> String {
    format!("arn:aws:ecs:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:cluster/{name}")
}

fn task_def_arn(family: &str, revision: i32) -> String {
    format!(
        "arn:aws:ecs:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:task-definition/{family}:{revision}"
    )
}

fn service_arn(cluster: &str, service: &str) -> String {
    format!("arn:aws:ecs:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:service/{cluster}/{service}")
}

fn lookup_task_def<'a>(state: &'a State, target: &str) -> Option<&'a TaskDefinition> {
    for td_list in state.task_definitions.values() {
        for td in td_list {
            if td.arn == target {
                return Some(td);
            }
            if format!("{}:{}", td.family, td.revision) == target {
                return Some(td);
            }
        }
    }
    state
        .task_definitions
        .get(target)
        .and_then(|list| list.iter().rfind(|td| td.status == "ACTIVE"))
}

fn cluster_json(c: &Cluster) -> Value {
    json!({
        "clusterName": c.name,
        "clusterArn": c.arn,
        "status": c.status,
        "registeredContainerInstancesCount": 0,
        "runningTasksCount": 0,
        "pendingTasksCount": 0,
        "activeServicesCount": 0,
    })
}

fn task_def_json(t: &TaskDefinition) -> Value {
    json!({
        "taskDefinitionArn": t.arn,
        "family": t.family,
        "revision": t.revision,
        "status": t.status,
        "containerDefinitions": t.container_definitions,
        "networkMode": t.network_mode,
        "cpu": t.cpu,
        "memory": t.memory,
        "registeredAt": t.registered.timestamp(),
    })
}

fn service_json(s: &EcsService) -> Value {
    json!({
        "serviceName": s.name,
        "serviceArn": s.arn,
        "clusterArn": s.cluster_arn,
        "taskDefinition": s.task_definition,
        "desiredCount": s.desired_count,
        "runningCount": s.desired_count,
        "pendingCount": 0,
        "status": s.status,
        "createdAt": s.created.timestamp(),
    })
}

fn task_json(t: &Task) -> Value {
    json!({
        "taskArn": t.arn,
        "clusterArn": t.cluster_arn,
        "taskDefinitionArn": t.task_definition_arn,
        "lastStatus": t.last_status,
        "desiredStatus": t.desired_status,
        "startedAt": t.started.timestamp(),
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidParameterException", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new("ClientException", format!("resource '{name}' not found"))
}

fn not_found_cluster(name: &str) -> AwsError {
    AwsError::new(
        "ClusterNotFoundException",
        format!("cluster '{name}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Ecs::new()));
}
