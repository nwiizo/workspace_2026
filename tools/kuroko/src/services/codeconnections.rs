//! AWS CodeConnections — AWS JSON 1.0, target prefix `CodeStar_connections_20191201`.

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

const TARGET_PREFIX: &str = "CodeConnections_20231201";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    connections: HashMap<String, Connection>,
    hosts: HashMap<String, Host>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Connection {
    arn: String,
    name: String,
    provider_type: String,
    status: String,
    host_arn: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Host {
    arn: String,
    name: String,
    provider_type: String,
    provider_endpoint: String,
    status: String,
}

pub struct CodeConnections {
    state: Arc<RwLock<State>>,
}
impl CodeConnections {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for CodeConnections {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CodeConnections {
    fn name(&self) -> &'static str {
        "codeconnections"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("codeconnections")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("codeconnections", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for CodeConnections {
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
                .map_err(|e| AwsError::new("InvalidInputException", e.to_string()))?
        };
        match action {
            "CreateConnection" => {
                let name = required(&req, "ConnectionName")?;
                let provider_type = req
                    .get("ProviderType")
                    .and_then(Value::as_str)
                    .unwrap_or("GitHub")
                    .to_string();
                let host_arn = req.get("HostArn").and_then(Value::as_str).map(String::from);
                let arn = connection_arn(&Uuid::new_v4().to_string());
                let conn = Connection {
                    arn: arn.clone(),
                    name,
                    provider_type,
                    status: "PENDING".into(),
                    host_arn,
                };
                self.state.write().connections.insert(arn.clone(), conn);
                Ok(json!({ "ConnectionArn": arn }))
            }
            "GetConnection" => {
                let arn = required(&req, "ConnectionArn")?;
                let s = self.state.read();
                let c = s
                    .connections
                    .get(&arn)
                    .ok_or_else(|| AwsError::new("ResourceNotFoundException", "not found"))?;
                Ok(json!({ "Connection": connection_json(c) }))
            }
            "ListConnections" => {
                let s = self.state.read();
                let list: Vec<Value> = s.connections.values().map(connection_json).collect();
                Ok(json!({ "Connections": list }))
            }
            "DeleteConnection" => {
                let arn = required(&req, "ConnectionArn")?;
                self.state.write().connections.remove(&arn);
                Ok(json!({}))
            }
            "CreateHost" => {
                let name = required(&req, "Name")?;
                let provider_type = required(&req, "ProviderType")?;
                let provider_endpoint = required(&req, "ProviderEndpoint")?;
                let arn = host_arn(&Uuid::new_v4().to_string());
                let host = Host {
                    arn: arn.clone(),
                    name,
                    provider_type,
                    provider_endpoint,
                    status: "AVAILABLE".into(),
                };
                self.state.write().hosts.insert(arn.clone(), host);
                Ok(json!({ "HostArn": arn }))
            }
            "ListHosts" => {
                let s = self.state.read();
                let list: Vec<Value> = s.hosts.values().map(host_json).collect();
                Ok(json!({ "Hosts": list }))
            }
            "GetHost" => {
                let arn = required(&req, "HostArn")?;
                let s = self.state.read();
                let h = s
                    .hosts
                    .get(&arn)
                    .ok_or_else(|| AwsError::new("ResourceNotFoundException", "not found"))?;
                Ok(json!({
                    "Name": h.name,
                    "Status": h.status,
                    "ProviderType": h.provider_type,
                    "ProviderEndpoint": h.provider_endpoint,
                }))
            }
            "DeleteHost" => {
                let arn = required(&req, "HostArn")?;
                self.state.write().hosts.remove(&arn);
                Ok(json!({}))
            }
            other => Err(AwsError::unsupported(format!("CodeConnections::{other}"))),
        }
    }
}

fn connection_arn(id: &str) -> String {
    format!("arn:aws:codeconnections:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:connection/{id}")
}
fn host_arn(id: &str) -> String {
    format!("arn:aws:codeconnections:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:host/{id}")
}

fn connection_json(c: &Connection) -> Value {
    json!({
        "ConnectionArn": c.arn,
        "ConnectionName": c.name,
        "ProviderType": c.provider_type,
        "ConnectionStatus": c.status,
        "OwnerAccountId": EMULATED_ACCOUNT_ID,
        "HostArn": c.host_arn,
    })
}

fn host_json(h: &Host) -> Value {
    json!({
        "HostArn": h.arn,
        "Name": h.name,
        "ProviderType": h.provider_type,
        "ProviderEndpoint": h.provider_endpoint,
        "Status": h.status,
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidInputException", format!("{key} required")))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(CodeConnections::new()));
}
