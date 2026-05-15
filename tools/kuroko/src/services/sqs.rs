//! SQS — AWS JSON 1.0 protocol via `X-Amz-Target: AmazonSQS.<Action>`.
//!
//! Covers the API surface most queue-based test workloads exercise:
//! CreateQueue, ListQueues, DeleteQueue, GetQueueUrl, GetQueueAttributes,
//! SendMessage, SendMessageBatch, ReceiveMessage, DeleteMessage,
//! DeleteMessageBatch, PurgeQueue, TagQueue, ListQueueTags.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{JsonProtocolService, Service, ServiceContext};

const TARGET_PREFIX: &str = "AmazonSQS";

use crate::service::{EMULATED_ACCOUNT_ID, EMULATED_REGION};

#[derive(Debug, Default)]
struct State {
    queues: HashMap<String, Queue>,
}

#[derive(Debug)]
struct Queue {
    name: String,
    url: String,
    attributes: HashMap<String, String>,
    tags: HashMap<String, String>,
    messages: VecDeque<Message>,
    /// Receipt handle → (message, visibility deadline).
    inflight: HashMap<String, (Message, Instant)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Message {
    message_id: String,
    body: String,
    receipt_handle: String,
    md5_of_body: String,
}

// In-flight visibility timers don't persist — on restore, messages return to
// the visible queue. This matches LocalStack's "best effort" behavior.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    queues: HashMap<String, PersistedQueue>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PersistedQueue {
    name: String,
    url: String,
    attributes: HashMap<String, String>,
    tags: HashMap<String, String>,
    messages: VecDeque<Message>,
}

impl From<&State> for PersistedState {
    fn from(s: &State) -> Self {
        let mut queues = HashMap::new();
        for (k, q) in &s.queues {
            let mut messages = q.messages.clone();
            // Recover in-flight messages so a snapshot taken mid-receive is
            // self-consistent.
            for (msg, _) in q.inflight.values() {
                messages.push_back(msg.clone());
            }
            queues.insert(
                k.clone(),
                PersistedQueue {
                    name: q.name.clone(),
                    url: q.url.clone(),
                    attributes: q.attributes.clone(),
                    tags: q.tags.clone(),
                    messages,
                },
            );
        }
        Self { queues }
    }
}

impl From<PersistedState> for State {
    fn from(p: PersistedState) -> Self {
        let queues = p
            .queues
            .into_iter()
            .map(|(k, q)| {
                (
                    k,
                    Queue {
                        name: q.name,
                        url: q.url,
                        attributes: q.attributes,
                        tags: q.tags,
                        messages: q.messages,
                        inflight: HashMap::new(),
                    },
                )
            })
            .collect();
        Self { queues }
    }
}

pub struct Sqs {
    state: Arc<RwLock<State>>,
}

impl Sqs {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Sqs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Sqs {
    fn name(&self) -> &'static str {
        "sqs"
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn reset(&self) {
        self.state.write().queues.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<PersistedState>("sqs")
                .map_err(crate::service::persistence_error)?
        {
            *self.state.write() = data.into();
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = PersistedState::from(&*self.state.read());
            snap.save("sqs", &data)
                .map_err(crate::service::persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Sqs {
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
            "CreateQueue" => self.create_queue(&req),
            "ListQueues" => self.list_queues(&req),
            "DeleteQueue" => self.delete_queue(&req),
            "GetQueueUrl" => self.get_queue_url(&req),
            "GetQueueAttributes" => self.get_queue_attributes(&req),
            "SetQueueAttributes" => self.set_queue_attributes(&req),
            "SendMessage" => self.send_message(&req),
            "SendMessageBatch" => self.send_message_batch(&req),
            "ReceiveMessage" => self.receive_message(&req),
            "DeleteMessage" => self.delete_message(&req),
            "DeleteMessageBatch" => self.delete_message_batch(&req),
            "PurgeQueue" => self.purge_queue(&req),
            "TagQueue" => self.tag_queue(&req),
            "UntagQueue" => self.untag_queue(&req),
            "ListQueueTags" => self.list_queue_tags(&req),
            "ChangeMessageVisibility" => Ok(json!({})),
            other => Err(AwsError::unsupported(format!("SQS::{other}"))),
        }
    }
}

impl Sqs {
    fn queue_url(&self, name: &str) -> String {
        format!("http://kuroko/queue/{name}")
    }

    /// Append a message to a queue from outside the SQS dispatch path. Used
    /// by SNS-to-SQS fanout. The queue must already exist; otherwise this is
    /// a no-op with a warning.
    pub fn push_external(&self, queue_name: &str, body: &str) {
        let mut s = self.state.write();
        let Some(q) = s.queues.get_mut(queue_name) else {
            tracing::warn!(
                queue = queue_name,
                "SQS push_external: queue does not exist"
            );
            return;
        };
        q.messages.push_back(make_message(body.to_string()));
    }

    fn create_queue(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("QueueName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueName is required"))?
            .to_string();
        let attrs = parse_string_map(req.get("Attributes"));
        // AWS sends `Tags` (capital T) on CreateQueue; legacy / lowercase
        // form is tolerated for older callers.
        let tags = parse_string_map(req.get("Tags").or_else(|| req.get("tags")));

        let url = self.queue_url(&name);
        let mut s = self.state.write();
        s.queues.entry(name.clone()).or_insert_with(|| Queue {
            name: name.clone(),
            url: url.clone(),
            attributes: attrs,
            tags,
            messages: VecDeque::new(),
            inflight: HashMap::new(),
        });
        Ok(json!({ "QueueUrl": url }))
    }

    fn list_queues(&self, req: &Value) -> Result<Value, AwsError> {
        let prefix = req
            .get("QueueNamePrefix")
            .and_then(Value::as_str)
            .unwrap_or("");
        let s = self.state.read();
        let urls: Vec<String> = s
            .queues
            .values()
            .filter(|q| q.name.starts_with(prefix))
            .map(|q| q.url.clone())
            .collect();
        Ok(json!({ "QueueUrls": urls }))
    }

    fn delete_queue(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let name = queue_name_from_url(url);
        self.state.write().queues.remove(&name);
        Ok(json!({}))
    }

    fn get_queue_url(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("QueueName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueName required"))?;
        let s = self.state.read();
        match s.queues.get(name) {
            Some(q) => Ok(json!({ "QueueUrl": q.url })),
            None => Err(AwsError::new(
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            )),
        }
    }

    fn get_queue_attributes(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let name = queue_name_from_url(url);
        // Take a write lock so we can reclaim in-flight messages whose
        // visibility timeout has elapsed; otherwise the reported
        // ApproximateNumberOfMessagesNotVisible would be stale until the next
        // ReceiveMessage call (rust #17).
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        reclaim_expired(q);
        let mut attrs = q.attributes.clone();
        attrs.insert(
            "ApproximateNumberOfMessages".into(),
            q.messages.len().to_string(),
        );
        attrs.insert(
            "ApproximateNumberOfMessagesNotVisible".into(),
            q.inflight.len().to_string(),
        );
        attrs.insert(
            "QueueArn".into(),
            format!(
                "arn:aws:sqs:{}:{}:{}",
                EMULATED_REGION, EMULATED_ACCOUNT_ID, q.name
            ),
        );
        Ok(json!({ "Attributes": attrs }))
    }

    fn set_queue_attributes(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let attrs = parse_string_map(req.get("Attributes"));
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        q.attributes.extend(attrs);
        Ok(json!({}))
    }

    fn send_message(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let body = req
            .get("MessageBody")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "MessageBody required"))?
            .to_string();
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        let msg = make_message(body);
        let result = json!({
            "MessageId": msg.message_id,
            "MD5OfMessageBody": msg.md5_of_body,
        });
        q.messages.push_back(msg);
        Ok(result)
    }

    fn send_message_batch(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let entries = req
            .get("Entries")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("MissingParameter", "Entries required"))?;
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        let mut successful = Vec::new();
        for entry in entries {
            let id = entry
                .get("Id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let body = entry
                .get("MessageBody")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let msg = make_message(body);
            successful.push(json!({
                "Id": id,
                "MessageId": msg.message_id,
                "MD5OfMessageBody": msg.md5_of_body,
            }));
            q.messages.push_back(msg);
        }
        Ok(json!({ "Successful": successful, "Failed": Vec::<Value>::new() }))
    }

    fn receive_message(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let max: usize = req
            .get("MaxNumberOfMessages")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .clamp(1, 10) as usize;
        let visibility = req
            .get("VisibilityTimeout")
            .and_then(Value::as_u64)
            .unwrap_or(30);
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;

        reclaim_expired(q);
        let now = Instant::now();

        let mut out = Vec::with_capacity(max);
        for _ in 0..max {
            let Some(msg) = q.messages.pop_front() else {
                break;
            };
            let handle = msg.receipt_handle.clone();
            out.push(json!({
                "MessageId": msg.message_id,
                "Body": msg.body,
                "ReceiptHandle": msg.receipt_handle,
                "MD5OfBody": msg.md5_of_body,
            }));
            q.inflight
                .insert(handle, (msg, now + Duration::from_secs(visibility)));
        }
        Ok(json!({ "Messages": out }))
    }

    fn delete_message(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let receipt = req
            .get("ReceiptHandle")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "ReceiptHandle required"))?;
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        q.inflight.remove(receipt);
        Ok(json!({}))
    }

    fn delete_message_batch(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let entries = req
            .get("Entries")
            .and_then(Value::as_array)
            .ok_or_else(|| AwsError::new("MissingParameter", "Entries required"))?;
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        let mut successful = Vec::new();
        for e in entries {
            let id = e
                .get("Id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if let Some(r) = e.get("ReceiptHandle").and_then(Value::as_str) {
                q.inflight.remove(r);
            }
            successful.push(json!({ "Id": id }));
        }
        Ok(json!({ "Successful": successful, "Failed": Vec::<Value>::new() }))
    }

    fn purge_queue(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        q.messages.clear();
        q.inflight.clear();
        Ok(json!({}))
    }

    fn tag_queue(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let tags = parse_string_map(req.get("Tags"));
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        q.tags.extend(tags);
        Ok(json!({}))
    }

    fn untag_queue(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let keys = req
            .get("TagKeys")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let name = queue_name_from_url(url);
        let mut s = self.state.write();
        let q = s.queues.get_mut(&name).ok_or_else(non_existent_queue)?;
        for k in keys {
            q.tags.remove(&k);
        }
        Ok(json!({}))
    }

    fn list_queue_tags(&self, req: &Value) -> Result<Value, AwsError> {
        let url = req
            .get("QueueUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("MissingParameter", "QueueUrl required"))?;
        let name = queue_name_from_url(url);
        let s = self.state.read();
        let q = s.queues.get(&name).ok_or_else(non_existent_queue)?;
        Ok(json!({ "Tags": q.tags }))
    }
}

fn parse_string_map(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn queue_name_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

fn non_existent_queue() -> AwsError {
    AwsError::new(
        "AWS.SimpleQueueService.NonExistentQueue",
        "The specified queue does not exist.",
    )
}

/// Move every in-flight message whose visibility deadline has elapsed back
/// onto the visible queue. Called from `ReceiveMessage` and
/// `GetQueueAttributes` so the queue counters never report stale values.
fn reclaim_expired(q: &mut Queue) {
    let now = Instant::now();
    let expired: Vec<String> = q
        .inflight
        .iter()
        .filter(|(_, (_, until))| *until <= now)
        .map(|(k, _)| k.clone())
        .collect();
    for k in expired {
        if let Some((msg, _)) = q.inflight.remove(&k) {
            q.messages.push_back(msg);
        }
    }
}

fn make_message(body: String) -> Message {
    let md5 = {
        use md5::Digest;
        let mut h = md5::Md5::new();
        h.update(body.as_bytes());
        hex::encode(h.finalize())
    };
    Message {
        message_id: Uuid::new_v4().to_string(),
        receipt_handle: Uuid::new_v4().simple().to_string(),
        body,
        md5_of_body: md5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_send_receive_roundtrip() {
        let svc = Sqs::new();
        let ctx = ServiceContext::new(None);

        let create = svc
            .dispatch(
                ctx.clone(),
                "CreateQueue",
                Bytes::from(r#"{"QueueName":"q1"}"#),
            )
            .await
            .unwrap();
        let url = create["QueueUrl"].as_str().unwrap().to_string();

        let send_body = format!(r#"{{"QueueUrl":"{url}","MessageBody":"hello"}}"#);
        svc.dispatch(ctx.clone(), "SendMessage", Bytes::from(send_body))
            .await
            .unwrap();

        let recv_body = format!(r#"{{"QueueUrl":"{url}"}}"#);
        let recv = svc
            .dispatch(ctx, "ReceiveMessage", Bytes::from(recv_body))
            .await
            .unwrap();
        assert_eq!(recv["Messages"][0]["Body"], "hello");
    }
}
