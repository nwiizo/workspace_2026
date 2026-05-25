//! AWS Cloud Control API — AWS JSON 1.0, target prefix `CloudApiService`.

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

const TARGET_PREFIX: &str = "CloudApiService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    requests: HashMap<String, RequestEntry>,
    resources: HashMap<(String, String), Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RequestEntry {
    token: String,
    type_name: String,
    identifier: String,
    operation: String,
    status: String,
}

pub struct CloudControl {
    state: Arc<RwLock<State>>,
}
impl CloudControl {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for CloudControl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CloudControl {
    fn name(&self) -> &'static str {
        "cloudcontrol"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("cloudcontrol")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("cloudcontrol", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for CloudControl {
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
                .map_err(|e| AwsError::new("ValidationException", e.to_string()))?
        };
        match action {
            "CreateResource" => {
                let type_name = required(&req, "TypeName")?;
                let desired_state: Value = req
                    .get("DesiredState")
                    .and_then(Value::as_str)
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(Value::Null);
                let identifier = format!("resource-{}", Uuid::new_v4().simple());
                let token = Uuid::new_v4().to_string();
                let entry = RequestEntry {
                    token: token.clone(),
                    type_name: type_name.clone(),
                    identifier: identifier.clone(),
                    operation: "CREATE".into(),
                    status: "SUCCESS".into(),
                };
                let mut s = self.state.write();
                s.resources
                    .insert((type_name.clone(), identifier.clone()), desired_state);
                s.requests.insert(token.clone(), entry.clone());
                Ok(json!({ "ProgressEvent": event_json(&entry) }))
            }
            "GetResource" => {
                let type_name = required(&req, "TypeName")?;
                let identifier = required(&req, "Identifier")?;
                let s = self.state.read();
                let props = s
                    .resources
                    .get(&(type_name.clone(), identifier.clone()))
                    .ok_or_else(|| {
                        AwsError::new(
                            "ResourceNotFoundException",
                            format!("{type_name}:{identifier} not found"),
                        )
                    })?;
                Ok(json!({
                    "TypeName": type_name,
                    "ResourceDescription": {
                        "Identifier": identifier,
                        "Properties": serde_json::to_string(props).unwrap_or_default(),
                    },
                }))
            }
            "ListResources" => {
                let type_name = required(&req, "TypeName")?;
                let s = self.state.read();
                let items: Vec<Value> = s
                    .resources
                    .iter()
                    .filter(|((tn, _), _)| *tn == type_name)
                    .map(|((_, id), props)| {
                        json!({
                            "Identifier": id,
                            "Properties": serde_json::to_string(props).unwrap_or_default(),
                        })
                    })
                    .collect();
                Ok(json!({
                    "TypeName": type_name,
                    "ResourceDescriptions": items,
                }))
            }
            "DeleteResource" => {
                let type_name = required(&req, "TypeName")?;
                let identifier = required(&req, "Identifier")?;
                let mut s = self.state.write();
                let removed = s
                    .resources
                    .remove(&(type_name.clone(), identifier.clone()))
                    .is_some();
                if !removed {
                    return Err(AwsError::new(
                        "ResourceNotFoundException",
                        format!("{type_name}:{identifier} not found"),
                    ));
                }
                let token = Uuid::new_v4().to_string();
                let entry = RequestEntry {
                    token: token.clone(),
                    type_name,
                    identifier,
                    operation: "DELETE".into(),
                    status: "SUCCESS".into(),
                };
                s.requests.insert(token, entry.clone());
                Ok(json!({ "ProgressEvent": event_json(&entry) }))
            }
            "GetResourceRequestStatus" => {
                let token = required(&req, "RequestToken")?;
                let s = self.state.read();
                let entry = s.requests.get(&token).ok_or_else(|| {
                    AwsError::new("RequestTokenNotFoundException", "token not found")
                })?;
                Ok(json!({ "ProgressEvent": event_json(entry) }))
            }
            other => Err(AwsError::unsupported(format!("CloudControl::{other}"))),
        }
    }
}

fn event_json(e: &RequestEntry) -> Value {
    json!({
        "RequestToken": e.token,
        "TypeName": e.type_name,
        "Identifier": e.identifier,
        "Operation": e.operation,
        "OperationStatus": e.status,
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", format!("{key} required")))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(CloudControl::new()));
}
