//! Amazon Pinpoint SMS & Voice API V2 — AWS JSON 1.0, target prefix `PinpointSMSVoiceV2`.

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

const TARGET_PREFIX: &str = "PinpointSMSVoiceV2";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    pools: HashMap<String, Pool>,
    sent: Vec<SentMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Pool {
    id: String,
    arn: String,
    name: String,
    status: String,
    iso_country_code: String,
    message_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SentMessage {
    message_id: String,
    destination: String,
    body: String,
}

pub struct PinpointSmsVoiceV2 {
    state: Arc<RwLock<State>>,
}
impl PinpointSmsVoiceV2 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for PinpointSmsVoiceV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for PinpointSmsVoiceV2 {
    fn name(&self) -> &'static str {
        "pinpointsmsvoicev2"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("pinpointsmsvoicev2")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("pinpointsmsvoicev2", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for PinpointSmsVoiceV2 {
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
            "CreatePool" => {
                let origination = req
                    .get("OriginationIdentity")
                    .and_then(Value::as_str)
                    .map(String::from);
                let iso = req
                    .get("IsoCountryCode")
                    .and_then(Value::as_str)
                    .unwrap_or("US")
                    .to_string();
                let message_type = req
                    .get("MessageType")
                    .and_then(Value::as_str)
                    .unwrap_or("TRANSACTIONAL")
                    .to_string();
                let id = format!("pool-{}", Uuid::new_v4().simple());
                let pool = Pool {
                    arn: pool_arn(&id),
                    name: origination.unwrap_or_else(|| id.clone()),
                    id: id.clone(),
                    status: "ACTIVE".into(),
                    iso_country_code: iso,
                    message_type,
                };
                let body = pool_json(&pool);
                self.state.write().pools.insert(id, pool);
                Ok(body)
            }
            "DescribePools" => {
                let s = self.state.read();
                let list: Vec<Value> = s.pools.values().map(pool_json).collect();
                Ok(json!({ "Pools": list }))
            }
            "DeletePool" => {
                let id = required(&req, "PoolId")?;
                let mut s = self.state.write();
                let p = s.pools.remove(&id).ok_or_else(|| {
                    AwsError::new(
                        "ResourceNotFoundException",
                        format!("pool '{id}' not found"),
                    )
                })?;
                Ok(pool_json(&p))
            }
            "SendTextMessage" => {
                let destination = required(&req, "DestinationPhoneNumber")?;
                let body_text = req
                    .get("MessageBody")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let message_id = Uuid::new_v4().to_string();
                self.state.write().sent.push(SentMessage {
                    message_id: message_id.clone(),
                    destination,
                    body: body_text,
                });
                Ok(json!({ "MessageId": message_id }))
            }
            other => Err(AwsError::unsupported(format!(
                "PinpointSMSVoiceV2::{other}"
            ))),
        }
    }
}

fn pool_json(p: &Pool) -> Value {
    json!({
        "PoolId": p.id,
        "PoolArn": p.arn,
        "Status": p.status,
        "MessageType": p.message_type,
        "TwoWayEnabled": false,
        "SelfManagedOptOutsEnabled": false,
        "OptOutListName": "Default",
        "SharedRoutesEnabled": false,
        "DeletionProtectionEnabled": false,
        "CreatedTimestamp": chrono::Utc::now().timestamp(),
        "IsoCountryCode": p.iso_country_code,
    })
}

fn pool_arn(id: &str) -> String {
    format!("arn:aws:sms-voice:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:pool/{id}")
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", format!("{key} required")))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(PinpointSmsVoiceV2::new()));
}
