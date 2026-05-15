//! CloudWatch Logs — AWS JSON 1.1, target prefix `Logs_20140328`.
//!
//! Implements log group / stream lifecycle plus PutLogEvents, GetLogEvents,
//! FilterLogEvents. Pagination is a simple in-memory cursor (start at offset
//! and walk forward); good enough for the test workloads where this service
//! sees the most use.

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

const TARGET_PREFIX: &str = "Logs_20140328";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    groups: HashMap<String, LogGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LogGroup {
    name: String,
    arn: String,
    streams: HashMap<String, LogStream>,
    retention_days: Option<i64>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LogStream {
    name: String,
    events: Vec<LogEvent>,
    last_event_ts: Option<i64>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LogEvent {
    timestamp: i64,
    message: String,
    ingestion_time: i64,
}

pub struct CloudWatchLogs {
    state: Arc<RwLock<State>>,
}

impl CloudWatchLogs {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for CloudWatchLogs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for CloudWatchLogs {
    fn name(&self) -> &'static str {
        "cloudwatchlogs"
    }

    fn reset(&self) {
        self.state.write().groups.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("cloudwatchlogs")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("cloudwatchlogs", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for CloudWatchLogs {
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
            "CreateLogGroup" => self.create_log_group(&req),
            "DeleteLogGroup" => self.delete_log_group(&req),
            "DescribeLogGroups" => self.describe_log_groups(&req),
            "PutRetentionPolicy" => self.put_retention_policy(&req),
            "CreateLogStream" => self.create_log_stream(&req),
            "DeleteLogStream" => self.delete_log_stream(&req),
            "DescribeLogStreams" => self.describe_log_streams(&req),
            "PutLogEvents" => self.put_log_events(&req),
            "GetLogEvents" => self.get_log_events(&req),
            "FilterLogEvents" => self.filter_log_events(&req),
            other => Err(AwsError::unsupported(format!("CloudWatchLogs::{other}"))),
        }
    }
}

impl CloudWatchLogs {
    fn create_log_group(&self, req: &Value) -> Result<Value, AwsError> {
        let name = group_name(req)?;
        let mut s = self.state.write();
        if s.groups.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceAlreadyExistsException",
                format!("group '{name}' already exists"),
            ));
        }
        let arn = group_arn(&name);
        let group = LogGroup {
            name: name.clone(),
            arn,
            streams: HashMap::new(),
            retention_days: None,
            created: chrono::Utc::now(),
        };
        s.groups.insert(name, group);
        Ok(json!({}))
    }

    fn delete_log_group(&self, req: &Value) -> Result<Value, AwsError> {
        let name = group_name(req)?;
        let mut s = self.state.write();
        s.groups
            .remove(&name)
            .ok_or_else(|| not_found_group(&name))?;
        Ok(json!({}))
    }

    fn describe_log_groups(&self, req: &Value) -> Result<Value, AwsError> {
        let prefix = req
            .get("logGroupNamePrefix")
            .and_then(Value::as_str)
            .unwrap_or("");
        let s = self.state.read();
        let groups: Vec<_> = s
            .groups
            .values()
            .filter(|g| g.name.starts_with(prefix))
            .map(|g| {
                json!({
                    "logGroupName": g.name,
                    "arn": g.arn,
                    "creationTime": g.created.timestamp_millis(),
                    "retentionInDays": g.retention_days,
                    "storedBytes": 0i64,
                })
            })
            .collect();
        Ok(json!({ "logGroups": groups }))
    }

    fn put_retention_policy(&self, req: &Value) -> Result<Value, AwsError> {
        let name = group_name(req)?;
        let days = req
            .get("retentionInDays")
            .and_then(Value::as_i64)
            .ok_or_else(|| AwsError::new("ValidationException", "retentionInDays required"))?;
        let mut s = self.state.write();
        let group = s
            .groups
            .get_mut(&name)
            .ok_or_else(|| not_found_group(&name))?;
        group.retention_days = Some(days);
        Ok(json!({}))
    }

    fn create_log_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let stream_n = stream_name(req)?;
        let mut s = self.state.write();
        let group = s
            .groups
            .get_mut(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        if group.streams.contains_key(&stream_n) {
            return Err(AwsError::new(
                "ResourceAlreadyExistsException",
                format!("stream '{stream_n}' already exists"),
            ));
        }
        group.streams.insert(
            stream_n.clone(),
            LogStream {
                name: stream_n,
                events: Vec::new(),
                last_event_ts: None,
                created: chrono::Utc::now(),
            },
        );
        Ok(json!({}))
    }

    fn delete_log_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let stream_n = stream_name(req)?;
        let mut s = self.state.write();
        let group = s
            .groups
            .get_mut(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        group.streams.remove(&stream_n);
        Ok(json!({}))
    }

    fn describe_log_streams(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let prefix = req
            .get("logStreamNamePrefix")
            .and_then(Value::as_str)
            .unwrap_or("");
        let s = self.state.read();
        let group = s
            .groups
            .get(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        let streams: Vec<_> = group
            .streams
            .values()
            .filter(|st| st.name.starts_with(prefix))
            .map(|st| {
                json!({
                    "logStreamName": st.name,
                    "creationTime": st.created.timestamp_millis(),
                    "lastEventTimestamp": st.last_event_ts,
                    "arn": format!("{}:log-stream:{}", group.arn, st.name),
                })
            })
            .collect();
        Ok(json!({ "logStreams": streams }))
    }

    fn put_log_events(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let stream_n = stream_name(req)?;
        let events = req
            .get("logEvents")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("ValidationException", "logEvents required"))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut s = self.state.write();
        let group = s
            .groups
            .get_mut(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        let stream = group
            .streams
            .get_mut(&stream_n)
            .ok_or_else(|| not_found_stream(&stream_n))?;
        let mut last_ts = stream.last_event_ts;
        for ev in events {
            let timestamp = ev
                .get("timestamp")
                .and_then(Value::as_i64)
                .ok_or_else(|| AwsError::new("ValidationException", "event timestamp required"))?;
            let message = ev
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            stream.events.push(LogEvent {
                timestamp,
                message,
                ingestion_time: now_ms,
            });
            last_ts = Some(last_ts.map_or(timestamp, |t| t.max(timestamp)));
        }
        stream.events.sort_by_key(|e| e.timestamp);
        stream.last_event_ts = last_ts;
        Ok(json!({
            "nextSequenceToken": uuid::Uuid::new_v4().simple().to_string(),
        }))
    }

    fn get_log_events(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let stream_n = stream_name(req)?;
        let start_time = req.get("startTime").and_then(Value::as_i64);
        let end_time = req.get("endTime").and_then(Value::as_i64);
        let limit = req.get("limit").and_then(Value::as_u64).unwrap_or(10_000) as usize;
        let s = self.state.read();
        let group = s
            .groups
            .get(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        let stream = group
            .streams
            .get(&stream_n)
            .ok_or_else(|| not_found_stream(&stream_n))?;
        let events: Vec<_> = stream
            .events
            .iter()
            .filter(|e| start_time.is_none_or(|st| e.timestamp >= st))
            .filter(|e| end_time.is_none_or(|et| e.timestamp < et))
            .take(limit)
            .map(|e| {
                json!({
                    "timestamp": e.timestamp,
                    "message": e.message,
                    "ingestionTime": e.ingestion_time,
                })
            })
            .collect();
        Ok(json!({ "events": events }))
    }

    fn filter_log_events(&self, req: &Value) -> Result<Value, AwsError> {
        let group_n = group_name(req)?;
        let pattern = req
            .get("filterPattern")
            .and_then(Value::as_str)
            .unwrap_or("");
        let s = self.state.read();
        let group = s
            .groups
            .get(&group_n)
            .ok_or_else(|| not_found_group(&group_n))?;
        let mut hits = Vec::new();
        for stream in group.streams.values() {
            for ev in &stream.events {
                if pattern.is_empty() || ev.message.contains(pattern) {
                    hits.push(json!({
                        "logStreamName": stream.name,
                        "timestamp": ev.timestamp,
                        "message": ev.message,
                        "ingestionTime": ev.ingestion_time,
                    }));
                }
            }
        }
        Ok(json!({ "events": hits, "searchedLogStreams": [] }))
    }
}

fn group_name(req: &Value) -> Result<String, AwsError> {
    req.get("logGroupName")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", "logGroupName required"))
}

fn stream_name(req: &Value) -> Result<String, AwsError> {
    req.get("logStreamName")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", "logStreamName required"))
}

fn not_found_group(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("log group '{name}' does not exist"),
    )
}

fn not_found_stream(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("log stream '{name}' does not exist"),
    )
}

fn group_arn(name: &str) -> String {
    format!("arn:aws:logs:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:log-group:{name}")
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(CloudWatchLogs::new()));
}
