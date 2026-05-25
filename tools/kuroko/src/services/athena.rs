//! Athena — AWS JSON 1.1, target prefix `AmazonAthena`.
//!
//! Query and workgroup metadata. kuroko does not execute SQL; every
//! StartQueryExecution transitions to SUCCEEDED immediately so polling
//! returns the terminal state on the first call. GetQueryResults returns
//! an empty result set.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{JsonProtocolService, Service, ServiceContext, persistence_error};

const TARGET_PREFIX: &str = "AmazonAthena";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    executions: HashMap<String, QueryExecution>,
    workgroups: HashMap<String, WorkGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QueryExecution {
    id: String,
    query: String,
    database: Option<String>,
    workgroup: String,
    state: String,
    submitted: chrono::DateTime<chrono::Utc>,
    completed: chrono::DateTime<chrono::Utc>,
    output_location: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WorkGroup {
    name: String,
    description: Option<String>,
    state: String,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Athena {
    state: Arc<RwLock<State>>,
}

impl Athena {
    pub fn new() -> Self {
        let mut s = State::default();
        s.workgroups.insert(
            "primary".into(),
            WorkGroup {
                name: "primary".into(),
                description: Some("default workgroup".into()),
                state: "ENABLED".into(),
                created: chrono::Utc::now(),
            },
        );
        Self {
            state: Arc::new(RwLock::new(s)),
        }
    }
}

impl Default for Athena {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Athena {
    fn name(&self) -> &'static str {
        "athena"
    }
    fn reset(&self) {
        let mut s = self.state.write();
        *s = State::default();
        s.workgroups.insert(
            "primary".into(),
            WorkGroup {
                name: "primary".into(),
                description: Some("default workgroup".into()),
                state: "ENABLED".into(),
                created: chrono::Utc::now(),
            },
        );
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("athena").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("athena", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Athena {
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
                .map_err(|e| AwsError::new("InvalidRequestException", e.to_string()))?
        };
        match action {
            "StartQueryExecution" => self.start_query_execution(&req),
            "GetQueryExecution" => self.get_query_execution(&req),
            "GetQueryResults" => self.get_query_results(&req),
            "StopQueryExecution" => self.stop_query_execution(&req),
            "ListQueryExecutions" => self.list_query_executions(&req),
            "CreateWorkGroup" => self.create_workgroup(&req),
            "ListWorkGroups" => self.list_workgroups(),
            "GetWorkGroup" => self.get_workgroup(&req),
            "DeleteWorkGroup" => self.delete_workgroup(&req),
            other => Err(AwsError::unsupported(format!("Athena::{other}"))),
        }
    }
}

impl Athena {
    fn start_query_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let query = req
            .get("QueryString")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidRequestException", "QueryString required"))?
            .to_string();
        let database = req
            .get("QueryExecutionContext")
            .and_then(|c| c.get("Database"))
            .and_then(Value::as_str)
            .map(String::from);
        let workgroup = req
            .get("WorkGroup")
            .and_then(Value::as_str)
            .unwrap_or("primary")
            .to_string();
        let output_location = req
            .get("ResultConfiguration")
            .and_then(|c| c.get("OutputLocation"))
            .and_then(Value::as_str)
            .unwrap_or("s3://kuroko-athena-results/")
            .to_string();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let exec = QueryExecution {
            id: id.clone(),
            query,
            database,
            workgroup,
            // Immediate SUCCEEDED — no actual query engine in kuroko.
            state: "SUCCEEDED".into(),
            submitted: now,
            completed: now,
            output_location,
        };
        self.state.write().executions.insert(id.clone(), exec);
        Ok(json!({ "QueryExecutionId": id }))
    }

    fn get_query_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let id = required(req, "QueryExecutionId")?;
        let s = self.state.read();
        let exec = s.executions.get(&id).ok_or_else(|| not_found(&id))?;
        Ok(json!({
            "QueryExecution": {
                "QueryExecutionId": exec.id,
                "Query": exec.query,
                "QueryExecutionContext": { "Database": exec.database },
                "WorkGroup": exec.workgroup,
                "Status": {
                    "State": exec.state,
                    "SubmissionDateTime": exec.submitted.timestamp(),
                    "CompletionDateTime": exec.completed.timestamp(),
                },
                "ResultConfiguration": { "OutputLocation": exec.output_location },
                "Statistics": {
                    "EngineExecutionTimeInMillis": 0,
                    "DataScannedInBytes": 0,
                },
            }
        }))
    }

    fn get_query_results(&self, req: &Value) -> Result<Value, AwsError> {
        let id = required(req, "QueryExecutionId")?;
        let s = self.state.read();
        s.executions.get(&id).ok_or_else(|| not_found(&id))?;
        // Empty result set; AWS shape with a "header row only" ResultSet.
        Ok(json!({
            "UpdateCount": 0,
            "ResultSet": {
                "Rows": [],
                "ResultSetMetadata": { "ColumnInfo": [] },
            }
        }))
    }

    fn stop_query_execution(&self, req: &Value) -> Result<Value, AwsError> {
        let id = required(req, "QueryExecutionId")?;
        let mut s = self.state.write();
        let exec = s.executions.get_mut(&id).ok_or_else(|| not_found(&id))?;
        if exec.state == "RUNNING" {
            exec.state = "CANCELLED".into();
        }
        Ok(json!({}))
    }

    fn list_query_executions(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let ids: Vec<_> = s.executions.keys().cloned().collect();
        Ok(json!({ "QueryExecutionIds": ids }))
    }

    fn create_workgroup(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let description = req
            .get("Description")
            .and_then(Value::as_str)
            .map(String::from);
        let mut s = self.state.write();
        if s.workgroups.contains_key(&name) {
            return Err(AwsError::new(
                "InvalidRequestException",
                format!("workgroup '{name}' already exists"),
            ));
        }
        s.workgroups.insert(
            name.clone(),
            WorkGroup {
                name,
                description,
                state: "ENABLED".into(),
                created: chrono::Utc::now(),
            },
        );
        Ok(json!({}))
    }

    fn list_workgroups(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let workgroups: Vec<_> = s
            .workgroups
            .values()
            .map(|w| {
                json!({
                    "Name": w.name,
                    "State": w.state,
                    "Description": w.description,
                    "CreationTime": w.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "WorkGroups": workgroups }))
    }

    fn get_workgroup(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "WorkGroup")?;
        let s = self.state.read();
        let w = s.workgroups.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "WorkGroup": {
                "Name": w.name,
                "State": w.state,
                "Description": w.description,
                "CreationTime": w.created.timestamp(),
                "Configuration": {},
            }
        }))
    }

    fn delete_workgroup(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "WorkGroup")?;
        if name == "primary" {
            return Err(AwsError::new(
                "InvalidRequestException",
                "cannot delete the primary workgroup",
            ));
        }
        self.state.write().workgroups.remove(&name);
        Ok(json!({}))
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidRequestException", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "InvalidRequestException",
        format!("resource '{name}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Athena::new()));
}
