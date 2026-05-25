//! Reusable "named-resource CRUD" stub for REST/JSON services that just need a
//! minimal control-plane surface. Each instance owns one collection of opaque
//! resources keyed by name; create/list/describe/delete are wired automatically.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{Service, ServiceContext, persistence_error};
use crate::services::rest_helpers::{rest_error, rest_json};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct ResourceState {
    resources: HashMap<String, Value>,
}

pub struct ResourceStub {
    name: &'static str,
    list_path: &'static str,
    item_path: &'static str,
    name_field: &'static str,
    list_field: &'static str,
    state: Arc<RwLock<ResourceState>>,
}

impl ResourceStub {
    pub fn new(
        name: &'static str,
        list_path: &'static str,
        item_path: &'static str,
        name_field: &'static str,
        list_field: &'static str,
    ) -> Self {
        Self {
            name,
            list_path,
            item_path,
            name_field,
            list_field,
            state: Arc::new(RwLock::new(ResourceState::default())),
        }
    }
}

#[derive(Clone)]
struct StubContext {
    state: Arc<RwLock<ResourceState>>,
    name_field: &'static str,
    list_field: &'static str,
}

#[async_trait]
impl Service for ResourceStub {
    fn name(&self) -> &'static str {
        self.name
    }
    fn reset(&self) {
        *self.state.write() = ResourceState::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(d) = snap
                .load::<ResourceState>(self.name)
                .map_err(persistence_error)?
        {
            *self.state.write() = d;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save(self.name, &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let ctx = StubContext {
            state: self.state.clone(),
            name_field: self.name_field,
            list_field: self.list_field,
        };
        Router::new()
            .route(self.list_path, post(create).get(list))
            .route(self.item_path, get(describe).delete(delete))
            .with_state(ctx)
    }
}

async fn create(State(ctx): State<StubContext>, body: Bytes) -> Response {
    let mut req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let name = req
        .get(ctx.name_field)
        .and_then(Value::as_str)
        .or_else(|| req.get("Name").and_then(Value::as_str))
        .or_else(|| req.get("name").and_then(Value::as_str))
        .map(String::from)
        .unwrap_or_else(|| format!("res-{}", uuid::Uuid::new_v4().simple()));
    if let Some(o) = req.as_object_mut() {
        o.insert(ctx.name_field.to_string(), json!(name.clone()));
        o.insert("Arn".to_string(), json!(format!("arn::stub::{name}")));
    }
    ctx.state
        .write()
        .resources
        .insert(name.clone(), req.clone());
    rest_json(StatusCode::OK, &req)
}

async fn list(State(ctx): State<StubContext>) -> Response {
    let s = ctx.state.read();
    let items: Vec<Value> = s.resources.values().cloned().collect();
    let mut obj = serde_json::Map::new();
    obj.insert(ctx.list_field.to_string(), Value::Array(items));
    rest_json(StatusCode::OK, &Value::Object(obj))
}

async fn describe(State(ctx): State<StubContext>, Path(name): Path<String>) -> Response {
    let s = ctx.state.read();
    match s.resources.get(&name) {
        Some(v) => rest_json(StatusCode::OK, v),
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn delete(State(ctx): State<StubContext>, Path(name): Path<String>) -> Response {
    let mut s = ctx.state.write();
    if s.resources.remove(&name).is_some() {
        rest_json(StatusCode::OK, &json!({}))
    } else {
        rest_error(AwsError::new("ResourceNotFoundException", "not found"))
    }
}
