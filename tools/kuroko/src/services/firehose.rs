//! Firehose — AWS JSON 1.1, target prefix `Firehose_20150804`.
//!
//! Delivery-stream metadata + record sink. Records sent via PutRecord /
//! PutRecordBatch are buffered in memory per stream; no destination
//! flushing happens (S3/Redshift/OpenSearch). Tests verify the API
//! contract and counts.

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

const TARGET_PREFIX: &str = "Firehose_20150804";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    streams: HashMap<String, DeliveryStream>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DeliveryStream {
    name: String,
    arn: String,
    type_: String,
    status: String,
    created: chrono::DateTime<chrono::Utc>,
    records: Vec<Record>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Record {
    record_id: String,
    data_b64: String,
}

pub struct Firehose {
    state: Arc<RwLock<State>>,
}

impl Firehose {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Firehose {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Firehose {
    fn name(&self) -> &'static str {
        "firehose"
    }
    fn reset(&self) {
        self.state.write().streams.clear();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("firehose").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("firehose", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Firehose {
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
            "CreateDeliveryStream" => self.create_delivery_stream(&req),
            "DescribeDeliveryStream" => self.describe_delivery_stream(&req),
            "ListDeliveryStreams" => self.list_delivery_streams(&req),
            "DeleteDeliveryStream" => self.delete_delivery_stream(&req),
            "PutRecord" => self.put_record(&req),
            "PutRecordBatch" => self.put_record_batch(&req),
            other => Err(AwsError::unsupported(format!("Firehose::{other}"))),
        }
    }
}

impl Firehose {
    fn create_delivery_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "DeliveryStreamName")?;
        let type_ = req
            .get("DeliveryStreamType")
            .and_then(Value::as_str)
            .unwrap_or("DirectPut")
            .to_string();
        let mut s = self.state.write();
        if s.streams.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceInUseException",
                format!("delivery stream '{name}' already exists"),
            ));
        }
        let arn = format!(
            "arn:aws:firehose:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:deliverystream/{name}"
        );
        s.streams.insert(
            name.clone(),
            DeliveryStream {
                name,
                arn: arn.clone(),
                type_,
                status: "ACTIVE".into(),
                created: chrono::Utc::now(),
                records: Vec::new(),
            },
        );
        Ok(json!({ "DeliveryStreamARN": arn }))
    }

    fn describe_delivery_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "DeliveryStreamName")?;
        let s = self.state.read();
        let stream = s.streams.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "DeliveryStreamDescription": {
                "DeliveryStreamName": stream.name,
                "DeliveryStreamARN": stream.arn,
                "DeliveryStreamType": stream.type_,
                "DeliveryStreamStatus": stream.status,
                "CreateTimestamp": stream.created.timestamp(),
                "VersionId": "1",
                "HasMoreDestinations": false,
                "Destinations": [],
            }
        }))
    }

    fn list_delivery_streams(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let mut names: Vec<_> = s.streams.keys().cloned().collect();
        names.sort();
        Ok(json!({ "DeliveryStreamNames": names, "HasMoreDeliveryStreams": false }))
    }

    fn delete_delivery_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "DeliveryStreamName")?;
        self.state
            .write()
            .streams
            .remove(&name)
            .ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }

    fn put_record(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "DeliveryStreamName")?;
        let data_b64 = req
            .get("Record")
            .and_then(|r| r.get("Data"))
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "Record.Data required"))?
            .to_string();
        let mut s = self.state.write();
        let stream = s.streams.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let record_id = Uuid::new_v4().simple().to_string();
        stream.records.push(Record {
            record_id: record_id.clone(),
            data_b64,
        });
        Ok(json!({ "RecordId": record_id, "Encrypted": false }))
    }

    fn put_record_batch(&self, req: &Value) -> Result<Value, AwsError> {
        let name = required(req, "DeliveryStreamName")?;
        let records = req
            .get("Records")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "Records required"))?;
        let mut s = self.state.write();
        let stream = s.streams.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let mut responses = Vec::with_capacity(records.len());
        for r in records {
            let data = r
                .get("Data")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let id = Uuid::new_v4().simple().to_string();
            stream.records.push(Record {
                record_id: id.clone(),
                data_b64: data,
            });
            responses.push(json!({ "RecordId": id }));
        }
        Ok(json!({
            "FailedPutCount": 0,
            "Encrypted": false,
            "RequestResponses": responses,
        }))
    }
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidArgumentException", format!("{key} required")))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("delivery stream '{name}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Firehose::new()));
}
