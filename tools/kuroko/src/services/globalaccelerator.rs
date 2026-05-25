//! Global Accelerator — AWS JSON 1.1, target prefix `GlobalAccelerator_V20180706`.

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
    EMULATED_ACCOUNT_ID, JsonProtocolService, Service, ServiceContext, persistence_error,
};

const TARGET_PREFIX: &str = "GlobalAccelerator_V20180706";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    accelerators: HashMap<String, Accelerator>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Accelerator {
    name: String,
    arn: String,
    status: String,
    enabled: bool,
}

pub struct GlobalAccelerator {
    state: Arc<RwLock<State>>,
}
impl GlobalAccelerator {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for GlobalAccelerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for GlobalAccelerator {
    fn name(&self) -> &'static str {
        "globalaccelerator"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("globalaccelerator")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("globalaccelerator", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for GlobalAccelerator {
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
                .map_err(|e| AwsError::new("InvalidArgumentException", e.to_string()))?
        };
        match action {
            "CreateAccelerator" => {
                let name = required(&req, "Name")?;
                let id = Uuid::new_v4().to_string();
                let arn =
                    format!("arn:aws:globalaccelerator::{EMULATED_ACCOUNT_ID}:accelerator/{id}");
                let a = Accelerator {
                    name,
                    arn: arn.clone(),
                    status: "IN_PROGRESS".into(),
                    enabled: true,
                };
                let resp = json!({ "Accelerator": acc_json(&a) });
                self.state.write().accelerators.insert(arn, a);
                Ok(resp)
            }
            "DescribeAccelerator" => {
                let arn = required(&req, "AcceleratorArn")?;
                let s = self.state.read();
                let a = s.accelerators.get(&arn).ok_or_else(|| not_found(&arn))?;
                Ok(json!({ "Accelerator": acc_json(a) }))
            }
            "ListAccelerators" => {
                let s = self.state.read();
                let list: Vec<_> = s.accelerators.values().map(acc_json).collect();
                Ok(json!({ "Accelerators": list }))
            }
            "DeleteAccelerator" => {
                let arn = required(&req, "AcceleratorArn")?;
                self.state.write().accelerators.remove(&arn);
                Ok(json!({}))
            }
            other => Err(AwsError::unsupported(format!("GlobalAccelerator::{other}"))),
        }
    }
}

fn acc_json(a: &Accelerator) -> Value {
    json!({
        "AcceleratorArn": a.arn,
        "Name": a.name,
        "Status": a.status,
        "Enabled": a.enabled,
        "IpAddressType": "IPV4",
        "DnsName": format!("{}.awsglobalaccelerator.com", a.name),
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidArgumentException", format!("{key} required")))
}

fn not_found(arn: &str) -> AwsError {
    AwsError::new(
        "AcceleratorNotFoundException",
        format!("accelerator '{arn}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(GlobalAccelerator::new()));
}
