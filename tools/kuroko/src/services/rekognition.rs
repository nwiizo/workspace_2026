//! Rekognition — AWS JSON 1.1, target prefix `RekognitionService`.

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

const TARGET_PREFIX: &str = "RekognitionService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    collections: HashMap<String, Collection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Collection {
    id: String,
    arn: String,
    face_count: i32,
}

pub struct Rekognition {
    state: Arc<RwLock<State>>,
}
impl Rekognition {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Rekognition {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Rekognition {
    fn name(&self) -> &'static str {
        "rekognition"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("rekognition")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("rekognition", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Rekognition {
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
            "CreateCollection" => {
                let id = required(&req, "CollectionId")?;
                let arn = format!(
                    "arn:aws:rekognition:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:collection/{id}"
                );
                let mut s = self.state.write();
                if s.collections.contains_key(&id) {
                    return Err(AwsError::new(
                        "ResourceAlreadyExistsException",
                        format!("collection '{id}' exists"),
                    ));
                }
                s.collections.insert(
                    id.clone(),
                    Collection {
                        id,
                        arn: arn.clone(),
                        face_count: 0,
                    },
                );
                Ok(
                    json!({ "StatusCode": 200, "CollectionArn": arn, "FaceModelVersion": "kuroko-1.0" }),
                )
            }
            "DescribeCollection" => {
                let id = required(&req, "CollectionId")?;
                let s = self.state.read();
                let c = s.collections.get(&id).ok_or_else(|| not_found(&id))?;
                Ok(json!({
                    "CollectionARN": c.arn,
                    "FaceCount": c.face_count,
                    "FaceModelVersion": "kuroko-1.0",
                    "CreationTimestamp": chrono::Utc::now().timestamp(),
                }))
            }
            "ListCollections" => {
                let s = self.state.read();
                let ids: Vec<_> = s.collections.keys().cloned().collect();
                Ok(
                    json!({ "CollectionIds": ids, "FaceModelVersions": vec!["kuroko-1.0"; s.collections.len()] }),
                )
            }
            "DeleteCollection" => {
                let id = required(&req, "CollectionId")?;
                self.state
                    .write()
                    .collections
                    .remove(&id)
                    .ok_or_else(|| not_found(&id))?;
                Ok(json!({ "StatusCode": 200 }))
            }
            "DetectLabels" => Ok(json!({
                "Labels": [
                    { "Name": "Person", "Confidence": 99.0 },
                    { "Name": "Object", "Confidence": 80.0 },
                ],
                "LabelModelVersion": "kuroko-1.0",
            })),
            "DetectFaces" => Ok(json!({ "FaceDetails": [] })),
            "DetectText" => Ok(json!({ "TextDetections": [] })),
            other => Err(AwsError::unsupported(format!("Rekognition::{other}"))),
        }
    }
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
        format!("collection '{id}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Rekognition::new()));
}
