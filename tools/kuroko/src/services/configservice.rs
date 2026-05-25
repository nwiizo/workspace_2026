//! AWS Config (configservice) — AWS JSON 1.1, target prefix `StarlingDoveService`.

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

const TARGET_PREFIX: &str = "StarlingDoveService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    rules: HashMap<String, Value>,
    recorders: HashMap<String, Value>,
}

pub struct ConfigSvc {
    state: Arc<RwLock<State>>,
}
impl ConfigSvc {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for ConfigSvc {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ConfigSvc {
    fn name(&self) -> &'static str {
        "configservice"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("configservice")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("configservice", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for ConfigSvc {
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
                .map_err(|e| AwsError::new("InvalidParameterValueException", e.to_string()))?
        };
        match action {
            "PutConfigRule" => {
                let rule = req
                    .get("ConfigRule")
                    .cloned()
                    .ok_or_else(|| invalid("ConfigRule required"))?;
                let name = rule
                    .get("ConfigRuleName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("ConfigRuleName required"))?
                    .to_string();
                let mut filled = rule.clone();
                let obj = filled.as_object_mut().unwrap();
                obj.entry("ConfigRuleArn".to_string()).or_insert_with(|| {
                    json!(format!(
                        "arn:aws:config:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:config-rule/{name}"
                    ))
                });
                obj.entry("ConfigRuleId".to_string())
                    .or_insert_with(|| json!(format!("config-rule-{name}")));
                obj.entry("ConfigRuleState".to_string())
                    .or_insert_with(|| json!("ACTIVE"));
                self.state.write().rules.insert(name, filled);
                Ok(json!({}))
            }
            "DescribeConfigRules" => {
                let names: Option<Vec<String>> = req
                    .get("ConfigRuleNames")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                let s = self.state.read();
                let rules: Vec<Value> = s
                    .rules
                    .iter()
                    .filter(|(n, _)| names.as_ref().is_none_or(|ns| ns.contains(n)))
                    .map(|(_, r)| r.clone())
                    .collect();
                Ok(json!({ "ConfigRules": rules }))
            }
            "DeleteConfigRule" => {
                let name = req
                    .get("ConfigRuleName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("ConfigRuleName required"))?
                    .to_string();
                if self.state.write().rules.remove(&name).is_none() {
                    return Err(AwsError::new(
                        "NoSuchConfigRuleException",
                        format!("rule '{name}' not found"),
                    ));
                }
                Ok(json!({}))
            }
            "PutConfigurationRecorder" => {
                let rec = req
                    .get("ConfigurationRecorder")
                    .cloned()
                    .ok_or_else(|| invalid("ConfigurationRecorder required"))?;
                let name = rec
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .to_string();
                self.state.write().recorders.insert(name, rec);
                Ok(json!({}))
            }
            "DescribeConfigurationRecorders" => {
                let s = self.state.read();
                let recs: Vec<Value> = s.recorders.values().cloned().collect();
                Ok(json!({ "ConfigurationRecorders": recs }))
            }
            other => Err(AwsError::unsupported(format!("ConfigService::{other}"))),
        }
    }
}

fn invalid(msg: &str) -> AwsError {
    AwsError::new("InvalidParameterValueException", msg.to_string())
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(ConfigSvc::new()));
}
