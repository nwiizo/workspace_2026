//! Kinesis — AWS JSON 1.1, target prefix `Kinesis_20131202`.
//!
//! Implementation simplifies the data plane to a **single shard per stream**.
//! Records form a sequence-numbered append-only log; GetShardIterator returns
//! the offset into that log, and GetRecords walks forward from the iterator.
//! Enough for tests that exercise the API contract without simulating
//! re-sharding, KCL checkpointing, or per-partition-key ordering.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "Kinesis_20131202";
const SHARD_ID: &str = "shardId-000000000000";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    streams: HashMap<String, Stream>,
    /// Shard iterator token → (stream_name, position).
    iterators: HashMap<String, (String, usize)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Stream {
    name: String,
    arn: String,
    records: Vec<Record>,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Record {
    sequence_number: String,
    partition_key: String,
    data_b64: String,
    approximate_arrival_ts: i64,
}

pub struct Kinesis {
    state: Arc<RwLock<State>>,
}

impl Kinesis {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Kinesis {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Kinesis {
    fn name(&self) -> &'static str {
        "kinesis"
    }

    fn reset(&self) {
        let mut s = self.state.write();
        s.streams.clear();
        s.iterators.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("kinesis").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("kinesis", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Kinesis {
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
            "CreateStream" => self.create_stream(&req),
            "DescribeStream" => self.describe_stream(&req),
            "DescribeStreamSummary" => self.describe_stream_summary(&req),
            "ListStreams" => self.list_streams(&req),
            "DeleteStream" => self.delete_stream(&req),
            "PutRecord" => self.put_record(&req),
            "PutRecords" => self.put_records(&req),
            "GetShardIterator" => self.get_shard_iterator(&req),
            "GetRecords" => self.get_records(&req),
            "ListShards" => self.list_shards(&req),
            other => Err(AwsError::unsupported(format!("Kinesis::{other}"))),
        }
    }
}

impl Kinesis {
    fn create_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("StreamName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "StreamName required"))?
            .to_string();
        let mut s = self.state.write();
        if s.streams.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceInUseException",
                format!("stream '{name}' already exists"),
            ));
        }
        s.streams.insert(
            name.clone(),
            Stream {
                name: name.clone(),
                arn: stream_arn(&name),
                records: Vec::new(),
                created: chrono::Utc::now(),
            },
        );
        Ok(json!({}))
    }

    fn describe_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let s = self.state.read();
        let st = s.streams.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "StreamDescription": {
                "StreamName": st.name,
                "StreamARN": st.arn,
                "StreamStatus": "ACTIVE",
                "Shards": [{
                    "ShardId": SHARD_ID,
                    "HashKeyRange": {
                        "StartingHashKey": "0",
                        "EndingHashKey": "340282366920938463463374607431768211455",
                    },
                    "SequenceNumberRange": {
                        "StartingSequenceNumber": "0",
                    },
                }],
                "HasMoreShards": false,
                "RetentionPeriodHours": 24,
                "StreamCreationTimestamp": st.created.timestamp(),
                "EnhancedMonitoring": [],
                "EncryptionType": "NONE",
            }
        }))
    }

    fn describe_stream_summary(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let s = self.state.read();
        let st = s.streams.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "StreamDescriptionSummary": {
                "StreamName": st.name,
                "StreamARN": st.arn,
                "StreamStatus": "ACTIVE",
                "RetentionPeriodHours": 24,
                "StreamCreationTimestamp": st.created.timestamp(),
                "EnhancedMonitoring": [],
                "EncryptionType": "NONE",
                "OpenShardCount": 1,
                "ConsumerCount": 0,
            }
        }))
    }

    fn list_streams(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let names: Vec<&str> = s.streams.keys().map(String::as_str).collect();
        let summaries: Vec<_> = s
            .streams
            .values()
            .map(|st| {
                json!({
                    "StreamName": st.name,
                    "StreamARN": st.arn,
                    "StreamStatus": "ACTIVE",
                    "StreamModeDetails": { "StreamMode": "PROVISIONED" },
                    "StreamCreationTimestamp": st.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({
            "StreamNames": names,
            "HasMoreStreams": false,
            "StreamSummaries": summaries,
        }))
    }

    fn delete_stream(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        self.state
            .write()
            .streams
            .remove(&name)
            .ok_or_else(|| not_found(&name))?;
        Ok(json!({}))
    }

    fn put_record(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let data_b64 = req
            .get("Data")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "Data required"))?
            .to_string();
        let partition_key = req
            .get("PartitionKey")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut s = self.state.write();
        let st = s.streams.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let seq = format!("{:020}", st.records.len());
        st.records.push(Record {
            sequence_number: seq.clone(),
            partition_key,
            data_b64,
            approximate_arrival_ts: chrono::Utc::now().timestamp_millis(),
        });
        Ok(json!({
            "ShardId": SHARD_ID,
            "SequenceNumber": seq,
            "EncryptionType": "NONE",
        }))
    }

    fn put_records(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let records = req
            .get("Records")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "Records required"))?;
        let mut s = self.state.write();
        let st = s.streams.get_mut(&name).ok_or_else(|| not_found(&name))?;
        let mut results = Vec::with_capacity(records.len());
        for r in records {
            let data = r
                .get("Data")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let pk = r
                .get("PartitionKey")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let seq = format!("{:020}", st.records.len());
            st.records.push(Record {
                sequence_number: seq.clone(),
                partition_key: pk,
                data_b64: data,
                approximate_arrival_ts: chrono::Utc::now().timestamp_millis(),
            });
            results.push(json!({
                "ShardId": SHARD_ID,
                "SequenceNumber": seq,
            }));
        }
        Ok(json!({
            "FailedRecordCount": 0,
            "Records": results,
            "EncryptionType": "NONE",
        }))
    }

    fn get_shard_iterator(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let iter_type = req
            .get("ShardIteratorType")
            .and_then(Value::as_str)
            .unwrap_or("TRIM_HORIZON");
        let starting_seq = req.get("StartingSequenceNumber").and_then(Value::as_str);
        let mut s = self.state.write();
        let st = s.streams.get(&name).ok_or_else(|| not_found(&name))?;
        let position = match iter_type {
            "TRIM_HORIZON" => 0,
            "LATEST" => st.records.len(),
            "AT_SEQUENCE_NUMBER" | "AFTER_SEQUENCE_NUMBER" => {
                let target: usize = starting_seq.and_then(|v| v.parse().ok()).unwrap_or(0);
                if iter_type == "AFTER_SEQUENCE_NUMBER" {
                    target + 1
                } else {
                    target
                }
            }
            _ => 0,
        };
        let token = uuid::Uuid::new_v4().simple().to_string();
        s.iterators.insert(token.clone(), (name, position));
        Ok(json!({ "ShardIterator": token }))
    }

    fn get_records(&self, req: &Value) -> Result<Value, AwsError> {
        let token = req
            .get("ShardIterator")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidArgumentException", "ShardIterator required"))?
            .to_string();
        let limit = req.get("Limit").and_then(Value::as_u64).unwrap_or(10_000) as usize;
        let mut s = self.state.write();
        let (stream_name, position) = s
            .iterators
            .get(&token)
            .cloned()
            .ok_or_else(|| AwsError::new("ExpiredIteratorException", "iterator expired"))?;
        let st = s
            .streams
            .get(&stream_name)
            .ok_or_else(|| not_found(&stream_name))?;
        let end = (position + limit).min(st.records.len());
        let records: Vec<_> = st.records[position..end]
            .iter()
            .map(|r| {
                json!({
                    "SequenceNumber": r.sequence_number,
                    "ApproximateArrivalTimestamp": r.approximate_arrival_ts as f64 / 1000.0,
                    "Data": r.data_b64,
                    "PartitionKey": r.partition_key,
                })
            })
            .collect();
        let next_token = uuid::Uuid::new_v4().simple().to_string();
        s.iterators.insert(next_token.clone(), (stream_name, end));
        Ok(json!({
            "Records": records,
            "NextShardIterator": next_token,
            "MillisBehindLatest": 0,
        }))
    }

    fn list_shards(&self, req: &Value) -> Result<Value, AwsError> {
        let name = stream_name(req)?;
        let s = self.state.read();
        s.streams.get(&name).ok_or_else(|| not_found(&name))?;
        Ok(json!({
            "Shards": [{
                "ShardId": SHARD_ID,
                "HashKeyRange": {
                    "StartingHashKey": "0",
                    "EndingHashKey": "340282366920938463463374607431768211455",
                },
                "SequenceNumberRange": {
                    "StartingSequenceNumber": "0",
                },
            }]
        }))
    }
}

fn stream_name(req: &Value) -> Result<String, AwsError> {
    req.get("StreamName")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("InvalidArgumentException", "StreamName required"))
}

fn stream_arn(name: &str) -> String {
    format!("arn:aws:kinesis:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:stream/{name}")
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("stream '{name}' not found"),
    )
}

/// Decode base64 in tests — unused by the dispatch path itself, but available
/// for cross-service consumers.
#[allow(dead_code)]
fn decode_data(b64: &str) -> Vec<u8> {
    BASE64.decode(b64.as_bytes()).unwrap_or_default()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Kinesis::new()));
}
