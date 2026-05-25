//! SageMaker — AWS JSON 1.1, target prefix `SageMaker`.
//!
//! Notebook instance, training job, model, endpoint metadata.

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

const TARGET_PREFIX: &str = "SageMaker";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    notebooks: HashMap<String, NotebookInstance>,
    training_jobs: HashMap<String, TrainingJob>,
    models: HashMap<String, Model>,
    endpoints: HashMap<String, Endpoint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NotebookInstance {
    name: String,
    arn: String,
    instance_type: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TrainingJob {
    name: String,
    arn: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Model {
    name: String,
    arn: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Endpoint {
    name: String,
    arn: String,
    status: String,
}

pub struct SageMaker {
    state: Arc<RwLock<State>>,
}
impl SageMaker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for SageMaker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for SageMaker {
    fn name(&self) -> &'static str {
        "sagemaker"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("sagemaker").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("sagemaker", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for SageMaker {
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
            "CreateNotebookInstance" => {
                let name = required(&req, "NotebookInstanceName")?;
                let instance_type = required(&req, "InstanceType")?;
                let arn = format!(
                    "arn:aws:sagemaker:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:notebook-instance/{name}"
                );
                self.state.write().notebooks.insert(
                    name.clone(),
                    NotebookInstance {
                        name,
                        arn: arn.clone(),
                        instance_type,
                        status: "InService".into(),
                    },
                );
                Ok(json!({ "NotebookInstanceArn": arn }))
            }
            "DescribeNotebookInstance" => {
                let name = required(&req, "NotebookInstanceName")?;
                let s = self.state.read();
                let n = s.notebooks.get(&name).ok_or_else(|| not_found(&name))?;
                Ok(json!({
                    "NotebookInstanceName": n.name,
                    "NotebookInstanceArn": n.arn,
                    "InstanceType": n.instance_type,
                    "NotebookInstanceStatus": n.status,
                }))
            }
            "ListNotebookInstances" => {
                let s = self.state.read();
                let list: Vec<_> = s
                    .notebooks
                    .values()
                    .map(|n| {
                        json!({
                            "NotebookInstanceName": n.name,
                            "NotebookInstanceArn": n.arn,
                            "NotebookInstanceStatus": n.status,
                        })
                    })
                    .collect();
                Ok(json!({ "NotebookInstances": list }))
            }
            "DeleteNotebookInstance" => {
                let name = required(&req, "NotebookInstanceName")?;
                self.state.write().notebooks.remove(&name);
                Ok(json!({}))
            }
            "CreateTrainingJob" => {
                let name = required(&req, "TrainingJobName")?;
                let arn = format!(
                    "arn:aws:sagemaker:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:training-job/{name}"
                );
                self.state.write().training_jobs.insert(
                    name.clone(),
                    TrainingJob {
                        name,
                        arn: arn.clone(),
                        status: "Completed".into(),
                    },
                );
                Ok(json!({ "TrainingJobArn": arn }))
            }
            "DescribeTrainingJob" => {
                let name = required(&req, "TrainingJobName")?;
                let s = self.state.read();
                let j = s.training_jobs.get(&name).ok_or_else(|| not_found(&name))?;
                Ok(json!({
                    "TrainingJobName": j.name,
                    "TrainingJobArn": j.arn,
                    "TrainingJobStatus": j.status,
                }))
            }
            "CreateModel" => {
                let name = required(&req, "ModelName")?;
                let arn = format!(
                    "arn:aws:sagemaker:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:model/{name}"
                );
                self.state.write().models.insert(
                    name.clone(),
                    Model {
                        name,
                        arn: arn.clone(),
                    },
                );
                Ok(json!({ "ModelArn": arn }))
            }
            "CreateEndpoint" => {
                let name = required(&req, "EndpointName")?;
                let arn = format!(
                    "arn:aws:sagemaker:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:endpoint/{name}"
                );
                self.state.write().endpoints.insert(
                    name.clone(),
                    Endpoint {
                        name,
                        arn: arn.clone(),
                        status: "InService".into(),
                    },
                );
                Ok(json!({ "EndpointArn": arn }))
            }
            other => Err(AwsError::unsupported(format!("SageMaker::{other}"))),
        }
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new("ResourceNotFound", format!("resource '{name}' not found"))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(SageMaker::new()));
}
