//! SES (v1) — AWS Query protocol, sdk_id `email`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{QueryProtocolService, Service, ServiceContext, persistence_error};

const SDK_ID: &str = "email";
const NS: &str = "http://ses.amazonaws.com/doc/2010-12-01/";

const ACTIONS: &[&str] = &[
    "SendEmail",
    "SendRawEmail",
    "VerifyEmailIdentity",
    "ListIdentities",
    "GetIdentityVerificationAttributes",
    "DeleteIdentity",
];

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    identities: HashMap<String, IdentityStatus>,
    sent: Vec<SentEmail>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IdentityStatus {
    verified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SentEmail {
    message_id: String,
    from: String,
    to: Vec<String>,
    subject: String,
    body: String,
}

pub struct Ses {
    state: Arc<RwLock<State>>,
}

impl Ses {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}
impl Default for Ses {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ses {
    fn name(&self) -> &'static str {
        "ses"
    }
    fn reset(&self) {
        *self.state.write() = State::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ses").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("ses", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Ses {
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
            "VerifyEmailIdentity" => {
                let email = required(params, "EmailAddress")?;
                self.state
                    .write()
                    .identities
                    .insert(email, IdentityStatus { verified: true });
                Ok(empty("VerifyEmailIdentity"))
            }
            "ListIdentities" => {
                let s = self.state.read();
                let members: String = s
                    .identities
                    .keys()
                    .map(|i| format!("<member>{}</member>", xml_escape(i)))
                    .collect();
                Ok(wrap(
                    "ListIdentities",
                    &format!("<Identities>{members}</Identities>"),
                ))
            }
            "GetIdentityVerificationAttributes" => {
                let mut entries = String::new();
                let mut i = 1;
                let s = self.state.read();
                loop {
                    let key = format!("Identities.member.{i}");
                    let Some(id) = params.get(&key) else { break };
                    let status = s
                        .identities
                        .get(id)
                        .map(|st| if st.verified { "Success" } else { "Pending" })
                        .unwrap_or("NotStarted");
                    entries.push_str(&format!(
                        "<entry><key>{}</key><value><VerificationStatus>{}</VerificationStatus></value></entry>",
                        xml_escape(id),
                        status
                    ));
                    i += 1;
                }
                Ok(wrap(
                    "GetIdentityVerificationAttributes",
                    &format!("<VerificationAttributes>{entries}</VerificationAttributes>"),
                ))
            }
            "DeleteIdentity" => {
                let email = required(params, "Identity")?;
                self.state.write().identities.remove(&email);
                Ok(empty("DeleteIdentity"))
            }
            "SendEmail" => {
                let from = required(params, "Source")?;
                let to = required(params, "Destination.ToAddresses.member.1").unwrap_or_default();
                let subject = required(params, "Message.Subject.Data").unwrap_or_default();
                let body = required(params, "Message.Body.Text.Data")
                    .or_else(|_| required(params, "Message.Body.Html.Data"))
                    .unwrap_or_default();
                let message_id = Uuid::new_v4().to_string();
                let s = self.state.read();
                let from_verified = s
                    .identities
                    .get(&from)
                    .map(|st| st.verified)
                    .unwrap_or(false);
                drop(s);
                if !from_verified {
                    return Err(AwsError::new(
                        "MessageRejected",
                        format!("Email address is not verified: {from}"),
                    ));
                }
                self.state.write().sent.push(SentEmail {
                    message_id: message_id.clone(),
                    from,
                    to: vec![to],
                    subject,
                    body,
                });
                Ok(wrap(
                    "SendEmail",
                    &format!("<MessageId>{}</MessageId>", xml_escape(&message_id)),
                ))
            }
            "SendRawEmail" => {
                let message_id = Uuid::new_v4().to_string();
                Ok(wrap(
                    "SendRawEmail",
                    &format!("<MessageId>{}</MessageId>", xml_escape(&message_id)),
                ))
            }
            other => Err(AwsError::unsupported(format!("SES::{other}"))),
        }
    }
}

fn required(p: &HashMap<String, String>, key: &str) -> Result<String, AwsError> {
    p.get(key)
        .cloned()
        .ok_or_else(|| AwsError::new("MissingParameter", format!("{key} required")))
}

fn wrap(action: &str, body: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><{action}Result>{body}</{action}Result><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

fn empty(action: &str) -> String {
    let rid = Uuid::new_v4();
    format!(
        "<{action}Response xmlns=\"{NS}\"><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
    )
}

pub fn register(registry: &Arc<Registry>) {
    registry.register_query(Arc::new(Ses::new()));
}
