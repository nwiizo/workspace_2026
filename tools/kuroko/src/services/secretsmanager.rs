//! Secrets Manager — AWS JSON 1.1 protocol.
//!
//! `X-Amz-Target` prefix is `secretsmanager`. Operations cover the standard
//! lifecycle: CreateSecret, GetSecretValue, PutSecretValue, UpdateSecret,
//! DescribeSecret, ListSecrets, DeleteSecret, RestoreSecret. Version IDs and
//! the AWSCURRENT / AWSPREVIOUS staging labels are honored.

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

const TARGET_PREFIX: &str = "secretsmanager";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    secrets: HashMap<String, Secret>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Secret {
    name: String,
    arn: String,
    description: String,
    versions: HashMap<String, SecretVersion>,
    /// version_id → list of staging labels.
    stages: HashMap<String, Vec<String>>,
    deleted: Option<chrono::DateTime<chrono::Utc>>,
    created: chrono::DateTime<chrono::Utc>,
    last_changed: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SecretVersion {
    version_id: String,
    secret_string: Option<String>,
    secret_binary_b64: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
}

pub struct SecretsManager {
    state: Arc<RwLock<State>>,
}

impl SecretsManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for SecretsManager {
    fn name(&self) -> &'static str {
        "secretsmanager"
    }

    fn reset(&self) {
        self.state.write().secrets.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State>("secretsmanager")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("secretsmanager", &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for SecretsManager {
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
            "CreateSecret" => self.create_secret(&req),
            "GetSecretValue" => self.get_secret_value(&req),
            "PutSecretValue" => self.put_secret_value(&req),
            "UpdateSecret" => self.update_secret(&req),
            "DescribeSecret" => self.describe_secret(&req),
            "ListSecrets" => self.list_secrets(&req),
            "DeleteSecret" => self.delete_secret(&req),
            "RestoreSecret" => self.restore_secret(&req),
            "ListSecretVersionIds" => self.list_secret_version_ids(&req),
            other => Err(AwsError::unsupported(format!("SecretsManager::{other}"))),
        }
    }
}

impl SecretsManager {
    fn create_secret(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("Name")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "Name required"))?
            .to_string();
        let description = req
            .get("Description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut s = self.state.write();
        if s.secrets.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceExistsException",
                format!("secret '{name}' already exists"),
            ));
        }
        let arn = secret_arn(&name);
        let version = build_version(req);
        let mut versions = HashMap::new();
        let mut stages = HashMap::new();
        stages.insert(version.version_id.clone(), vec!["AWSCURRENT".to_string()]);
        let now = chrono::Utc::now();
        let version_id = version.version_id.clone();
        versions.insert(version_id.clone(), version);
        let secret = Secret {
            name: name.clone(),
            arn: arn.clone(),
            description,
            versions,
            stages,
            deleted: None,
            created: now,
            last_changed: now,
        };
        s.secrets.insert(name.clone(), secret);
        Ok(json!({ "Name": name, "ARN": arn, "VersionId": version_id }))
    }

    fn get_secret_value(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let stage = req
            .get("VersionStage")
            .and_then(Value::as_str)
            .unwrap_or("AWSCURRENT")
            .to_string();
        let requested_version = req
            .get("VersionId")
            .and_then(Value::as_str)
            .map(String::from);
        let s = self.state.read();
        let secret = s.secrets.get(&id).ok_or_else(|| not_found(&id))?;
        let version_id = if let Some(v) = requested_version {
            v
        } else {
            secret
                .stages
                .iter()
                .find(|(_, labels)| labels.iter().any(|l| l == &stage))
                .map(|(v, _)| v.clone())
                .ok_or_else(|| {
                    AwsError::new(
                        "ResourceNotFoundException",
                        format!("no secret value for stage {stage}"),
                    )
                })?
        };
        let v = secret.versions.get(&version_id).ok_or_else(|| {
            AwsError::new(
                "ResourceNotFoundException",
                format!("version {version_id} not found"),
            )
        })?;
        let mut out = json!({
            "Name": secret.name,
            "ARN": secret.arn,
            "VersionId": v.version_id,
            "CreatedDate": v.created.timestamp(),
            "VersionStages": secret.stages.get(&v.version_id).cloned().unwrap_or_default(),
        });
        if let Some(string) = &v.secret_string {
            out["SecretString"] = json!(string);
        }
        if let Some(b) = &v.secret_binary_b64 {
            out["SecretBinary"] = json!(b);
        }
        Ok(out)
    }

    fn put_secret_value(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let mut s = self.state.write();
        let secret = s.secrets.get_mut(&id).ok_or_else(|| not_found(&id))?;
        let version = build_version(req);
        // The new version becomes AWSCURRENT; the previous current (if any)
        // moves to AWSPREVIOUS, matching AWS semantics.
        for stages in secret.stages.values_mut() {
            stages.retain(|l| l != "AWSPREVIOUS");
        }
        let previous_current: Option<String> = secret
            .stages
            .iter()
            .find(|(_, labels)| labels.iter().any(|l| l == "AWSCURRENT"))
            .map(|(v, _)| v.clone());
        if let Some(prev) = previous_current
            && let Some(stages) = secret.stages.get_mut(&prev)
        {
            stages.retain(|l| l != "AWSCURRENT");
            stages.push("AWSPREVIOUS".to_string());
        }
        let new_id = version.version_id.clone();
        secret
            .stages
            .insert(new_id.clone(), vec!["AWSCURRENT".to_string()]);
        secret.versions.insert(new_id.clone(), version);
        secret.last_changed = chrono::Utc::now();
        Ok(json!({
            "Name": secret.name,
            "ARN": secret.arn,
            "VersionId": new_id,
        }))
    }

    fn update_secret(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        {
            let mut s = self.state.write();
            let secret = s.secrets.get_mut(&id).ok_or_else(|| not_found(&id))?;
            if let Some(d) = req.get("Description").and_then(Value::as_str) {
                secret.description = d.to_string();
                secret.last_changed = chrono::Utc::now();
            }
        }
        if req.get("SecretString").is_some() || req.get("SecretBinary").is_some() {
            self.put_secret_value(req)
        } else {
            let s = self.state.read();
            let secret = s.secrets.get(&id).ok_or_else(|| not_found(&id))?;
            Ok(json!({ "Name": secret.name, "ARN": secret.arn }))
        }
    }

    fn describe_secret(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let s = self.state.read();
        let secret = s.secrets.get(&id).ok_or_else(|| not_found(&id))?;
        let mut version_to_stages = serde_json::Map::new();
        for (v, labels) in &secret.stages {
            version_to_stages.insert(v.clone(), json!(labels));
        }
        Ok(json!({
            "Name": secret.name,
            "ARN": secret.arn,
            "Description": secret.description,
            "VersionIdsToStages": Value::Object(version_to_stages),
            "CreatedDate": secret.created.timestamp(),
            "LastChangedDate": secret.last_changed.timestamp(),
            "DeletedDate": secret.deleted.map(|d| d.timestamp()),
        }))
    }

    fn list_secrets(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let list: Vec<_> = s
            .secrets
            .values()
            .map(|secret| {
                json!({
                    "Name": secret.name,
                    "ARN": secret.arn,
                    "Description": secret.description,
                    "DeletedDate": secret.deleted.map(|d| d.timestamp()),
                    "CreatedDate": secret.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "SecretList": list }))
    }

    fn delete_secret(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let force = req
            .get("ForceDeleteWithoutRecovery")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut s = self.state.write();
        if force {
            let secret = s.secrets.remove(&id).ok_or_else(|| not_found(&id))?;
            return Ok(json!({
                "Name": secret.name,
                "ARN": secret.arn,
                "DeletionDate": chrono::Utc::now().timestamp(),
            }));
        }
        let secret = s.secrets.get_mut(&id).ok_or_else(|| not_found(&id))?;
        secret.deleted = Some(chrono::Utc::now());
        Ok(json!({
            "Name": secret.name,
            "ARN": secret.arn,
            "DeletionDate": secret.deleted.map(|d| d.timestamp()),
        }))
    }

    fn restore_secret(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let mut s = self.state.write();
        let secret = s.secrets.get_mut(&id).ok_or_else(|| not_found(&id))?;
        secret.deleted = None;
        Ok(json!({ "Name": secret.name, "ARN": secret.arn }))
    }

    fn list_secret_version_ids(&self, req: &Value) -> Result<Value, AwsError> {
        let id = secret_id(req)?;
        let s = self.state.read();
        let secret = s.secrets.get(&id).ok_or_else(|| not_found(&id))?;
        let versions: Vec<_> = secret
            .versions
            .values()
            .map(|v| {
                json!({
                    "VersionId": v.version_id,
                    "CreatedDate": v.created.timestamp(),
                    "VersionStages": secret
                        .stages
                        .get(&v.version_id)
                        .cloned()
                        .unwrap_or_default(),
                })
            })
            .collect();
        Ok(json!({
            "Name": secret.name,
            "ARN": secret.arn,
            "Versions": versions,
        }))
    }
}

fn secret_id(req: &Value) -> Result<String, AwsError> {
    let raw = req
        .get("SecretId")
        .and_then(Value::as_str)
        .ok_or_else(|| AwsError::new("ValidationException", "SecretId required"))?;
    // ARNs look like `arn:aws:secretsmanager:<region>:<acct>:secret:<name>-<6>`.
    // Pull the name back out for our by-name storage.
    if let Some(idx) = raw.find(":secret:") {
        let tail = &raw[idx + ":secret:".len()..];
        return Ok(strip_random_suffix(tail));
    }
    Ok(strip_random_suffix(raw))
}

fn strip_random_suffix(s: &str) -> String {
    if let Some(dash) = s.rfind('-')
        && s[dash + 1..].len() == 6
        && s[dash + 1..].bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return s[..dash].to_string();
    }
    s.to_string()
}

fn not_found(id: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("Secrets Manager can't find the specified secret '{id}'"),
    )
}

fn secret_arn(name: &str) -> String {
    use rand::Rng;
    let suffix: String = (0..6)
        .map(|_| {
            let c = rand::thread_rng().gen_range(0u8..36);
            if c < 10 {
                (b'0' + c) as char
            } else {
                (b'a' + (c - 10)) as char
            }
        })
        .collect();
    format!("arn:aws:secretsmanager:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:secret:{name}-{suffix}")
}

fn build_version(req: &Value) -> SecretVersion {
    SecretVersion {
        version_id: req
            .get("ClientRequestToken")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        secret_string: req
            .get("SecretString")
            .and_then(Value::as_str)
            .map(String::from),
        secret_binary_b64: req
            .get("SecretBinary")
            .and_then(Value::as_str)
            .map(String::from),
        created: chrono::Utc::now(),
    }
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(SecretsManager::new()));
}
