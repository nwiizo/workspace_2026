//! Forecast — AWS JSON 1.1, target prefix `AmazonForecast`.

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

const TARGET_PREFIX: &str = "AmazonForecast";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    dataset_groups: HashMap<String, DatasetGroup>,
    datasets: HashMap<String, Dataset>,
    predictors: HashMap<String, Predictor>,
    forecasts: HashMap<String, Forecast>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DatasetGroup {
    name: String,
    arn: String,
    domain: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Dataset {
    name: String,
    arn: String,
    domain: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Predictor {
    name: String,
    arn: String,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Forecast {
    name: String,
    arn: String,
    status: String,
}

pub struct ForecastSvc {
    state: Arc<RwLock<State>>,
}
impl ForecastSvc {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for ForecastSvc {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ForecastSvc {
    fn name(&self) -> &'static str {
        "forecast"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("forecast").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("forecast", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for ForecastSvc {
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
                .map_err(|e| AwsError::new("InvalidInputException", e.to_string()))?
        };
        match action {
            "CreateDatasetGroup" => {
                let name = required(&req, "DatasetGroupName")?;
                let domain = required(&req, "Domain")?;
                let arn = format!(
                    "arn:aws:forecast:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:dataset-group/{name}"
                );
                self.state.write().dataset_groups.insert(
                    name.clone(),
                    DatasetGroup {
                        name,
                        arn: arn.clone(),
                        domain,
                        status: "ACTIVE".into(),
                    },
                );
                Ok(json!({ "DatasetGroupArn": arn }))
            }
            "DescribeDatasetGroup" => {
                let arn = required(&req, "DatasetGroupArn")?;
                let name = arn.rsplit('/').next().unwrap_or("").to_string();
                let s = self.state.read();
                let g = s.dataset_groups.get(&name).ok_or_else(|| not_found(&arn))?;
                Ok(json!({
                    "DatasetGroupName": g.name,
                    "DatasetGroupArn": g.arn,
                    "Domain": g.domain,
                    "Status": g.status,
                }))
            }
            "ListDatasetGroups" => {
                let s = self.state.read();
                let list: Vec<_> = s
                    .dataset_groups
                    .values()
                    .map(|g| {
                        json!({
                            "DatasetGroupName": g.name,
                            "DatasetGroupArn": g.arn,
                        })
                    })
                    .collect();
                Ok(json!({ "DatasetGroups": list }))
            }
            "DeleteDatasetGroup" => {
                let arn = required(&req, "DatasetGroupArn")?;
                let name = arn.rsplit('/').next().unwrap_or("").to_string();
                self.state.write().dataset_groups.remove(&name);
                Ok(json!({}))
            }
            "CreateDataset" => {
                let name = required(&req, "DatasetName")?;
                let domain = required(&req, "Domain")?;
                let arn = format!(
                    "arn:aws:forecast:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:dataset/{name}"
                );
                self.state.write().datasets.insert(
                    name.clone(),
                    Dataset {
                        name,
                        arn: arn.clone(),
                        domain,
                        status: "ACTIVE".into(),
                    },
                );
                Ok(json!({ "DatasetArn": arn }))
            }
            "ListDatasets" => {
                let s = self.state.read();
                let list: Vec<_> = s
                    .datasets
                    .values()
                    .map(|d| {
                        json!({
                            "DatasetName": d.name,
                            "DatasetArn": d.arn,
                        })
                    })
                    .collect();
                Ok(json!({ "Datasets": list }))
            }
            "CreatePredictor" => {
                let name = required(&req, "PredictorName")?;
                let arn = format!(
                    "arn:aws:forecast:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:predictor/{name}"
                );
                self.state.write().predictors.insert(
                    name.clone(),
                    Predictor {
                        name,
                        arn: arn.clone(),
                        status: "ACTIVE".into(),
                    },
                );
                Ok(json!({ "PredictorArn": arn }))
            }
            "CreateForecast" => {
                let name = required(&req, "ForecastName")?;
                let arn = format!(
                    "arn:aws:forecast:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:forecast/{name}"
                );
                self.state.write().forecasts.insert(
                    name.clone(),
                    Forecast {
                        name,
                        arn: arn.clone(),
                        status: "ACTIVE".into(),
                    },
                );
                Ok(json!({ "ForecastArn": arn }))
            }
            "ListForecasts" => {
                let s = self.state.read();
                let list: Vec<_> = s
                    .forecasts
                    .values()
                    .map(|f| {
                        json!({
                            "ForecastName": f.name,
                            "ForecastArn": f.arn,
                            "Status": f.status,
                        })
                    })
                    .collect();
                Ok(json!({ "Forecasts": list }))
            }
            other => Err(AwsError::unsupported(format!("Forecast::{other}"))),
        }
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidInputException", format!("{key} required")))
}

fn not_found(arn: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("resource '{arn}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(ForecastSvc::new()));
}
