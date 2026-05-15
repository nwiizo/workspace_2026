//! ECR — AWS JSON 1.1, target prefix `AmazonEC2ContainerRegistry_V20150921`.
//!
//! Repository metadata and image tagging only — no actual blob hosting.
//! PutImage records a (digest, manifest, tags...) tuple in memory; BatchGetImage
//! retrieves manifests by tag or digest.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "AmazonEC2ContainerRegistry_V20150921";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    repositories: HashMap<String, Repository>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Repository {
    name: String,
    arn: String,
    registry_id: String,
    repository_uri: String,
    created: chrono::DateTime<chrono::Utc>,
    images: Vec<Image>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Image {
    digest: String,
    manifest: String,
    tags: HashSet<String>,
    pushed: chrono::DateTime<chrono::Utc>,
}

pub struct Ecr {
    state: Arc<RwLock<State>>,
}

impl Ecr {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Ecr {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Ecr {
    fn name(&self) -> &'static str {
        "ecr"
    }

    fn reset(&self) {
        self.state.write().repositories.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("ecr").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("ecr", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Ecr {
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
            "CreateRepository" => self.create_repository(&req),
            "DescribeRepositories" => self.describe_repositories(&req),
            "DeleteRepository" => self.delete_repository(&req),
            "ListImages" => self.list_images(&req),
            "DescribeImages" => self.describe_images(&req),
            "BatchGetImage" => self.batch_get_image(&req),
            "PutImage" => self.put_image(&req),
            "BatchDeleteImage" => self.batch_delete_image(&req),
            "GetAuthorizationToken" => self.get_authorization_token(),
            other => Err(AwsError::unsupported(format!("ECR::{other}"))),
        }
    }
}

impl Ecr {
    fn create_repository(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?
            .to_string();
        let mut s = self.state.write();
        if s.repositories.contains_key(&name) {
            return Err(AwsError::new(
                "RepositoryAlreadyExistsException",
                format!("repository '{name}' already exists"),
            ));
        }
        let repo = build_repository(name.clone());
        let value = repository_json(&repo);
        s.repositories.insert(name, repo);
        Ok(json!({ "repository": value }))
    }

    fn describe_repositories(&self, req: &Value) -> Result<Value, AwsError> {
        let names: Option<Vec<String>> =
            req.get("repositoryNames")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                });
        let s = self.state.read();
        let repos: Vec<_> = s
            .repositories
            .values()
            .filter(|r| names.as_ref().is_none_or(|ns| ns.contains(&r.name)))
            .map(repository_json)
            .collect();
        Ok(json!({ "repositories": repos }))
    }

    fn delete_repository(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let force = req.get("force").and_then(Value::as_bool).unwrap_or(false);
        let mut s = self.state.write();
        let repo = s.repositories.get(name).ok_or_else(|| not_found(name))?;
        if !force && !repo.images.is_empty() {
            return Err(AwsError::new(
                "RepositoryNotEmptyException",
                "repository contains images; pass force=true to delete",
            ));
        }
        let removed = s.repositories.remove(name).unwrap();
        Ok(json!({ "repository": repository_json(&removed) }))
    }

    fn put_image(&self, req: &Value) -> Result<Value, AwsError> {
        let repo_name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let manifest = req
            .get("imageManifest")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "imageManifest required"))?
            .to_string();
        let tag = req
            .get("imageTag")
            .and_then(Value::as_str)
            .map(String::from);
        let digest = digest_of(&manifest);
        let mut s = self.state.write();
        let repo = s
            .repositories
            .get_mut(repo_name)
            .ok_or_else(|| not_found(repo_name))?;

        // Tag uniqueness: if the same tag exists on a different digest, move
        // it to the new image (matches AWS's PutImage tag-repointing behavior).
        if let Some(ref t) = tag {
            for img in repo.images.iter_mut() {
                img.tags.remove(t);
            }
        }

        let entry = repo.images.iter_mut().find(|i| i.digest == digest);
        let image = if let Some(existing) = entry {
            if let Some(t) = tag.clone() {
                existing.tags.insert(t);
            }
            existing.clone()
        } else {
            let mut tags = HashSet::new();
            if let Some(t) = tag.clone() {
                tags.insert(t);
            }
            let img = Image {
                digest: digest.clone(),
                manifest: manifest.clone(),
                tags,
                pushed: chrono::Utc::now(),
            };
            repo.images.push(img.clone());
            img
        };
        Ok(json!({ "image": image_json(repo_name, &image) }))
    }

    fn list_images(&self, req: &Value) -> Result<Value, AwsError> {
        let repo_name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let s = self.state.read();
        let repo = s
            .repositories
            .get(repo_name)
            .ok_or_else(|| not_found(repo_name))?;
        let mut ids = Vec::new();
        for img in &repo.images {
            if img.tags.is_empty() {
                ids.push(json!({ "imageDigest": img.digest }));
            } else {
                for t in &img.tags {
                    ids.push(json!({ "imageDigest": img.digest, "imageTag": t }));
                }
            }
        }
        Ok(json!({ "imageIds": ids }))
    }

    fn describe_images(&self, req: &Value) -> Result<Value, AwsError> {
        let repo_name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let s = self.state.read();
        let repo = s
            .repositories
            .get(repo_name)
            .ok_or_else(|| not_found(repo_name))?;
        let details: Vec<_> = repo
            .images
            .iter()
            .map(|i| {
                json!({
                    "registryId": EMULATED_ACCOUNT_ID,
                    "repositoryName": repo_name,
                    "imageDigest": i.digest,
                    "imageTags": i.tags.iter().cloned().collect::<Vec<_>>(),
                    "imageSizeInBytes": i.manifest.len() as i64,
                    "imagePushedAt": i.pushed.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "imageDetails": details }))
    }

    fn batch_get_image(&self, req: &Value) -> Result<Value, AwsError> {
        let repo_name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let ids = req
            .get("imageIds")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let s = self.state.read();
        let repo = s
            .repositories
            .get(repo_name)
            .ok_or_else(|| not_found(repo_name))?;
        let mut found = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            let digest = id.get("imageDigest").and_then(Value::as_str);
            let tag = id.get("imageTag").and_then(Value::as_str);
            let img = repo.images.iter().find(|i| match (digest, tag) {
                (Some(d), _) => i.digest == d,
                (None, Some(t)) => i.tags.contains(t),
                _ => false,
            });
            match img {
                Some(i) => found.push(image_json(repo_name, i)),
                None => failed.push(json!({
                    "imageId": id,
                    "failureCode": "ImageNotFound",
                    "failureReason": "Requested image not found",
                })),
            }
        }
        Ok(json!({ "images": found, "failures": failed }))
    }

    fn batch_delete_image(&self, req: &Value) -> Result<Value, AwsError> {
        let repo_name = req
            .get("repositoryName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("InvalidParameterException", "repositoryName required"))?;
        let ids = req
            .get("imageIds")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut s = self.state.write();
        let repo = s
            .repositories
            .get_mut(repo_name)
            .ok_or_else(|| not_found(repo_name))?;
        let mut deleted = Vec::new();
        for id in ids {
            let digest = id
                .get("imageDigest")
                .and_then(Value::as_str)
                .map(String::from);
            let tag = id.get("imageTag").and_then(Value::as_str).map(String::from);
            if let Some(d) = &digest {
                let before = repo.images.len();
                repo.images.retain(|i| &i.digest != d);
                if repo.images.len() < before {
                    deleted.push(json!({ "imageDigest": d }));
                }
            } else if let Some(t) = &tag {
                for img in repo.images.iter_mut() {
                    if img.tags.remove(t) {
                        deleted.push(json!({ "imageTag": t, "imageDigest": img.digest }));
                    }
                }
            }
        }
        Ok(json!({ "imageIds": deleted, "failures": [] }))
    }

    fn get_authorization_token(&self) -> Result<Value, AwsError> {
        let token = BASE64.encode("AWS:kuroko");
        Ok(json!({
            "authorizationData": [{
                "authorizationToken": token,
                "expiresAt": (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp(),
                "proxyEndpoint": format!(
                    "https://{EMULATED_ACCOUNT_ID}.dkr.ecr.{EMULATED_REGION}.amazonaws.com"
                ),
            }]
        }))
    }
}

fn build_repository(name: String) -> Repository {
    Repository {
        arn: format!("arn:aws:ecr:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:repository/{name}"),
        registry_id: EMULATED_ACCOUNT_ID.to_string(),
        repository_uri: format!(
            "{EMULATED_ACCOUNT_ID}.dkr.ecr.{EMULATED_REGION}.amazonaws.com/{name}"
        ),
        name,
        created: chrono::Utc::now(),
        images: Vec::new(),
    }
}

fn repository_json(r: &Repository) -> Value {
    json!({
        "repositoryArn": r.arn,
        "registryId": r.registry_id,
        "repositoryName": r.name,
        "repositoryUri": r.repository_uri,
        "createdAt": r.created.timestamp(),
    })
}

fn image_json(repo_name: &str, i: &Image) -> Value {
    let tag = i.tags.iter().next().cloned();
    let mut image_id = json!({ "imageDigest": i.digest });
    if let Some(t) = tag {
        image_id["imageTag"] = Value::String(t);
    }
    json!({
        "registryId": EMULATED_ACCOUNT_ID,
        "repositoryName": repo_name,
        "imageId": image_id,
        "imageManifest": i.manifest,
    })
}

fn digest_of(manifest: &str) -> String {
    let mut h = Sha256::new();
    h.update(manifest.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "RepositoryNotFoundException",
        format!("repository '{name}' not found"),
    )
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Ecr::new()));
}
