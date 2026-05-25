//! AWS Directory Service (ds) — AWS JSON 1.1, target prefix `DirectoryService_20150416`.

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

const TARGET_PREFIX: &str = "DirectoryService_20150416";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    directories: HashMap<String, Directory>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Directory {
    id: String,
    name: String,
    short_name: Option<String>,
    size: String,
    directory_type: String,
    stage: String,
    description: Option<String>,
}

pub struct Ds {
    state: Arc<RwLock<State>>,
}
impl Ds {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Ds {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ds {
    fn name(&self) -> &'static str {
        "ds"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ds").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("ds", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Ds {
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
            "CreateDirectory" | "CreateMicrosoftAD" | "ConnectDirectory" => {
                let name = required(&req, "Name")?;
                let size = req
                    .get("Size")
                    .and_then(Value::as_str)
                    .unwrap_or("Small")
                    .to_string();
                let short = req
                    .get("ShortName")
                    .and_then(Value::as_str)
                    .map(String::from);
                let desc = req
                    .get("Description")
                    .and_then(Value::as_str)
                    .map(String::from);
                let directory_type = match action {
                    "CreateMicrosoftAD" => "MicrosoftAD",
                    "ConnectDirectory" => "ADConnector",
                    _ => "SimpleAD",
                }
                .to_string();
                let id = format!("d-{}", &Uuid::new_v4().simple().to_string()[..10]);
                let dir = Directory {
                    id: id.clone(),
                    name,
                    short_name: short,
                    size,
                    directory_type,
                    stage: "Active".into(),
                    description: desc,
                };
                self.state.write().directories.insert(id.clone(), dir);
                Ok(json!({ "DirectoryId": id }))
            }
            "DescribeDirectories" => {
                let s = self.state.read();
                let ids: Option<Vec<String>> =
                    req.get("DirectoryIds").and_then(Value::as_array).map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                let list: Vec<_> = s
                    .directories
                    .values()
                    .filter(|d| ids.as_ref().is_none_or(|ii| ii.contains(&d.id)))
                    .map(directory_json)
                    .collect();
                Ok(json!({ "DirectoryDescriptions": list }))
            }
            "DeleteDirectory" => {
                let id = required(&req, "DirectoryId")?;
                let mut s = self.state.write();
                if s.directories.remove(&id).is_none() {
                    return Err(AwsError::new(
                        "EntityDoesNotExistException",
                        format!("directory '{id}' not found"),
                    ));
                }
                Ok(json!({ "DirectoryId": id }))
            }
            other => Err(AwsError::unsupported(format!("DirectoryService::{other}"))),
        }
    }
}

fn directory_json(d: &Directory) -> Value {
    json!({
        "DirectoryId": d.id,
        "Name": d.name,
        "ShortName": d.short_name,
        "Size": d.size,
        "Type": d.directory_type,
        "Stage": d.stage,
        "Description": d.description,
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidParameterException", format!("{key} required")))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Ds::new()));
}
