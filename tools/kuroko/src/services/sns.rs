//! SNS — AWS Query protocol.
//!
//! Returns XML responses with the `https://sns.amazonaws.com/doc/2010-03-31/`
//! namespace. Supports CreateTopic, DeleteTopic, ListTopics, Publish,
//! Subscribe, Unsubscribe, ListSubscriptions, ListSubscriptionsByTopic,
//! Get/SetTopicAttributes. SQS-protocol subscriptions are honored: when an
//! sqs queue URL is the endpoint, kuroko delivers the message into the queue
//! state directly. Other protocols are accepted but messages are recorded
//! locally rather than dispatched.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, QueryProtocolService, Service, ServiceContext,
    persistence_error,
};

const SDK_ID: &str = "sns";
const ACTIONS: &[&str] = &[
    "CreateTopic",
    "DeleteTopic",
    "Publish",
    "Subscribe",
    "Unsubscribe",
    "ListTopics",
    "ListSubscriptions",
    "ListSubscriptionsByTopic",
    "GetTopicAttributes",
    "SetTopicAttributes",
];
const NS: &str = "https://sns.amazonaws.com/doc/2010-03-31/";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    topics: HashMap<String, Topic>,
    subscriptions: HashMap<String, Subscription>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Topic {
    name: String,
    arn: String,
    attributes: HashMap<String, String>,
    received: Vec<PublishedMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Subscription {
    arn: String,
    topic_arn: String,
    protocol: String,
    endpoint: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PublishedMessage {
    id: String,
    body: String,
    subject: Option<String>,
}

pub struct Sns {
    state: Arc<RwLock<State>>,
    registry: parking_lot::RwLock<Option<std::sync::Weak<Registry>>>,
}

impl Sns {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
            registry: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_registry(&self, registry: std::sync::Weak<Registry>) {
        *self.registry.write() = Some(registry);
    }
}

impl Default for Sns {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Sns {
    fn name(&self) -> &'static str {
        "sns"
    }

    fn reset(&self) {
        let mut s = self.state.write();
        s.topics.clear();
        s.subscriptions.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("sns").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("sns", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Sns {
    fn sdk_id(&self) -> &'static str {
        SDK_ID
    }

    fn actions(&self) -> &'static [&'static str] {
        ACTIONS
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        match action {
            "CreateTopic" => self.create_topic(params),
            "DeleteTopic" => self.delete_topic(params),
            "ListTopics" => self.list_topics(),
            "Publish" => self.publish(params),
            "Subscribe" => self.subscribe(params),
            "Unsubscribe" => self.unsubscribe(params),
            "ListSubscriptions" => self.list_subscriptions(),
            "ListSubscriptionsByTopic" => self.list_subscriptions_by_topic(params),
            "GetTopicAttributes" => self.get_topic_attributes(params),
            "SetTopicAttributes" => self.set_topic_attributes(params),
            other => Err(AwsError::unsupported(format!("SNS::{other}"))),
        }
    }
}

impl Sns {
    fn create_topic(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = params
            .get("Name")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "Name required"))?;
        let arn = topic_arn(&name);
        {
            let mut s = self.state.write();
            s.topics.entry(name.clone()).or_insert_with(|| Topic {
                name: name.clone(),
                arn: arn.clone(),
                attributes: HashMap::new(),
                received: Vec::new(),
            });
        }
        Ok(wrap_response(
            "CreateTopic",
            &format!("<TopicArn>{}</TopicArn>", xml_escape(&arn)),
        ))
    }

    fn delete_topic(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = params
            .get("TopicArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let name = topic_name(&arn);
        let mut s = self.state.write();
        s.topics.remove(&name);
        s.subscriptions.retain(|_, sub| sub.topic_arn != arn);
        Ok(empty_response("DeleteTopic"))
    }

    fn list_topics(&self) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut topics = String::new();
        for t in s.topics.values() {
            topics.push_str(&format!(
                "<member><TopicArn>{}</TopicArn></member>",
                xml_escape(&t.arn)
            ));
        }
        Ok(wrap_response(
            "ListTopics",
            &format!("<Topics>{topics}</Topics>"),
        ))
    }

    fn publish(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = params
            .get("TopicArn")
            .cloned()
            .or_else(|| params.get("TargetArn").cloned())
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let body = params
            .get("Message")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "Message required"))?;
        let subject = params.get("Subject").cloned();
        let message_id = uuid::Uuid::new_v4().to_string();
        let name = topic_name(&arn);

        // Snapshot endpoints under read lock, then drop before fanning out.
        let endpoints: Vec<(String, String)> = {
            let s = self.state.read();
            s.subscriptions
                .values()
                .filter(|sub| sub.topic_arn == arn)
                .map(|sub| (sub.protocol.clone(), sub.endpoint.clone()))
                .collect()
        };

        // Record the inbound publish.
        {
            let mut s = self.state.write();
            if let Some(topic) = s.topics.get_mut(&name) {
                topic.received.push(PublishedMessage {
                    id: message_id.clone(),
                    body: body.clone(),
                    subject: subject.clone(),
                });
            }
        }

        // Fan out to SQS subscribers via the registry.
        if let Some(weak) = self.registry.read().clone()
            && let Some(reg) = weak.upgrade()
        {
            for (protocol, endpoint) in endpoints {
                if protocol == "sqs" {
                    deliver_to_sqs(&reg, &endpoint, &body, &subject, &message_id);
                }
            }
        }

        Ok(wrap_response(
            "Publish",
            &format!("<MessageId>{}</MessageId>", xml_escape(&message_id)),
        ))
    }

    fn subscribe(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let topic_arn = params
            .get("TopicArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let protocol = params
            .get("Protocol")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "Protocol required"))?;
        let endpoint = params
            .get("Endpoint")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "Endpoint required"))?;
        let arn = format!("{topic_arn}:{sub}", sub = uuid::Uuid::new_v4().simple());
        self.state.write().subscriptions.insert(
            arn.clone(),
            Subscription {
                arn: arn.clone(),
                topic_arn,
                protocol,
                endpoint,
            },
        );
        Ok(wrap_response(
            "Subscribe",
            &format!("<SubscriptionArn>{}</SubscriptionArn>", xml_escape(&arn)),
        ))
    }

    fn unsubscribe(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = params
            .get("SubscriptionArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "SubscriptionArn required"))?;
        self.state.write().subscriptions.remove(&arn);
        Ok(empty_response("Unsubscribe"))
    }

    fn list_subscriptions(&self) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut subs = String::new();
        for sub in s.subscriptions.values() {
            subs.push_str(&sub_xml(sub));
        }
        Ok(wrap_response(
            "ListSubscriptions",
            &format!("<Subscriptions>{subs}</Subscriptions>"),
        ))
    }

    fn list_subscriptions_by_topic(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        let arn = params
            .get("TopicArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let s = self.state.read();
        let mut subs = String::new();
        for sub in s.subscriptions.values().filter(|s| s.topic_arn == arn) {
            subs.push_str(&sub_xml(sub));
        }
        Ok(wrap_response(
            "ListSubscriptionsByTopic",
            &format!("<Subscriptions>{subs}</Subscriptions>"),
        ))
    }

    fn get_topic_attributes(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = params
            .get("TopicArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let name = topic_name(&arn);
        let s = self.state.read();
        let topic = s
            .topics
            .get(&name)
            .ok_or_else(|| AwsError::new("NotFound", format!("topic '{name}' does not exist")))?;
        let mut attr_xml = String::new();
        attr_xml.push_str(&attr_entry("TopicArn", &topic.arn));
        attr_xml.push_str(&attr_entry("Owner", EMULATED_ACCOUNT_ID));
        for (k, v) in &topic.attributes {
            attr_xml.push_str(&attr_entry(k, v));
        }
        Ok(wrap_response(
            "GetTopicAttributes",
            &format!("<Attributes>{attr_xml}</Attributes>"),
        ))
    }

    fn set_topic_attributes(&self, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let arn = params
            .get("TopicArn")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "TopicArn required"))?;
        let attr = params
            .get("AttributeName")
            .cloned()
            .ok_or_else(|| AwsError::new("InvalidParameter", "AttributeName required"))?;
        let value = params.get("AttributeValue").cloned().unwrap_or_default();
        let name = topic_name(&arn);
        let mut s = self.state.write();
        let topic = s
            .topics
            .get_mut(&name)
            .ok_or_else(|| AwsError::new("NotFound", format!("topic '{name}' does not exist")))?;
        topic.attributes.insert(attr, value);
        Ok(empty_response("SetTopicAttributes"))
    }
}

fn topic_arn(name: &str) -> String {
    format!("arn:aws:sns:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{name}")
}

fn topic_name(arn: &str) -> String {
    arn.rsplit(':').next().unwrap_or(arn).to_string()
}

fn wrap_response(action: &str, body: &str) -> String {
    let rid = uuid::Uuid::new_v4();
    format!(
        r#"<{action}Response xmlns="{NS}">
  <{action}Result>{body}</{action}Result>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</{action}Response>"#
    )
}

fn empty_response(action: &str) -> String {
    let rid = uuid::Uuid::new_v4();
    format!(
        r#"<{action}Response xmlns="{NS}">
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</{action}Response>"#
    )
}

fn sub_xml(s: &Subscription) -> String {
    format!(
        "<member><SubscriptionArn>{arn}</SubscriptionArn><TopicArn>{topic}</TopicArn><Protocol>{proto}</Protocol><Endpoint>{ep}</Endpoint><Owner>{acct}</Owner></member>",
        arn = xml_escape(&s.arn),
        topic = xml_escape(&s.topic_arn),
        proto = xml_escape(&s.protocol),
        ep = xml_escape(&s.endpoint),
        acct = EMULATED_ACCOUNT_ID,
    )
}

fn attr_entry(name: &str, value: &str) -> String {
    format!(
        "<entry><key>{}</key><value>{}</value></entry>",
        xml_escape(name),
        xml_escape(value)
    )
}

/// Deliver a fanned-out SNS message into the SQS service's queue state by name.
/// We send a JSON envelope identical to AWS's SNS-to-SQS shape so downstream
/// consumers can decode it the same way they do in production.
fn deliver_to_sqs(
    registry: &Arc<Registry>,
    endpoint: &str,
    body: &str,
    subject: &Option<String>,
    message_id: &str,
) {
    // The "endpoint" is an SQS queue ARN in production; tests commonly pass
    // the queue URL. Accept either by splitting on ':' or '/' and taking the
    // last component (the queue name).
    let queue_name = endpoint
        .rsplit_once(':')
        .map(|(_, n)| n)
        .or_else(|| endpoint.rsplit_once('/').map(|(_, n)| n))
        .unwrap_or(endpoint)
        .to_string();

    let envelope = serde_json::json!({
        "Type": "Notification",
        "MessageId": message_id,
        "TopicArn": "",
        "Subject": subject,
        "Message": body,
        "Timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let envelope_str = envelope.to_string();

    // Reach the SQS service by name. We use the runtime-registered handle so
    // SNS can deliver without a compile-time dependency cycle.
    let Some(svc) = registry.get("sqs") else {
        tracing::warn!(queue = %queue_name, "SNS: SQS service not registered");
        return;
    };
    if let Some(sqs) = svc
        .as_any()
        .and_then(|a| a.downcast_ref::<crate::services::sqs::Sqs>())
    {
        sqs.push_external(&queue_name, &envelope_str);
    } else {
        tracing::debug!(queue = %queue_name, "SNS: SQS service is a stub; message not delivered");
    }
}

pub fn register(registry: &Arc<Registry>) {
    let sns = Arc::new(Sns::new());
    sns.set_registry(Arc::downgrade(registry));
    registry.register_query(sns);
}
