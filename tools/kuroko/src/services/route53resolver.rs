//! Route 53 Resolver — AWS JSON 1.1, target prefix `Route53Resolver`.

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

const TARGET_PREFIX: &str = "Route53Resolver";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    endpoints: HashMap<String, ResolverEndpoint>,
    rules: HashMap<String, ResolverRule>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ResolverEndpoint {
    id: String,
    arn: String,
    name: String,
    direction: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ResolverRule {
    id: String,
    arn: String,
    name: Option<String>,
    domain_name: String,
    rule_type: String,
    status: String,
}

pub struct Route53Resolver {
    state: Arc<RwLock<State>>,
}
impl Route53Resolver {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Route53Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Route53Resolver {
    fn name(&self) -> &'static str {
        "route53resolver"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("route53resolver")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("route53resolver", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Route53Resolver {
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
            "CreateResolverEndpoint" => {
                let direction = required(&req, "Direction")?;
                let name = req
                    .get("Name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let id = format!("rslvr-{}", Uuid::new_v4().simple());
                let arn = format!(
                    "arn:aws:route53resolver:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:resolver-endpoint/{id}"
                );
                let ep = ResolverEndpoint {
                    id: id.clone(),
                    arn: arn.clone(),
                    name,
                    direction,
                    status: "OPERATIONAL".into(),
                };
                let resp = endpoint_json(&ep);
                self.state.write().endpoints.insert(id, ep);
                Ok(json!({ "ResolverEndpoint": resp }))
            }
            "GetResolverEndpoint" => {
                let id = required(&req, "ResolverEndpointId")?;
                let s = self.state.read();
                let ep = s.endpoints.get(&id).ok_or_else(|| not_found(&id))?;
                Ok(json!({ "ResolverEndpoint": endpoint_json(ep) }))
            }
            "ListResolverEndpoints" => {
                let s = self.state.read();
                let eps: Vec<_> = s.endpoints.values().map(endpoint_json).collect();
                Ok(json!({ "ResolverEndpoints": eps }))
            }
            "DeleteResolverEndpoint" => {
                let id = required(&req, "ResolverEndpointId")?;
                let mut s = self.state.write();
                let ep = s.endpoints.remove(&id).ok_or_else(|| not_found(&id))?;
                Ok(json!({ "ResolverEndpoint": endpoint_json(&ep) }))
            }
            "CreateResolverRule" => {
                let domain_name = required(&req, "DomainName")?;
                let rule_type = required(&req, "RuleType")?;
                let name = req.get("Name").and_then(Value::as_str).map(String::from);
                let id = format!("rslvr-rr-{}", Uuid::new_v4().simple());
                let arn = format!(
                    "arn:aws:route53resolver:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:resolver-rule/{id}"
                );
                let rule = ResolverRule {
                    id: id.clone(),
                    arn: arn.clone(),
                    name,
                    domain_name,
                    rule_type,
                    status: "COMPLETE".into(),
                };
                let resp = rule_json(&rule);
                self.state.write().rules.insert(id, rule);
                Ok(json!({ "ResolverRule": resp }))
            }
            "ListResolverRules" => {
                let s = self.state.read();
                let rules: Vec<_> = s.rules.values().map(rule_json).collect();
                Ok(json!({ "ResolverRules": rules }))
            }
            other => Err(AwsError::unsupported(format!("Route53Resolver::{other}"))),
        }
    }
}

fn endpoint_json(ep: &ResolverEndpoint) -> Value {
    json!({
        "Id": ep.id,
        "Arn": ep.arn,
        "Name": ep.name,
        "Direction": ep.direction,
        "Status": ep.status,
    })
}

fn rule_json(r: &ResolverRule) -> Value {
    json!({
        "Id": r.id,
        "Arn": r.arn,
        "Name": r.name,
        "DomainName": r.domain_name,
        "RuleType": r.rule_type,
        "Status": r.status,
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidParameterException", format!("{key} required")))
}

fn not_found(id: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("resource '{id}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Route53Resolver::new()));
}
