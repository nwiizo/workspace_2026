//! KMS — AWS JSON 1.1 protocol via `X-Amz-Target: TrentService.<Action>`.
//!
//! Implements the operations most-commonly hit in CI: CreateKey, ListKeys,
//! DescribeKey, Encrypt, Decrypt, GenerateDataKey, ScheduleKeyDeletion. The
//! cipher is **NOT** secure — the emulator XOR-wraps the plaintext with the
//! key material as a deterministic, reversible stand-in. This matches what
//! LocalStack does in its community tier and is documented as such.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "TrentService";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    keys: HashMap<String, KmsKey>,
    aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KmsKey {
    key_id: String,
    arn: String,
    description: String,
    /// 256-bit (32-byte) symmetric material. Base64 in the JSON snapshot.
    material_b64: String,
    enabled: bool,
    deletion_date: Option<chrono::DateTime<chrono::Utc>>,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct Kms {
    state: Arc<RwLock<State>>,
}

impl Kms {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Kms {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Kms {
    fn name(&self) -> &'static str {
        "kms"
    }

    fn reset(&self) {
        let mut s = self.state.write();
        s.keys.clear();
        s.aliases.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("kms").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("kms", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Kms {
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
            "CreateKey" => self.create_key(&req),
            "ListKeys" => self.list_keys(&req),
            "DescribeKey" => self.describe_key(&req),
            "CreateAlias" => self.create_alias(&req),
            "DeleteAlias" => self.delete_alias(&req),
            "ListAliases" => self.list_aliases(&req),
            "Encrypt" => self.encrypt(&req),
            "Decrypt" => self.decrypt(&req),
            "GenerateDataKey" => self.generate_data_key(&req),
            "EnableKey" => self.set_enabled(&req, true),
            "DisableKey" => self.set_enabled(&req, false),
            "ScheduleKeyDeletion" => self.schedule_key_deletion(&req),
            "CancelKeyDeletion" => self.cancel_key_deletion(&req),
            other => Err(AwsError::unsupported(format!("KMS::{other}"))),
        }
    }
}

impl Kms {
    fn create_key(&self, req: &Value) -> Result<Value, AwsError> {
        let description = req
            .get("Description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let key_id = Uuid::new_v4().to_string();
        let arn = format!("arn:aws:kms:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:key/{key_id}");
        let material = random_bytes(32);
        let key = KmsKey {
            key_id: key_id.clone(),
            arn,
            description,
            material_b64: BASE64.encode(&material),
            enabled: true,
            deletion_date: None,
            created: chrono::Utc::now(),
        };
        self.state.write().keys.insert(key_id.clone(), key.clone());
        Ok(json!({ "KeyMetadata": key_metadata(&key) }))
    }

    fn list_keys(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let keys: Vec<_> = s
            .keys
            .values()
            .map(|k| json!({ "KeyId": k.key_id, "KeyArn": k.arn }))
            .collect();
        Ok(json!({ "Keys": keys }))
    }

    fn describe_key(&self, req: &Value) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let s = self.state.read();
        let key = resolve_key(&s, &id)?;
        Ok(json!({ "KeyMetadata": key_metadata(key) }))
    }

    fn create_alias(&self, req: &Value) -> Result<Value, AwsError> {
        let alias = req
            .get("AliasName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "AliasName required"))?
            .to_string();
        if !alias.starts_with("alias/") {
            return Err(AwsError::new(
                "ValidationException",
                "AliasName must start with 'alias/'",
            ));
        }
        let target = req
            .get("TargetKeyId")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "TargetKeyId required"))?
            .to_string();
        let mut s = self.state.write();
        resolve_key(&s, &target)?; // verify target exists
        s.aliases.insert(alias, target);
        Ok(json!({}))
    }

    fn delete_alias(&self, req: &Value) -> Result<Value, AwsError> {
        let alias = req
            .get("AliasName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "AliasName required"))?;
        self.state.write().aliases.remove(alias);
        Ok(json!({}))
    }

    fn list_aliases(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let aliases: Vec<_> = s
            .aliases
            .iter()
            .map(|(name, target)| {
                json!({
                    "AliasName": name,
                    "TargetKeyId": target,
                    "AliasArn": format!("arn:aws:kms:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{name}"),
                })
            })
            .collect();
        Ok(json!({ "Aliases": aliases }))
    }

    fn encrypt(&self, req: &Value) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let plaintext_b64 = req
            .get("Plaintext")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Plaintext required"))?;
        let plaintext = BASE64
            .decode(plaintext_b64.as_bytes())
            .map_err(|_| AwsError::new("ValidationException", "Plaintext is not valid base64"))?;
        let s = self.state.read();
        let key = resolve_key(&s, &id)?;
        if !key.enabled {
            return Err(AwsError::new("DisabledException", "key is disabled"));
        }
        let mat = BASE64
            .decode(key.material_b64.as_bytes())
            .map_err(|_| AwsError::internal("corrupt key material"))?;
        let cipher = xor_with_envelope(&key.key_id, &mat, &plaintext);
        Ok(json!({
            "KeyId": key.arn,
            "CiphertextBlob": BASE64.encode(&cipher),
        }))
    }

    fn decrypt(&self, req: &Value) -> Result<Value, AwsError> {
        let ciphertext_b64 = req
            .get("CiphertextBlob")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "CiphertextBlob required"))?;
        let cipher = BASE64
            .decode(ciphertext_b64.as_bytes())
            .map_err(|_| AwsError::new("ValidationException", "CiphertextBlob not base64"))?;
        let (key_id, payload) = unwrap_envelope(&cipher).ok_or_else(|| {
            AwsError::new("InvalidCiphertextException", "ciphertext not from kuroko")
        })?;
        let s = self.state.read();
        let key = resolve_key(&s, &key_id)?;
        let mat = BASE64
            .decode(key.material_b64.as_bytes())
            .map_err(|_| AwsError::internal("corrupt key material"))?;
        let plaintext = xor_bytes(&payload, &mat);
        Ok(json!({
            "KeyId": key.arn,
            "Plaintext": BASE64.encode(&plaintext),
        }))
    }

    fn generate_data_key(&self, req: &Value) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let bytes = match req.get("KeySpec").and_then(Value::as_str) {
            Some("AES_128") => 16,
            Some("AES_256") | None => 32,
            _ => req
                .get("NumberOfBytes")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(32),
        };
        let plaintext = random_bytes(bytes);
        let s = self.state.read();
        let key = resolve_key(&s, &id)?;
        let mat = BASE64
            .decode(key.material_b64.as_bytes())
            .map_err(|_| AwsError::internal("corrupt key material"))?;
        let cipher = xor_with_envelope(&key.key_id, &mat, &plaintext);
        Ok(json!({
            "KeyId": key.arn,
            "Plaintext": BASE64.encode(&plaintext),
            "CiphertextBlob": BASE64.encode(&cipher),
        }))
    }

    fn set_enabled(&self, req: &Value, enabled: bool) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let mut s = self.state.write();
        let resolved = resolve_key_id(&s, &id)?;
        let k = s.keys.get_mut(&resolved).ok_or_else(|| not_found(&id))?;
        k.enabled = enabled;
        Ok(json!({}))
    }

    fn schedule_key_deletion(&self, req: &Value) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let days = req
            .get("PendingWindowInDays")
            .and_then(Value::as_i64)
            .unwrap_or(30);
        let when = chrono::Utc::now() + chrono::Duration::days(days);
        let mut s = self.state.write();
        let resolved = resolve_key_id(&s, &id)?;
        let k = s.keys.get_mut(&resolved).ok_or_else(|| not_found(&id))?;
        k.deletion_date = Some(when);
        k.enabled = false;
        Ok(json!({
            "KeyId": k.arn,
            "DeletionDate": when.timestamp(),
        }))
    }

    fn cancel_key_deletion(&self, req: &Value) -> Result<Value, AwsError> {
        let id = key_id_from_req(req)?;
        let mut s = self.state.write();
        let resolved = resolve_key_id(&s, &id)?;
        let k = s.keys.get_mut(&resolved).ok_or_else(|| not_found(&id))?;
        k.deletion_date = None;
        Ok(json!({ "KeyId": k.arn }))
    }
}

fn key_id_from_req(req: &Value) -> Result<String, AwsError> {
    req.get("KeyId")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", "KeyId required"))
}

/// Accept a raw UUID, a full ARN, or `alias/<name>` — return the underlying
/// canonical key id.
fn resolve_key_id(s: &State, id: &str) -> Result<String, AwsError> {
    if let Some(target) = s.aliases.get(id) {
        return Ok(target.clone());
    }
    if let Some((_, after)) = id.rsplit_once('/')
        && s.keys.contains_key(after)
    {
        return Ok(after.to_string());
    }
    if s.keys.contains_key(id) {
        Ok(id.to_string())
    } else {
        Err(not_found(id))
    }
}

fn resolve_key<'a>(s: &'a State, id: &str) -> Result<&'a KmsKey, AwsError> {
    let canonical = resolve_key_id(s, id)?;
    s.keys.get(&canonical).ok_or_else(|| not_found(id))
}

fn not_found(id: &str) -> AwsError {
    AwsError::new("NotFoundException", format!("Key '{id}' does not exist"))
}

fn key_metadata(k: &KmsKey) -> Value {
    json!({
        "KeyId": k.key_id,
        "Arn": k.arn,
        "Description": k.description,
        "Enabled": k.enabled,
        "CreationDate": k.created.timestamp(),
        "KeyState": if k.deletion_date.is_some() {
            "PendingDeletion"
        } else if k.enabled {
            "Enabled"
        } else {
            "Disabled"
        },
        "KeyUsage": "ENCRYPT_DECRYPT",
        "KeySpec": "SYMMETRIC_DEFAULT",
        "Origin": "AWS_KMS",
    })
}

fn random_bytes(n: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

fn xor_bytes(payload: &[u8], material: &[u8]) -> Vec<u8> {
    payload
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ material[i % material.len()])
        .collect()
}

/// Wrap ciphertext in a self-describing envelope so Decrypt can locate the
/// key without the caller passing `KeyId` back.
fn xor_with_envelope(key_id: &str, material: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let ct = xor_bytes(plaintext, material);
    let mut env = Vec::with_capacity(8 + key_id.len() + 1 + ct.len());
    env.extend_from_slice(b"kuroko1:");
    env.extend_from_slice(key_id.as_bytes());
    env.push(b':');
    env.extend_from_slice(&ct);
    env
}

fn unwrap_envelope(blob: &[u8]) -> Option<(String, Vec<u8>)> {
    let prefix: &[u8] = b"kuroko1:";
    let body = blob.strip_prefix(prefix)?;
    let colon = body.iter().position(|b| *b == b':')?;
    let key_id = std::str::from_utf8(&body[..colon]).ok()?.to_string();
    let payload = body[colon + 1..].to_vec();
    Some((key_id, payload))
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Kms::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn xor_envelope_roundtrip() {
        let mat = b"0123456789abcdef0123456789abcdef";
        let cipher = xor_with_envelope("kid-1", mat, b"hello kuroko");
        let (kid, payload) = unwrap_envelope(&cipher).unwrap();
        assert_eq!(kid, "kid-1");
        let plain = xor_bytes(&payload, mat);
        assert_eq!(plain, b"hello kuroko");
    }
}
