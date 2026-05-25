//! AWS Cost Explorer (ce) — AWS JSON 1.1, target prefix `AWSInsightsIndexService`.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{JsonProtocolService, Service, ServiceContext, persistence_error};

const TARGET_PREFIX: &str = "AWSInsightsIndexService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    tags: Vec<String>,
}

pub struct CostExplorer {
    state: Arc<RwLock<State>>,
}
impl CostExplorer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for CostExplorer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CostExplorer {
    fn name(&self) -> &'static str {
        "ce"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ce").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("ce", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for CostExplorer {
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
        let _ = req;
        match action {
            "GetCostAndUsage" => Ok(json!({
                "ResultsByTime": [
                    {
                        "TimePeriod": { "Start": "2026-01-01", "End": "2026-01-02" },
                        "Total": {
                            "BlendedCost": { "Amount": "0.0", "Unit": "USD" },
                            "UnblendedCost": { "Amount": "0.0", "Unit": "USD" },
                        },
                        "Groups": [],
                        "Estimated": false,
                    }
                ],
                "DimensionValueAttributes": [],
            })),
            "GetCostForecast" => Ok(json!({
                "Total": { "Amount": "0.0", "Unit": "USD" },
                "ForecastResultsByTime": [],
            })),
            "GetDimensionValues" => Ok(json!({
                "DimensionValues": [],
                "ReturnSize": 0,
                "TotalSize": 0,
            })),
            "GetTags" => {
                let s = self.state.read();
                Ok(json!({
                    "Tags": s.tags,
                    "ReturnSize": s.tags.len(),
                    "TotalSize": s.tags.len(),
                }))
            }
            other => Err(AwsError::unsupported(format!("CostExplorer::{other}"))),
        }
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(CostExplorer::new()));
}
