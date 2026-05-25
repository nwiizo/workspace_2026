//! CloudTrail — AWS JSON 1.1, target prefix `CloudTrail_20131101`.

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

const TARGET_PREFIX: &str = "CloudTrail_20131101";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    trails: HashMap<String, Trail>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Trail {
    name: String,
    arn: String,
    s3_bucket_name: String,
    s3_key_prefix: Option<String>,
    include_global_service_events: bool,
    is_multi_region_trail: bool,
    is_logging: bool,
}

pub struct CloudTrail {
    state: Arc<RwLock<State>>,
}

impl CloudTrail {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for CloudTrail {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CloudTrail {
    fn name(&self) -> &'static str {
        "cloudtrail"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("cloudtrail")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("cloudtrail", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for CloudTrail {
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
                .map_err(|e| AwsError::new("InvalidRequest", e.to_string()))?
        };
        match action {
            "CreateTrail" => self.create_trail(&req),
            "DescribeTrails" => self.describe_trails(),
            "GetTrail" => self.get_trail(&req),
            "ListTrails" => self.list_trails(),
            "StartLogging" => self.start_logging(&req),
            "StopLogging" => self.stop_logging(&req),
            "GetTrailStatus" => self.get_trail_status(&req),
            "DeleteTrail" => self.delete_trail(&req),
            "LookupEvents" => Ok(json!({ "Events": [] })),
            other => Err(AwsError::unsupported(format!("CloudTrail::{other}"))),
        }
    }
}

impl CloudTrail {
    fn create_trail(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let s3_bucket = required(req, "S3BucketName")?;
        let mut s = self.state.write();
        if s.trails.contains_key(&name) {
            return Err(AwsError::new(
                "TrailAlreadyExistsException",
                format!("trail '{name}' already exists"),
            ));
        }
        let arn =
            format!("arn:aws:cloudtrail:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:trail/{name}");
        let trail = Trail {
            name: name.clone(),
            arn,
            s3_bucket_name: s3_bucket,
            s3_key_prefix: req
                .get("S3KeyPrefix")
                .and_then(Value::as_str)
                .map(String::from),
            include_global_service_events: req
                .get("IncludeGlobalServiceEvents")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            is_multi_region_trail: req
                .get("IsMultiRegionTrail")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            is_logging: false,
        };
        let resp = trail_json(&trail);
        s.trails.insert(name, trail);
        Ok(resp)
    }

    fn describe_trails(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let trail_list: Vec<_> = s.trails.values().map(trail_json).collect();
        Ok(json!({ "trailList": trail_list }))
    }

    fn get_trail(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let s = self.state.read();
        let trail = s.trails.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({ "Trail": trail_json(trail) }))
    }

    fn list_trails(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let trails: Vec<_> = s
            .trails
            .values()
            .map(|t| {
                json!({
                    "Name": t.name,
                    "TrailARN": t.arn,
                    "HomeRegion": EMULATED_REGION,
                })
            })
            .collect();
        Ok(json!({ "Trails": trails }))
    }

    fn start_logging(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let mut s = self.state.write();
        let trail = s.trails.get_mut(&name).ok_or_else(|| not_found(&name))?;
        trail.is_logging = true;
        Ok(json!({}))
    }

    fn stop_logging(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let mut s = self.state.write();
        let trail = s.trails.get_mut(&name).ok_or_else(|| not_found(&name))?;
        trail.is_logging = false;
        Ok(json!({}))
    }

    fn get_trail_status(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        let s = self.state.read();
        let trail = s.trails.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "IsLogging": trail.is_logging,
            "LatestDeliveryTime": chrono::Utc::now().timestamp(),
        }))
    }

    fn delete_trail(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "Name")?;
        self.state
            .write()
            .trails
            .remove(&name)
            .ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidRequest", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "TrailNotFoundException",
        format!("trail '{name}' not found"),
    )
}

fn trail_json(t: &Trail) -> Value {
    json!({
        "Name": t.name,
        "TrailARN": t.arn,
        "S3BucketName": t.s3_bucket_name,
        "S3KeyPrefix": t.s3_key_prefix,
        "IncludeGlobalServiceEvents": t.include_global_service_events,
        "IsMultiRegionTrail": t.is_multi_region_trail,
        "HomeRegion": EMULATED_REGION,
        "LogFileValidationEnabled": false,
    })
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(CloudTrail::new()));
}
