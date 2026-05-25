//! Amazon GameLift — AWS JSON 1.1, target prefix `GameLift`.

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
    CborProtocolService, EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service,
    ServiceContext, persistence_error,
};

const TARGET_PREFIX: &str = "GameLift";
const CBOR_SERVICE: &str = "GameLift";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    fleets: HashMap<String, Fleet>,
    builds: HashMap<String, Build>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Fleet {
    id: String,
    arn: String,
    name: String,
    status: String,
    build_id: Option<String>,
    instance_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Build {
    id: String,
    arn: String,
    name: String,
    version: String,
    status: String,
    operating_system: String,
}

pub struct GameLift {
    state: Arc<RwLock<State>>,
}
impl GameLift {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for GameLift {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for GameLift {
    fn name(&self) -> &'static str {
        "gamelift"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("gamelift").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("gamelift", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for GameLift {
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
            "CreateBuild" => {
                let name = req
                    .get("Name")
                    .and_then(Value::as_str)
                    .unwrap_or("build")
                    .to_string();
                let version = req
                    .get("Version")
                    .and_then(Value::as_str)
                    .unwrap_or("1.0.0")
                    .to_string();
                let os = req
                    .get("OperatingSystem")
                    .and_then(Value::as_str)
                    .unwrap_or("AMAZON_LINUX_2")
                    .to_string();
                let id = format!("build-{}", Uuid::new_v4());
                let arn =
                    format!("arn:aws:gamelift:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:build/{id}");
                let b = Build {
                    id: id.clone(),
                    arn,
                    name,
                    version,
                    status: "READY".into(),
                    operating_system: os,
                };
                let body = build_json(&b);
                self.state.write().builds.insert(id, b);
                Ok(json!({ "Build": body }))
            }
            "DescribeBuild" => {
                let id = required(&req, "BuildId")?;
                let s = self.state.read();
                let b = s
                    .builds
                    .get(&id)
                    .ok_or_else(|| AwsError::new("NotFoundException", "build not found"))?;
                Ok(json!({ "Build": build_json(b) }))
            }
            "ListBuilds" => {
                let s = self.state.read();
                let builds: Vec<Value> = s.builds.values().map(build_json).collect();
                Ok(json!({ "Builds": builds }))
            }
            "DeleteBuild" => {
                let id = required(&req, "BuildId")?;
                self.state.write().builds.remove(&id);
                Ok(json!({}))
            }
            "CreateFleet" => {
                let name = required(&req, "Name")?;
                let instance_type = req
                    .get("EC2InstanceType")
                    .and_then(Value::as_str)
                    .unwrap_or("c5.large")
                    .to_string();
                let build_id = req.get("BuildId").and_then(Value::as_str).map(String::from);
                let id = format!("fleet-{}", Uuid::new_v4());
                let arn =
                    format!("arn:aws:gamelift:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:fleet/{id}");
                let f = Fleet {
                    id: id.clone(),
                    arn,
                    name,
                    status: "ACTIVE".into(),
                    build_id,
                    instance_type,
                };
                let body = fleet_json(&f);
                self.state.write().fleets.insert(id, f);
                Ok(json!({ "FleetAttributes": body }))
            }
            "DescribeFleetAttributes" => {
                let s = self.state.read();
                let ids: Option<Vec<String>> =
                    req.get("FleetIds").and_then(Value::as_array).map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                let list: Vec<Value> = s
                    .fleets
                    .values()
                    .filter(|f| ids.as_ref().is_none_or(|i| i.contains(&f.id)))
                    .map(fleet_json)
                    .collect();
                Ok(json!({ "FleetAttributes": list }))
            }
            "ListFleets" => {
                let s = self.state.read();
                let ids: Vec<String> = s.fleets.keys().cloned().collect();
                Ok(json!({ "FleetIds": ids }))
            }
            "DeleteFleet" => {
                let id = required(&req, "FleetId")?;
                self.state.write().fleets.remove(&id);
                Ok(json!({}))
            }
            other => Err(AwsError::unsupported(format!("GameLift::{other}"))),
        }
    }
}

fn build_json(b: &Build) -> Value {
    json!({
        "BuildId": b.id,
        "BuildArn": b.arn,
        "Name": b.name,
        "Version": b.version,
        "Status": b.status,
        "OperatingSystem": b.operating_system,
    })
}

fn fleet_json(f: &Fleet) -> Value {
    json!({
        "FleetId": f.id,
        "FleetArn": f.arn,
        "Name": f.name,
        "Status": f.status,
        "BuildId": f.build_id,
        "InstanceType": f.instance_type,
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidRequestException", format!("{key} required")))
}

#[async_trait]
impl CborProtocolService for GameLift {
    fn smithy_service(&self) -> &'static str {
        CBOR_SERVICE
    }
    async fn dispatch(
        &self,
        ctx: ServiceContext,
        operation: &str,
        body: Bytes,
    ) -> Result<Bytes, AwsError> {
        let req: Value = if body.is_empty() {
            json!({})
        } else {
            let cbor: ciborium::value::Value = ciborium::de::from_reader(body.as_ref())
                .map_err(|e| AwsError::new("ValidationException", e.to_string()))?;
            cbor_to_json(cbor)
        };
        let body_json = serde_json::to_vec(&req).unwrap_or_default();
        let resp =
            <Self as JsonProtocolService>::dispatch(self, ctx, operation, Bytes::from(body_json))
                .await?;
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&resp, &mut buf)
            .map_err(|e| AwsError::internal(e.to_string()))?;
        Ok(Bytes::from(buf))
    }
}

fn cbor_to_json(v: ciborium::value::Value) -> Value {
    use ciborium::value::Value as CV;
    match v {
        CV::Null => Value::Null,
        CV::Bool(b) => Value::Bool(b),
        CV::Integer(i) => {
            let n: i128 = i.into();
            if let Ok(i64v) = i64::try_from(n) {
                Value::Number(i64v.into())
            } else {
                Value::String(n.to_string())
            }
        }
        CV::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        CV::Text(s) => Value::String(s),
        CV::Bytes(b) => {
            use base64::Engine;
            Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        CV::Array(arr) => Value::Array(arr.into_iter().map(cbor_to_json).collect()),
        CV::Map(entries) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in entries {
                if let CV::Text(key) = k {
                    obj.insert(key, cbor_to_json(val));
                }
            }
            Value::Object(obj)
        }
        CV::Tag(_, inner) => cbor_to_json(*inner),
        _ => Value::Null,
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    let svc = Arc::new(GameLift::new());
    registry.register_json(svc.clone());
    registry.register_cbor(svc);
}
