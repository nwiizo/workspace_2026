//! API Gateway (v1 / REST APIs) — restJson1 protocol mounted under
//! `/restapis/*`.
//!
//! Implements the resource hierarchy used by typical IaC provisioning:
//! RestApi, Resource (with parent/path), Method (per-resource HTTP verb),
//! Integration (Method → backend wiring), Deployment, Stage. No actual
//! request proxying happens — this is the control-plane API only.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post, put};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{Service, ServiceContext, persistence_error};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    apis: HashMap<String, RestApi>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RestApi {
    id: String,
    name: String,
    description: Option<String>,
    created: chrono::DateTime<chrono::Utc>,
    /// resource_id → Resource
    resources: HashMap<String, Resource>,
    /// deployment_id → ts
    deployments: HashMap<String, chrono::DateTime<chrono::Utc>>,
    /// stage_name → deployment_id
    stages: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Resource {
    id: String,
    parent_id: Option<String>,
    path_part: Option<String>,
    path: String,
    methods: HashMap<String, Method>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Method {
    http_method: String,
    authorization_type: String,
    api_key_required: bool,
    integration: Option<Integration>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Integration {
    type_: String,
    integration_http_method: Option<String>,
    uri: Option<String>,
    request_templates: HashMap<String, String>,
}

pub struct ApiGateway {
    state: Arc<RwLock<State_>>,
}

impl ApiGateway {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}

impl Default for ApiGateway {
    fn default() -> Self {
        Self::new()
    }
}

type AgwState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for ApiGateway {
    fn name(&self) -> &'static str {
        "apigateway"
    }

    fn reset(&self) {
        self.state.write().apis.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap
                .load::<State_>("apigateway")
                .map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("apigateway", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route("/restapis", post(create_rest_api).get(get_rest_apis))
            .route(
                "/restapis/{api_id}",
                get(get_rest_api).delete(delete_rest_api),
            )
            .route("/restapis/{api_id}/resources", get(get_resources))
            .route(
                "/restapis/{api_id}/resources/{resource_id}",
                post(create_resource)
                    .get(get_resource)
                    .delete(delete_resource),
            )
            .route(
                "/restapis/{api_id}/resources/{resource_id}/methods/{http_method}",
                put(put_method).get(get_method).delete(delete_method),
            )
            .route(
                "/restapis/{api_id}/resources/{resource_id}/methods/{http_method}/integration",
                put(put_integration).get(get_integration),
            )
            .route("/restapis/{api_id}/deployments", post(create_deployment))
            .route("/restapis/{api_id}/stages", post(create_stage))
            .route("/restapis/{api_id}/stages/{stage_name}", get(get_stage))
            .with_state(state)
    }
}

async fn create_rest_api(State(state): State<AgwState>, body: Bytes) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let name = match req.get("name").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => return rest_error(AwsError::new("BadRequestException", "name required")),
    };
    let description = req
        .get("description")
        .and_then(Value::as_str)
        .map(String::from);
    let id = short_id();
    let root_id = short_id();
    let root = Resource {
        id: root_id.clone(),
        parent_id: None,
        path_part: None,
        path: "/".into(),
        methods: HashMap::new(),
    };
    let mut resources = HashMap::new();
    resources.insert(root_id, root);
    let api = RestApi {
        id: id.clone(),
        name,
        description,
        created: chrono::Utc::now(),
        resources,
        deployments: HashMap::new(),
        stages: HashMap::new(),
    };
    let body = api_json(&api);
    state.write().apis.insert(id, api);
    rest_json(StatusCode::CREATED, &body)
}

async fn get_rest_apis(State(state): State<AgwState>) -> Response {
    let s = state.read();
    let items: Vec<_> = s.apis.values().map(api_json).collect();
    rest_json(
        StatusCode::OK,
        // AWS API Gateway wire format uses the singular `item` array key.
        &json!({ "item": items, "position": Value::Null }),
    )
}

async fn get_rest_api(State(state): State<AgwState>, Path(api_id): Path<String>) -> Response {
    let s = state.read();
    match s.apis.get(&api_id) {
        Some(a) => rest_json(StatusCode::OK, &api_json(a)),
        None => rest_error(not_found("rest api", &api_id)),
    }
}

async fn delete_rest_api(State(state): State<AgwState>, Path(api_id): Path<String>) -> Response {
    if state.write().apis.remove(&api_id).is_none() {
        return rest_error(not_found("rest api", &api_id));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn get_resources(State(state): State<AgwState>, Path(api_id): Path<String>) -> Response {
    tracing::debug!(api_id = %api_id, "apigateway: get_resources");
    let s = state.read();
    let Some(api) = s.apis.get(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let items: Vec<_> = api.resources.values().map(resource_json).collect();
    rest_json(
        StatusCode::OK,
        // AWS API Gateway wire format uses the singular `item` array key.
        &json!({ "item": items, "position": Value::Null }),
    )
}

async fn create_resource(
    State(state): State<AgwState>,
    Path((api_id, parent_id)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let part = match req.get("pathPart").and_then(Value::as_str) {
        Some(p) => p.to_string(),
        None => return rest_error(AwsError::new("BadRequestException", "pathPart required")),
    };
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let parent_path = match api.resources.get(&parent_id) {
        Some(p) => p.path.clone(),
        None => return rest_error(not_found("resource", &parent_id)),
    };
    let path = if parent_path == "/" {
        format!("/{part}")
    } else {
        format!("{parent_path}/{part}")
    };
    let id = short_id();
    let resource = Resource {
        id: id.clone(),
        parent_id: Some(parent_id),
        path_part: Some(part),
        path,
        methods: HashMap::new(),
    };
    let body = resource_json(&resource);
    api.resources.insert(id, resource);
    rest_json(StatusCode::CREATED, &body)
}

async fn get_resource(
    State(state): State<AgwState>,
    Path((api_id, resource_id)): Path<(String, String)>,
) -> Response {
    let s = state.read();
    let Some(api) = s.apis.get(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    match api.resources.get(&resource_id) {
        Some(r) => rest_json(StatusCode::OK, &resource_json(r)),
        None => rest_error(not_found("resource", &resource_id)),
    }
}

async fn delete_resource(
    State(state): State<AgwState>,
    Path((api_id, resource_id)): Path<(String, String)>,
) -> Response {
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    if api.resources.remove(&resource_id).is_none() {
        return rest_error(not_found("resource", &resource_id));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn put_method(
    State(state): State<AgwState>,
    Path((api_id, resource_id, http_method)): Path<(String, String, String)>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let authorization_type = req
        .get("authorizationType")
        .and_then(Value::as_str)
        .unwrap_or("NONE")
        .to_string();
    let api_key_required = req
        .get("apiKeyRequired")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let Some(resource) = api.resources.get_mut(&resource_id) else {
        return rest_error(not_found("resource", &resource_id));
    };
    let method = Method {
        http_method: http_method.clone(),
        authorization_type,
        api_key_required,
        integration: None,
    };
    let body = method_json(&method);
    resource.methods.insert(http_method, method);
    rest_json(StatusCode::CREATED, &body)
}

async fn get_method(
    State(state): State<AgwState>,
    Path((api_id, resource_id, http_method)): Path<(String, String, String)>,
) -> Response {
    let s = state.read();
    let api = match s.apis.get(&api_id) {
        Some(a) => a,
        None => return rest_error(not_found("rest api", &api_id)),
    };
    let resource = match api.resources.get(&resource_id) {
        Some(r) => r,
        None => return rest_error(not_found("resource", &resource_id)),
    };
    match resource.methods.get(&http_method) {
        Some(m) => rest_json(StatusCode::OK, &method_json(m)),
        None => rest_error(not_found("method", &http_method)),
    }
}

async fn delete_method(
    State(state): State<AgwState>,
    Path((api_id, resource_id, http_method)): Path<(String, String, String)>,
) -> Response {
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let Some(resource) = api.resources.get_mut(&resource_id) else {
        return rest_error(not_found("resource", &resource_id));
    };
    if resource.methods.remove(&http_method).is_none() {
        return rest_error(not_found("method", &http_method));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn put_integration(
    State(state): State<AgwState>,
    Path((api_id, resource_id, http_method)): Path<(String, String, String)>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let type_ = req
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("AWS_PROXY")
        .to_string();
    let integration_http_method = req
        .get("integrationHttpMethod")
        .and_then(Value::as_str)
        .map(String::from);
    let uri = req.get("uri").and_then(Value::as_str).map(String::from);
    let request_templates: HashMap<String, String> = req
        .get("requestTemplates")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let Some(resource) = api.resources.get_mut(&resource_id) else {
        return rest_error(not_found("resource", &resource_id));
    };
    let Some(method) = resource.methods.get_mut(&http_method) else {
        return rest_error(not_found("method", &http_method));
    };
    let integration = Integration {
        type_,
        integration_http_method,
        uri,
        request_templates,
    };
    let body = integration_json(&integration);
    method.integration = Some(integration);
    rest_json(StatusCode::CREATED, &body)
}

async fn get_integration(
    State(state): State<AgwState>,
    Path((api_id, resource_id, http_method)): Path<(String, String, String)>,
) -> Response {
    let s = state.read();
    let api = match s.apis.get(&api_id) {
        Some(a) => a,
        None => return rest_error(not_found("rest api", &api_id)),
    };
    let resource = match api.resources.get(&resource_id) {
        Some(r) => r,
        None => return rest_error(not_found("resource", &resource_id)),
    };
    let method = match resource.methods.get(&http_method) {
        Some(m) => m,
        None => return rest_error(not_found("method", &http_method)),
    };
    match &method.integration {
        Some(i) => rest_json(StatusCode::OK, &integration_json(i)),
        None => rest_error(not_found("integration", &http_method)),
    }
}

async fn create_deployment(
    State(state): State<AgwState>,
    Path(api_id): Path<String>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let stage_name = req
        .get("stageName")
        .and_then(Value::as_str)
        .map(String::from);
    let id = short_id();
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    let now = chrono::Utc::now();
    api.deployments.insert(id.clone(), now);
    if let Some(stage) = &stage_name {
        api.stages.insert(stage.clone(), id.clone());
    }
    rest_json(
        StatusCode::CREATED,
        &json!({
            "id": id,
            "createdDate": now.timestamp(),
            "description": Value::Null,
        }),
    )
}

async fn create_stage(
    State(state): State<AgwState>,
    Path(api_id): Path<String>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => json!({}),
    };
    let stage_name = match req.get("stageName").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => return rest_error(AwsError::new("BadRequestException", "stageName required")),
    };
    let deployment_id = match req.get("deploymentId").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return rest_error(AwsError::new(
                "BadRequestException",
                "deploymentId required",
            ));
        }
    };
    let mut s = state.write();
    let Some(api) = s.apis.get_mut(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    if !api.deployments.contains_key(&deployment_id) {
        return rest_error(not_found("deployment", &deployment_id));
    }
    api.stages.insert(stage_name.clone(), deployment_id.clone());
    rest_json(
        StatusCode::CREATED,
        &json!({
            "stageName": stage_name,
            "deploymentId": deployment_id,
            "createdDate": chrono::Utc::now().timestamp(),
        }),
    )
}

async fn get_stage(
    State(state): State<AgwState>,
    Path((api_id, stage_name)): Path<(String, String)>,
) -> Response {
    let s = state.read();
    let Some(api) = s.apis.get(&api_id) else {
        return rest_error(not_found("rest api", &api_id));
    };
    match api.stages.get(&stage_name) {
        Some(dep) => rest_json(
            StatusCode::OK,
            &json!({
                "stageName": stage_name,
                "deploymentId": dep,
            }),
        ),
        None => rest_error(not_found("stage", &stage_name)),
    }
}

fn api_json(a: &RestApi) -> Value {
    json!({
        "id": a.id,
        "name": a.name,
        "description": a.description,
        "createdDate": a.created.timestamp(),
        "apiKeySource": "HEADER",
        "endpointConfiguration": {"types": ["REGIONAL"]},
    })
}

fn resource_json(r: &Resource) -> Value {
    let mut v = json!({
        "id": r.id,
        "path": r.path,
    });
    if let Some(p) = &r.parent_id {
        v["parentId"] = Value::String(p.clone());
    }
    if let Some(p) = &r.path_part {
        v["pathPart"] = Value::String(p.clone());
    }
    if !r.methods.is_empty() {
        let methods: serde_json::Map<String, Value> =
            r.methods.keys().map(|k| (k.clone(), json!({}))).collect();
        v["resourceMethods"] = Value::Object(methods);
    }
    v
}

fn method_json(m: &Method) -> Value {
    json!({
        "httpMethod": m.http_method,
        "authorizationType": m.authorization_type,
        "apiKeyRequired": m.api_key_required,
    })
}

fn integration_json(i: &Integration) -> Value {
    let mut v = json!({
        "type": i.type_,
        "requestTemplates": i.request_templates,
    });
    if let Some(m) = &i.integration_http_method {
        v["httpMethod"] = Value::String(m.clone());
    }
    if let Some(u) = &i.uri {
        v["uri"] = Value::String(u.clone());
    }
    v
}

fn not_found(kind: &str, id: &str) -> AwsError {
    AwsError::new("NotFoundException", format!("{kind} '{id}' not found"))
        .status(StatusCode::NOT_FOUND)
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..10].to_string()
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_error(err: AwsError) -> Response {
    let body = json!({
        "Type": err.code,
        "message": err.message,
    });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(ApiGateway::new()));
}
