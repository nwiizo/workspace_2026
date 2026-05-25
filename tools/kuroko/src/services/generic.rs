//! Generic minimal-resource implementation helpers.
//!
//! Many AWS services follow a near-identical Create/Get/List/Delete shape over
//! a single named resource. Rather than copy-paste 200-line modules for each
//! of the 30+ remaining stubs, this module exposes:
//!
//! - `GenericJsonResource`: a `JsonProtocolService` for services whose action
//!   names map cleanly to `Create<Resource>` / `Describe<Resource>` etc.
//! - `GenericQueryResource`: same idea for AWS Query (XML) services.
//!
//! Each service module declares its `target_prefix` (or `sdk_id`+actions),
//! the name of its primary resource, and the AWS-spec field names used in
//! request/response. The helper handles the wire shapes.
//!
//! These helpers are intentionally simple — they're not meant to cover the
//! full surface of any one service. They give the SDK a correctly-shaped
//! response for the lifecycle operations IaC tools most care about. Services
//! with more sophisticated needs (S3, SQS, DynamoDB, etc.) implement
//! `JsonProtocolService` / `QueryProtocolService` by hand.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::{AwsError, xml_escape};
use crate::registry::Registry;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, QueryProtocolService, Service,
    ServiceContext, persistence_error,
};

/// Per-service configuration baked into the generic JSON service. The string
/// fields are AWS-spec names that vary across services (e.g. SageMaker uses
/// `NotebookInstanceName`, Cognito uses `PoolName`, etc.).
pub struct JsonResourceConfig {
    /// Service name as registered (e.g. "sagemaker").
    pub service_name: &'static str,
    /// `X-Amz-Target` prefix (e.g. "SageMaker").
    pub target_prefix: &'static str,
    /// Resource label used in AWS messages (e.g. "notebook instance").
    pub resource_label: &'static str,
    /// Request field that carries the resource name on Create.
    pub create_name_field: &'static str,
    /// Request field that carries the resource name on Describe/Delete.
    pub identify_field: &'static str,
    /// Action prefixes the service speaks. The helper handles any action that
    /// matches `Create<Suffix>`, `Describe<Suffix>`, `List<Suffix>` (plural),
    /// or `Delete<Suffix>` where `Suffix` is one of these.
    pub action_suffixes: &'static [&'static str],
    /// ARN service component (e.g. "sagemaker"). Builds
    /// `arn:aws:<arn_service>:<region>:<account>:<resource_label>/<name>`.
    pub arn_service: &'static str,
    /// Field name used to wrap the resource in the Describe response.
    pub describe_wrap_field: &'static str,
    /// Field name used to wrap the list in the List response.
    pub list_wrap_field: &'static str,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Resource {
    name: String,
    arn: String,
    created: chrono::DateTime<chrono::Utc>,
    /// All other fields from the create request, stored verbatim. We echo
    /// them back on describe so the SDK can read its own writes.
    extras: HashMap<String, Value>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct ResourceState {
    items: HashMap<String, Resource>,
}

pub struct GenericJsonResource {
    config: JsonResourceConfig,
    state: Arc<RwLock<ResourceState>>,
}

impl GenericJsonResource {
    pub fn new(config: JsonResourceConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ResourceState::default())),
        }
    }

    fn snapshot_key(&self) -> &'static str {
        self.config.service_name
    }

    fn arn_for(&self, name: &str) -> String {
        format!(
            "arn:aws:{svc}:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{label}/{name}",
            svc = self.config.arn_service,
            label = self.config.resource_label
        )
    }
}

#[async_trait]
impl Service for GenericJsonResource {
    fn name(&self) -> &'static str {
        self.config.service_name
    }
    fn reset(&self) {
        self.state.write().items.clear();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<ResourceState>(self.snapshot_key())
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save(self.snapshot_key(), &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for GenericJsonResource {
    fn target_prefix(&self) -> &'static str {
        self.config.target_prefix
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
                .map_err(|e| AwsError::new("InvalidParameterException", e.to_string()))?
        };
        for suffix in self.config.action_suffixes {
            if let Some(verb) = action.strip_suffix(suffix) {
                return match verb {
                    "Create" | "Register" => self.create(&req),
                    "Describe" | "Get" => self.describe(&req),
                    "Delete" | "Deregister" => self.delete(&req),
                    _ => continue,
                };
            }
            // List<Suffix> often becomes List<SuffixPlural>; tolerate both.
            let plural = format!("{suffix}s");
            if let Some(verb) = action
                .strip_suffix(&plural)
                .or_else(|| action.strip_suffix(suffix))
                && verb == "List"
            {
                return self.list();
            }
        }
        Err(AwsError::unsupported(format!(
            "{}::{action}",
            self.config.service_name
        )))
    }
}

impl GenericJsonResource {
    fn create(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get(self.config.create_name_field)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationException",
                    format!("{} required", self.config.create_name_field),
                )
            })?
            .to_string();
        let arn = self.arn_for(&name);
        let mut extras = HashMap::new();
        if let Some(obj) = req.as_object() {
            for (k, v) in obj {
                if k != self.config.create_name_field {
                    extras.insert(k.clone(), v.clone());
                }
            }
        }
        let mut s = self.state.write();
        if s.items.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceInUseException",
                format!(
                    "{label} '{name}' already exists",
                    label = self.config.resource_label
                ),
            ));
        }
        s.items.insert(
            name.clone(),
            Resource {
                name: name.clone(),
                arn: arn.clone(),
                created: chrono::Utc::now(),
                extras,
            },
        );
        Ok(json!({
            self.config.create_name_field: name,
            "Arn": arn,
            "CreationTime": chrono::Utc::now().timestamp(),
        }))
    }

    fn describe(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get(self.config.identify_field)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationException",
                    format!("{} required", self.config.identify_field),
                )
            })?
            .to_string();
        let s = self.state.read();
        let item = s.items.get(&name).ok_or_else(|| {
            AwsError::new(
                "ResourceNotFoundException",
                format!(
                    "{label} '{name}' not found",
                    label = self.config.resource_label
                ),
            )
        })?;
        let mut inner = json!({
            self.config.create_name_field: item.name,
            "Arn": item.arn,
            "CreationTime": item.created.timestamp(),
        });
        if let Some(obj) = inner.as_object_mut() {
            for (k, v) in &item.extras {
                obj.insert(k.clone(), v.clone());
            }
        }
        Ok(json!({ self.config.describe_wrap_field: inner }))
    }

    fn list(&self) -> Result<Value, AwsError> {
        let s = self.state.read();
        let summaries: Vec<Value> = s
            .items
            .values()
            .map(|r| {
                json!({
                    self.config.create_name_field: r.name,
                    "Arn": r.arn,
                    "CreationTime": r.created.timestamp(),
                })
            })
            .collect();
        Ok(json!({ self.config.list_wrap_field: summaries }))
    }

    fn delete(&self, req: &Value) -> Result<Value, AwsError> {
        let name = req
            .get(self.config.identify_field)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationException",
                    format!("{} required", self.config.identify_field),
                )
            })?
            .to_string();
        self.state.write().items.remove(&name).ok_or_else(|| {
            AwsError::new(
                "ResourceNotFoundException",
                format!(
                    "{label} '{name}' not found",
                    label = self.config.resource_label
                ),
            )
        })?;
        Ok(json!({}))
    }
}

pub fn register_json(registry: &Arc<Registry>, config: JsonResourceConfig) {
    registry.register_json(Arc::new(GenericJsonResource::new(config)));
}

// === Query-protocol variant ===

pub struct QueryResourceConfig {
    pub service_name: &'static str,
    pub sdk_id: &'static str,
    pub actions: &'static [&'static str],
    /// XML namespace for response envelopes.
    pub namespace: &'static str,
    /// Field name on Create that carries the resource name.
    pub create_name_field: &'static str,
    /// Field name on identify (Describe/Delete) that carries the name.
    pub identify_field: &'static str,
    /// ARN service component.
    pub arn_service: &'static str,
    /// Resource label used in ARN and error messages.
    pub resource_label: &'static str,
}

pub struct GenericQueryResource {
    config: QueryResourceConfig,
    state: Arc<RwLock<ResourceState>>,
}

impl GenericQueryResource {
    pub fn new(config: QueryResourceConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ResourceState::default())),
        }
    }

    fn arn_for(&self, name: &str) -> String {
        format!(
            "arn:aws:{svc}:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{label}/{name}",
            svc = self.config.arn_service,
            label = self.config.resource_label
        )
    }
}

#[async_trait]
impl Service for GenericQueryResource {
    fn name(&self) -> &'static str {
        self.config.service_name
    }
    fn reset(&self) {
        self.state.write().items.clear();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<ResourceState>(self.config.service_name)
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save(self.config.service_name, &*data)
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl QueryProtocolService for GenericQueryResource {
    fn sdk_id(&self) -> &'static str {
        self.config.sdk_id
    }
    fn actions(&self) -> &'static [&'static str] {
        self.config.actions
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        params: &HashMap<String, String>,
    ) -> Result<String, AwsError> {
        // Identify by verb prefix; this is a 1-resource service, so suffix
        // doesn't matter for routing.
        if action.starts_with("Create") {
            return self.create(action, params);
        }
        if action.starts_with("Describe") || action.starts_with("Get") {
            return self.describe(action, params);
        }
        if action.starts_with("Delete") {
            return self.delete(action, params);
        }
        if action.starts_with("List") {
            return self.list(action);
        }
        Err(AwsError::unsupported(format!(
            "{}::{action}",
            self.config.service_name
        )))
    }
}

impl GenericQueryResource {
    fn create(&self, action: &str, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = params
            .get(self.config.create_name_field)
            .cloned()
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationError",
                    format!("{} required", self.config.create_name_field),
                )
            })?;
        let mut s = self.state.write();
        if s.items.contains_key(&name) {
            return Err(AwsError::new(
                "ResourceInUseFault",
                format!(
                    "{label} '{name}' already exists",
                    label = self.config.resource_label
                ),
            ));
        }
        let arn = self.arn_for(&name);
        s.items.insert(
            name.clone(),
            Resource {
                name: name.clone(),
                arn: arn.clone(),
                created: chrono::Utc::now(),
                extras: HashMap::new(),
            },
        );
        Ok(self.wrap_response(
            action,
            &format!(
                "<{label}><{name_field}>{name}</{name_field}><Arn>{arn}</Arn></{label}>",
                label = pascal(self.config.resource_label),
                name_field = self.config.create_name_field,
                name = xml_escape(&name),
                arn = xml_escape(&arn),
            ),
        ))
    }

    fn describe(&self, action: &str, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = params
            .get(self.config.identify_field)
            .cloned()
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationError",
                    format!("{} required", self.config.identify_field),
                )
            })?;
        let s = self.state.read();
        let item = s.items.get(&name).ok_or_else(|| {
            AwsError::new(
                "ResourceNotFoundFault",
                format!(
                    "{label} '{name}' not found",
                    label = self.config.resource_label
                ),
            )
        })?;
        Ok(self.wrap_response(
            action,
            &format!(
                "<{label}><{name_field}>{name}</{name_field}><Arn>{arn}</Arn></{label}>",
                label = pascal(self.config.resource_label),
                name_field = self.config.create_name_field,
                name = xml_escape(&item.name),
                arn = xml_escape(&item.arn),
            ),
        ))
    }

    fn list(&self, action: &str) -> Result<String, AwsError> {
        let s = self.state.read();
        let mut members = String::new();
        for item in s.items.values() {
            members.push_str(&format!(
                "<member><{name_field}>{name}</{name_field}><Arn>{arn}</Arn></member>",
                name_field = self.config.create_name_field,
                name = xml_escape(&item.name),
                arn = xml_escape(&item.arn),
            ));
        }
        Ok(self.wrap_response(
            action,
            &format!(
                "<{label}s>{members}</{label}s>",
                label = pascal(self.config.resource_label)
            ),
        ))
    }

    fn delete(&self, action: &str, params: &HashMap<String, String>) -> Result<String, AwsError> {
        let name = params
            .get(self.config.identify_field)
            .cloned()
            .ok_or_else(|| {
                AwsError::new(
                    "ValidationError",
                    format!("{} required", self.config.identify_field),
                )
            })?;
        self.state.write().items.remove(&name).ok_or_else(|| {
            AwsError::new(
                "ResourceNotFoundFault",
                format!(
                    "{label} '{name}' not found",
                    label = self.config.resource_label
                ),
            )
        })?;
        Ok(self.empty_response(action))
    }

    fn wrap_response(&self, action: &str, body: &str) -> String {
        let rid = Uuid::new_v4();
        let ns = self.config.namespace;
        format!(
            "<{action}Response xmlns=\"{ns}\"><{action}Result>{body}</{action}Result><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
        )
    }

    fn empty_response(&self, action: &str) -> String {
        let rid = Uuid::new_v4();
        let ns = self.config.namespace;
        format!(
            "<{action}Response xmlns=\"{ns}\"><ResponseMetadata><RequestId>{rid}</RequestId></ResponseMetadata></{action}Response>"
        )
    }
}

pub fn register_query(registry: &Arc<Registry>, config: QueryResourceConfig) {
    registry.register_query(Arc::new(GenericQueryResource::new(config)));
}

fn pascal(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}
