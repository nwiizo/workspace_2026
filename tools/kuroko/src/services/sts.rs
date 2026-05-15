//! STS — AWS Query protocol. Returns XML responses with the
//! `https://sts.amazonaws.com/doc/2011-06-15/` namespace.
//!
//! Implementations are stateless and deterministic — STS in kuroko is a
//! "yes you're who you say you are" stub so SDKs that call GetCallerIdentity
//! during bootstrapping succeed.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::aws_error::{AwsError, xml_escape};
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, QueryProtocolService, Service, ServiceContext,
};

const SDK_ID: &str = "sts";
const ACTIONS: &[&str] = &[
    "GetCallerIdentity",
    "AssumeRole",
    "GetSessionToken",
    "AssumeRoleWithWebIdentity",
    "DecodeAuthorizationMessage",
];

pub struct Sts;

impl Sts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Sts {
    fn name(&self) -> &'static str {
        "sts"
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for Sts {
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
            "GetCallerIdentity" => Ok(get_caller_identity_response()),
            "AssumeRole" => Ok(assume_role_response(params)),
            "GetSessionToken" => Ok(get_session_token_response()),
            "AssumeRoleWithWebIdentity" => Ok(assume_role_with_web_identity_response(params)),
            "DecodeAuthorizationMessage" => Ok(decode_authorization_message_response(params)),
            other => Err(AwsError::unsupported(format!("STS::{other}"))),
        }
    }
}

const NS: &str = "https://sts.amazonaws.com/doc/2011-06-15/";

fn caller_arn() -> String {
    format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:user/kuroko")
}

fn get_caller_identity_response() -> String {
    let rid = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<GetCallerIdentityResponse xmlns="{ns}">
  <GetCallerIdentityResult>
    <Arn>{arn}</Arn>
    <UserId>AIDAKUROKOEMULATOR</UserId>
    <Account>{acct}</Account>
  </GetCallerIdentityResult>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</GetCallerIdentityResponse>"#,
        ns = NS,
        arn = xml_escape(&caller_arn()),
        acct = EMULATED_ACCOUNT_ID,
        rid = rid,
    )
}

fn assume_role_response(params: &HashMap<String, String>) -> String {
    let role_arn = params
        .get("RoleArn")
        .cloned()
        .unwrap_or_else(|| format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:role/kuroko"));
    let session_name = params
        .get("RoleSessionName")
        .cloned()
        .unwrap_or_else(|| "kuroko-session".into());
    let duration = params
        .get("DurationSeconds")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(3600);
    let expiry = (chrono::Utc::now() + chrono::Duration::seconds(duration))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let rid = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<AssumeRoleResponse xmlns="{ns}">
  <AssumeRoleResult>
    <Credentials>
      <AccessKeyId>ASIA{access}</AccessKeyId>
      <SecretAccessKey>{secret}</SecretAccessKey>
      <SessionToken>{token}</SessionToken>
      <Expiration>{expiry}</Expiration>
    </Credentials>
    <AssumedRoleUser>
      <Arn>{role_arn}/{session_name}</Arn>
      <AssumedRoleId>AROAKUROKO:{session_name}</AssumedRoleId>
    </AssumedRoleUser>
  </AssumeRoleResult>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</AssumeRoleResponse>"#,
        ns = NS,
        access = random_access_id(),
        secret = random_secret(),
        token = random_session_token(),
        expiry = expiry,
        role_arn = xml_escape(&role_arn),
        session_name = xml_escape(&session_name),
        rid = rid,
    )
}

fn get_session_token_response() -> String {
    let expiry = (chrono::Utc::now() + chrono::Duration::hours(12))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let rid = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<GetSessionTokenResponse xmlns="{ns}">
  <GetSessionTokenResult>
    <Credentials>
      <AccessKeyId>ASIA{access}</AccessKeyId>
      <SecretAccessKey>{secret}</SecretAccessKey>
      <SessionToken>{token}</SessionToken>
      <Expiration>{expiry}</Expiration>
    </Credentials>
  </GetSessionTokenResult>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</GetSessionTokenResponse>"#,
        ns = NS,
        access = random_access_id(),
        secret = random_secret(),
        token = random_session_token(),
        expiry = expiry,
        rid = rid,
    )
}

fn assume_role_with_web_identity_response(params: &HashMap<String, String>) -> String {
    let role_arn = params
        .get("RoleArn")
        .cloned()
        .unwrap_or_else(|| format!("arn:aws:iam::{EMULATED_ACCOUNT_ID}:role/web"));
    let session_name = params
        .get("RoleSessionName")
        .cloned()
        .unwrap_or_else(|| "kuroko-web-session".into());
    let expiry = (chrono::Utc::now() + chrono::Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let rid = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<AssumeRoleWithWebIdentityResponse xmlns="{ns}">
  <AssumeRoleWithWebIdentityResult>
    <Credentials>
      <AccessKeyId>ASIA{access}</AccessKeyId>
      <SecretAccessKey>{secret}</SecretAccessKey>
      <SessionToken>{token}</SessionToken>
      <Expiration>{expiry}</Expiration>
    </Credentials>
    <SubjectFromWebIdentityToken>kuroko-web</SubjectFromWebIdentityToken>
    <AssumedRoleUser>
      <Arn>{role_arn}/{session_name}</Arn>
      <AssumedRoleId>AROAKUROKO:{session_name}</AssumedRoleId>
    </AssumedRoleUser>
    <Provider>kuroko</Provider>
  </AssumeRoleWithWebIdentityResult>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</AssumeRoleWithWebIdentityResponse>"#,
        ns = NS,
        access = random_access_id(),
        secret = random_secret(),
        token = random_session_token(),
        expiry = expiry,
        role_arn = xml_escape(&role_arn),
        session_name = xml_escape(&session_name),
        rid = rid,
    )
}

fn decode_authorization_message_response(params: &HashMap<String, String>) -> String {
    let msg = params.get("EncodedMessage").cloned().unwrap_or_default();
    let decoded = BASE64
        .decode(msg.as_bytes())
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_else(|_| msg.clone());
    let rid = uuid::Uuid::new_v4().to_string();
    format!(
        r#"<DecodeAuthorizationMessageResponse xmlns="{ns}">
  <DecodeAuthorizationMessageResult>
    <DecodedMessage>{decoded}</DecodedMessage>
  </DecodeAuthorizationMessageResult>
  <ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata>
</DecodeAuthorizationMessageResponse>"#,
        ns = NS,
        decoded = xml_escape(&decoded),
        rid = rid,
    )
}

fn random_access_id() -> String {
    use rand::Rng;
    (0..16)
        .map(|_| {
            let c = rand::thread_rng().gen_range(0u8..26);
            (b'A' + c) as char
        })
        .collect()
}

fn random_secret() -> String {
    use rand::Rng;
    (0..40)
        .map(|_| {
            let c = rand::thread_rng().gen_range(0u8..36);
            if c < 10 {
                (b'0' + c) as char
            } else {
                (b'a' + (c - 10)) as char
            }
        })
        .collect()
}

fn random_session_token() -> String {
    use rand::Rng;
    (0..64)
        .map(|_| {
            let c = rand::thread_rng().gen_range(0u8..36);
            if c < 10 {
                (b'0' + c) as char
            } else {
                (b'a' + (c - 10)) as char
            }
        })
        .collect()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    let _ = EMULATED_REGION; // suppress unused-imports if region inlined into formats above.
    registry.register_query(Arc::new(Sts::new()));
}
