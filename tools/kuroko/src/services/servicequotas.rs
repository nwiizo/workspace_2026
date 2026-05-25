//! Service Quotas — AWS JSON 1.1, target prefix `ServiceQuotasV20190624`.

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

const TARGET_PREFIX: &str = "ServiceQuotasV20190624";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    quotas: HashMap<(String, String), QuotaOverride>,
    requests: HashMap<String, RequestRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QuotaOverride {
    value: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RequestRecord {
    id: String,
    service_code: String,
    quota_code: String,
    desired_value: f64,
    status: String,
}

pub struct ServiceQuotas {
    state: Arc<RwLock<State>>,
}
impl ServiceQuotas {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for ServiceQuotas {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ServiceQuotas {
    fn name(&self) -> &'static str {
        "servicequotas"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("servicequotas")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("servicequotas", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for ServiceQuotas {
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
            "ListServices" => Ok(json!({
                "Services": [
                    { "ServiceCode": "ec2", "ServiceName": "Amazon Elastic Compute Cloud (Amazon EC2)" },
                    { "ServiceCode": "lambda", "ServiceName": "AWS Lambda" },
                    { "ServiceCode": "s3", "ServiceName": "Amazon Simple Storage Service (Amazon S3)" },
                    { "ServiceCode": "sqs", "ServiceName": "Amazon Simple Queue Service" },
                    { "ServiceCode": "dynamodb", "ServiceName": "Amazon DynamoDB" },
                ]
            })),
            "ListServiceQuotas" => {
                let svc = required(&req, "ServiceCode")?;
                let quotas = default_quotas(&svc);
                let s = self.state.read();
                let list: Vec<Value> = quotas
                    .into_iter()
                    .map(|(code, name, default)| {
                        let value = s
                            .quotas
                            .get(&(svc.clone(), code.to_string()))
                            .map(|q| q.value)
                            .unwrap_or(default);
                        quota_json(&svc, code, name, value, false)
                    })
                    .collect();
                Ok(json!({ "Quotas": list }))
            }
            "ListAWSDefaultServiceQuotas" => {
                let svc = required(&req, "ServiceCode")?;
                let quotas = default_quotas(&svc);
                let list: Vec<Value> = quotas
                    .into_iter()
                    .map(|(code, name, default)| quota_json(&svc, code, name, default, true))
                    .collect();
                Ok(json!({ "Quotas": list }))
            }
            "GetServiceQuota" => {
                let svc = required(&req, "ServiceCode")?;
                let qc = required(&req, "QuotaCode")?;
                let quotas = default_quotas(&svc);
                let entry = quotas
                    .into_iter()
                    .find(|(c, _, _)| *c == qc)
                    .ok_or_else(|| {
                        AwsError::new(
                            "NoSuchResourceException",
                            format!("quota {qc} not found for {svc}"),
                        )
                    })?;
                let s = self.state.read();
                let value = s
                    .quotas
                    .get(&(svc.clone(), qc.clone()))
                    .map(|q| q.value)
                    .unwrap_or(entry.2);
                Ok(json!({ "Quota": quota_json(&svc, entry.0, entry.1, value, false) }))
            }
            "GetAWSDefaultServiceQuota" => {
                let svc = required(&req, "ServiceCode")?;
                let qc = required(&req, "QuotaCode")?;
                let quotas = default_quotas(&svc);
                let entry = quotas
                    .into_iter()
                    .find(|(c, _, _)| *c == qc)
                    .ok_or_else(|| {
                        AwsError::new(
                            "NoSuchResourceException",
                            format!("quota {qc} not found for {svc}"),
                        )
                    })?;
                Ok(json!({ "Quota": quota_json(&svc, entry.0, entry.1, entry.2, true) }))
            }
            "RequestServiceQuotaIncrease" => {
                let svc = required(&req, "ServiceCode")?;
                let qc = required(&req, "QuotaCode")?;
                let desired = req
                    .get("DesiredValue")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| invalid("DesiredValue required"))?;
                let id = format!("req-{}", Uuid::new_v4().simple());
                let rec = RequestRecord {
                    id: id.clone(),
                    service_code: svc.clone(),
                    quota_code: qc.clone(),
                    desired_value: desired,
                    status: "PENDING".into(),
                };
                self.state.write().requests.insert(id.clone(), rec.clone());
                Ok(json!({ "RequestedQuota": request_json(&rec) }))
            }
            "ListRequestedServiceQuotaChangeHistory" => {
                let s = self.state.read();
                let list: Vec<Value> = s.requests.values().map(request_json).collect();
                Ok(json!({ "RequestedQuotas": list }))
            }
            other => Err(AwsError::unsupported(format!("ServiceQuotas::{other}"))),
        }
    }
}

fn default_quotas(svc: &str) -> Vec<(&'static str, &'static str, f64)> {
    match svc {
        "ec2" => vec![
            ("L-1216C47A", "Running On-Demand Standard instances", 5.0),
            ("L-D44B4CD0", "EC2-VPC Elastic IPs", 5.0),
        ],
        "lambda" => vec![
            ("L-B99A9384", "Concurrent executions", 1000.0),
            ("L-548AE339", "Function and layer storage", 75.0),
        ],
        "s3" => vec![("L-DC2B2D3D", "Buckets", 100.0)],
        "sqs" => vec![("L-CFE6BF63", "Maximum number of queues", 1_000_000.0)],
        "dynamodb" => vec![("L-F98FE922", "Tables per Region", 2500.0)],
        _ => Vec::new(),
    }
}

fn quota_json(
    service_code: &str,
    quota_code: &str,
    quota_name: &str,
    value: f64,
    global: bool,
) -> Value {
    json!({
        "ServiceCode": service_code,
        "ServiceName": service_code,
        "QuotaCode": quota_code,
        "QuotaName": quota_name,
        "Value": value,
        "Unit": "None",
        "Adjustable": true,
        "GlobalQuota": global,
        "QuotaArn": format!(
            "arn:aws:servicequotas:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{service_code}/{quota_code}"
        ),
    })
}

fn request_json(r: &RequestRecord) -> Value {
    json!({
        "Id": r.id,
        "ServiceCode": r.service_code,
        "QuotaCode": r.quota_code,
        "DesiredValue": r.desired_value,
        "Status": r.status,
        "Requester": "kuroko",
    })
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| invalid(&format!("{key} required")))
}

fn invalid(msg: &str) -> AwsError {
    AwsError::new("InvalidParameterValueException", msg.to_string())
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(ServiceQuotas::new()));
}
