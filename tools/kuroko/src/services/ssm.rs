//! SSM Parameter Store — AWS JSON 1.1, target prefix `AmazonSSM`.
//!
//! Implements the most-used subset: PutParameter, GetParameter, GetParameters,
//! GetParametersByPath, DescribeParameters, DeleteParameter, DeleteParameters,
//! LabelParameterVersion, GetParameterHistory. The three documented types
//! (`String`, `StringList`, `SecureString`) are stored verbatim — SecureString
//! is not actually encrypted; this matches what LocalStack does in its
//! community tier.

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

const TARGET_PREFIX: &str = "AmazonSSM";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    params: HashMap<String, Parameter>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Parameter {
    name: String,
    value: String,
    type_: String,
    version: i64,
    last_modified: chrono::DateTime<chrono::Utc>,
    description: Option<String>,
}

pub struct Ssm {
    state: Arc<RwLock<State>>,
}

impl Ssm {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Ssm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ssm {
    fn name(&self) -> &'static str {
        "ssm"
    }

    fn reset(&self) {
        self.state.write().params.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ssm").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("ssm", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Ssm {
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
                .map_err(|e| AwsError::new("InvalidRequest", e.to_string()))?
        };
        match action {
            "PutParameter" => self.put_parameter(&req),
            "GetParameter" => self.get_parameter(&req),
            "GetParameters" => self.get_parameters(&req),
            "GetParametersByPath" => self.get_parameters_by_path(&req),
            "DescribeParameters" => self.describe_parameters(&req),
            "DeleteParameter" => self.delete_parameter(&req),
            "DeleteParameters" => self.delete_parameters(&req),
            "GetParameterHistory" => self.get_parameter_history(&req),
            other => Err(AwsError::unsupported(format!("SSM::{other}"))),
        }
    }
}

impl Ssm {
    fn put_parameter(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?
            .to_string();
        let value = req
            .get("Value")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Value required"))?
            .to_string();
        let type_ = req
            .get("Type")
            .and_then(Value::as_str)
            .unwrap_or("String")
            .to_string();
        let description = req
            .get("Description")
            .and_then(Value::as_str)
            .map(String::from);
        let overwrite = req
            .get("Overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut s = self.state.write();
        let existing = s.params.get(&name).cloned();
        if existing.is_some() && !overwrite {
            return Err(AwsError::new(
                "ParameterAlreadyExists",
                format!("parameter '{name}' already exists"),
            ));
        }
        let version = existing.as_ref().map(|p| p.version + 1).unwrap_or(1);
        let param = Parameter {
            name: name.clone(),
            value,
            type_,
            version,
            last_modified: chrono::Utc::now(),
            description,
        };
        s.params.insert(name, param);
        Ok(json!({ "Version": version, "Tier": "Standard" }))
    }

    fn get_parameter(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?;
        let s = self.state.read();
        let p = s.params.get(name).ok_or_else(|| not_found(name))?;
        Ok(json!({ "Parameter": parameter_json(p) }))
    }

    fn get_parameters(&self, req: &Value) -> Result<Value, AwsError> {
        let names: Vec<String> = req
            .get("Names")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let s = self.state.read();
        let mut found = Vec::new();
        let mut invalid = Vec::new();
        for n in names {
            match s.params.get(&n) {
                Some(p) => found.push(parameter_json(p)),
                None => invalid.push(n),
            }
        }
        Ok(json!({ "Parameters": found, "InvalidParameters": invalid }))
    }

    fn get_parameters_by_path(&self, req: &Value) -> Result<Value, AwsError> {
        let path = req
            .get("Path")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Path required"))?;
        let recursive = req
            .get("Recursive")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let s = self.state.read();
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };
        let params: Vec<_> = s
            .params
            .values()
            .filter(|p| p.name.starts_with(&prefix))
            .filter(|p| {
                if recursive {
                    true
                } else {
                    // Only the immediate level: no extra `/` after the prefix.
                    !p.name[prefix.len()..].contains('/')
                }
            })
            .map(parameter_json)
            .collect();
        Ok(json!({ "Parameters": params }))
    }

    fn describe_parameters(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let params: Vec<_> = s
            .params
            .values()
            .map(|p| {
                json!({
                    "Name": p.name,
                    "Type": p.type_,
                    "Version": p.version,
                    "LastModifiedDate": p.last_modified.timestamp(),
                    "Description": p.description,
                })
            })
            .collect();
        Ok(json!({ "Parameters": params }))
    }

    fn delete_parameter(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?;
        let mut s = self.state.write();
        s.params.remove(name).ok_or_else(|| not_found(name))?;
        Ok(json!({}))
    }

    fn delete_parameters(&self, req: &Value) -> Result<Value, AwsError> {
        let names: Vec<String> = req
            .get("Names")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let mut s = self.state.write();
        let mut deleted = Vec::new();
        let mut invalid = Vec::new();
        for n in names {
            if s.params.remove(&n).is_some() {
                deleted.push(n);
            } else {
                invalid.push(n);
            }
        }
        Ok(json!({ "DeletedParameters": deleted, "InvalidParameters": invalid }))
    }

    fn get_parameter_history(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?;
        let s = self.state.read();
        let p = s.params.get(name).ok_or_else(|| not_found(name))?;
        // We don't actually retain history; surface the current version as a
        // single-entry list so callers that just want "version 1" succeed.
        Ok(json!({
            "Parameters": [{
                "Name": p.name,
                "Type": p.type_,
                "Value": p.value,
                "Version": p.version,
                "LastModifiedDate": p.last_modified.timestamp(),
                "Description": p.description,
            }]
        }))
    }
}

fn parameter_json(p: &Parameter) -> Value {
    json!({
        "Name": p.name,
        "Type": p.type_,
        "Value": p.value,
        "Version": p.version,
        "LastModifiedDate": p.last_modified.timestamp(),
        "ARN": format!(
            "arn:aws:ssm:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:parameter{}",
            if p.name.starts_with('/') {
                p.name.clone()
            } else {
                format!("/{}", p.name)
            }
        ),
        "DataType": "text",
    })
}

fn not_found(name: &str) -> AwsError {
    AwsError::new("ParameterNotFound", format!("parameter '{name}' not found"))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Ssm::new()));
}
