//! Comprehend — AWS JSON 1.1, target prefix `Comprehend_20171127`.
//!
//! Synchronous detection ops return deterministic stub results based on the
//! input text (e.g. always detects English, always positive sentiment).
//! Custom classifiers / entity recognizers are tracked as metadata only.

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

const TARGET_PREFIX: &str = "Comprehend_20171127";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    classifiers: HashMap<String, Classifier>,
    entity_recognizers: HashMap<String, EntityRecognizer>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Classifier {
    name: String,
    arn: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EntityRecognizer {
    name: String,
    arn: String,
    status: String,
}

pub struct Comprehend {
    state: Arc<RwLock<State>>,
}

impl Comprehend {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Comprehend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Comprehend {
    fn name(&self) -> &'static str {
        "comprehend"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("comprehend")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("comprehend", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Comprehend {
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
            "DetectDominantLanguage" => Ok(json!({
                "Languages": [{ "LanguageCode": "en", "Score": 0.99 }],
            })),
            "DetectSentiment" => Ok(json!({
                "Sentiment": "POSITIVE",
                "SentimentScore": {
                    "Positive": 0.9, "Negative": 0.05, "Neutral": 0.04, "Mixed": 0.01,
                }
            })),
            "DetectEntities" => Ok(json!({
                "Entities": [{ "Score": 0.99, "Type": "OTHER", "Text": "kuroko", "BeginOffset": 0, "EndOffset": 6 }],
            })),
            "DetectKeyPhrases" => Ok(json!({
                "KeyPhrases": [{ "Score": 0.95, "Text": "kuroko", "BeginOffset": 0, "EndOffset": 6 }],
            })),
            "DetectPiiEntities" => Ok(json!({ "Entities": [] })),
            "DetectSyntax" => Ok(json!({ "SyntaxTokens": [] })),
            "CreateDocumentClassifier" => {
                let name = req
                    .get("DocumentClassifierName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AwsError::new("InvalidRequestException", "Name required"))?
                    .to_string();
                let arn = format!(
                    "arn:aws:comprehend:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:document-classifier/{name}"
                );
                self.state.write().classifiers.insert(
                    name.clone(),
                    Classifier {
                        name,
                        arn: arn.clone(),
                        status: "TRAINED".into(),
                    },
                );
                Ok(json!({ "DocumentClassifierArn": arn }))
            }
            "DescribeDocumentClassifier" => {
                let arn = req
                    .get("DocumentClassifierArn")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let name = arn.rsplit('/').next().unwrap_or("");
                let s = self.state.read();
                match s.classifiers.get(name) {
                    Some(c) => Ok(json!({
                        "DocumentClassifierProperties": {
                            "DocumentClassifierArn": c.arn,
                            "Status": c.status,
                        }
                    })),
                    None => Err(AwsError::new(
                        "ResourceNotFoundException",
                        "classifier not found",
                    )),
                }
            }
            "ListDocumentClassifiers" => {
                let s = self.state.read();
                let props: Vec<_> = s
                    .classifiers
                    .values()
                    .map(|c| {
                        json!({
                            "DocumentClassifierArn": c.arn,
                            "Status": c.status,
                        })
                    })
                    .collect();
                Ok(json!({ "DocumentClassifierPropertiesList": props }))
            }
            "DeleteDocumentClassifier" => {
                let arn = req
                    .get("DocumentClassifierArn")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let name = arn.rsplit('/').next().unwrap_or("").to_string();
                self.state.write().classifiers.remove(&name);
                Ok(json!({}))
            }
            "CreateEntityRecognizer" => {
                let name = req
                    .get("RecognizerName")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AwsError::new("InvalidRequestException", "Name required"))?
                    .to_string();
                let arn = format!(
                    "arn:aws:comprehend:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:entity-recognizer/{name}"
                );
                self.state.write().entity_recognizers.insert(
                    name.clone(),
                    EntityRecognizer {
                        name,
                        arn: arn.clone(),
                        status: "TRAINED".into(),
                    },
                );
                Ok(json!({ "EntityRecognizerArn": arn }))
            }
            "ListEntityRecognizers" => {
                let s = self.state.read();
                let props: Vec<_> = s
                    .entity_recognizers
                    .values()
                    .map(|r| {
                        json!({
                            "EntityRecognizerArn": r.arn,
                            "Status": r.status,
                        })
                    })
                    .collect();
                Ok(json!({ "EntityRecognizerPropertiesList": props }))
            }
            other => Err(AwsError::unsupported(format!("Comprehend::{other}"))),
        }
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Comprehend::new()));
}
