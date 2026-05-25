//! Amazon Location Service — restJson1.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, Service, ServiceContext, persistence_error,
};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    place_indexes: HashMap<String, Resource>,
    maps: HashMap<String, Resource>,
    geofence_collections: HashMap<String, Resource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Resource {
    name: String,
    arn: String,
    description: String,
}

pub struct Location {
    state: Arc<RwLock<State_>>,
}
impl Location {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for Location {
    fn default() -> Self {
        Self::new()
    }
}

type LState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Location {
    fn name(&self) -> &'static str {
        "location"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("location").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            snap.save("location", &*self.state.read())
                .map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let s = self.state.clone();
        Router::new()
            .route(
                "/places/v0/indexes",
                post(create_place_index).get(list_place_indexes),
            )
            .route(
                "/places/v0/indexes/{name}",
                get(describe_place_index).delete(delete_place_index),
            )
            .route("/maps/v0/maps", post(create_map).get(list_maps))
            .route("/maps/v0/maps/{name}", get(describe_map).delete(delete_map))
            .route(
                "/geofencing/v0/collections",
                post(create_geofence_collection).get(list_geofence_collections),
            )
            .with_state(s)
    }
}

fn arn(kind: &str, name: &str) -> String {
    format!("arn:aws:geo:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:{kind}/{name}")
}

fn parse_name(body: &Bytes, key: &str) -> Result<(String, String), Box<Response>> {
    let req: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let name = req
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Box::new(rest_error(AwsError::new(
                "ValidationException",
                format!("{key} required"),
            )))
        })?
        .to_string();
    let desc = req
        .get("Description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok((name, desc))
}

async fn create_place_index(State(state): State<LState>, body: Bytes) -> Response {
    let (name, desc) = match parse_name(&body, "IndexName") {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let a = arn("place-index", &name);
    state.write().place_indexes.insert(
        name.clone(),
        Resource {
            name: name.clone(),
            arn: a.clone(),
            description: desc,
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({
            "IndexName": name,
            "IndexArn": a,
            "CreateTime": chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn list_place_indexes(State(state): State<LState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s
        .place_indexes
        .values()
        .map(|r| {
            json!({
                "IndexName": r.name,
                "Description": r.description,
                "CreateTime": chrono::Utc::now().to_rfc3339(),
                "UpdateTime": chrono::Utc::now().to_rfc3339(),
                "DataSource": "Esri",
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "Entries": items }))
}

async fn describe_place_index(State(state): State<LState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    match s.place_indexes.get(&name) {
        Some(r) => rest_json(
            StatusCode::OK,
            &json!({
                "IndexName": r.name,
                "IndexArn": r.arn,
                "Description": r.description,
                "DataSource": "Esri",
                "DataSourceConfiguration": { "IntendedUse": "SingleUse" },
                "CreateTime": chrono::Utc::now().to_rfc3339(),
                "UpdateTime": chrono::Utc::now().to_rfc3339(),
            }),
        ),
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn delete_place_index(State(state): State<LState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    if s.place_indexes.remove(&name).is_some() {
        rest_json(StatusCode::OK, &json!({}))
    } else {
        rest_error(AwsError::new("ResourceNotFoundException", "not found"))
    }
}

async fn create_map(State(state): State<LState>, body: Bytes) -> Response {
    let (name, desc) = match parse_name(&body, "MapName") {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let a = arn("map", &name);
    state.write().maps.insert(
        name.clone(),
        Resource {
            name: name.clone(),
            arn: a.clone(),
            description: desc,
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({
            "MapName": name,
            "MapArn": a,
            "CreateTime": chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn list_maps(State(state): State<LState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s
        .maps
        .values()
        .map(|r| {
            json!({
                "MapName": r.name,
                "Description": r.description,
                "DataSource": "Esri",
                "CreateTime": chrono::Utc::now().to_rfc3339(),
                "UpdateTime": chrono::Utc::now().to_rfc3339(),
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "Entries": items }))
}

async fn describe_map(State(state): State<LState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    match s.maps.get(&name) {
        Some(r) => rest_json(
            StatusCode::OK,
            &json!({
                "MapName": r.name,
                "MapArn": r.arn,
                "Description": r.description,
                "DataSource": "Esri",
                "Configuration": { "Style": "VectorEsriStreets" },
                "CreateTime": chrono::Utc::now().to_rfc3339(),
                "UpdateTime": chrono::Utc::now().to_rfc3339(),
            }),
        ),
        None => rest_error(AwsError::new("ResourceNotFoundException", "not found")),
    }
}

async fn delete_map(State(state): State<LState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    if s.maps.remove(&name).is_some() {
        rest_json(StatusCode::OK, &json!({}))
    } else {
        rest_error(AwsError::new("ResourceNotFoundException", "not found"))
    }
}

async fn create_geofence_collection(State(state): State<LState>, body: Bytes) -> Response {
    let (name, desc) = match parse_name(&body, "CollectionName") {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let a = arn("geofence-collection", &name);
    state.write().geofence_collections.insert(
        name.clone(),
        Resource {
            name: name.clone(),
            arn: a.clone(),
            description: desc,
        },
    );
    rest_json(
        StatusCode::OK,
        &json!({
            "CollectionName": name,
            "CollectionArn": a,
            "CreateTime": chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn list_geofence_collections(State(state): State<LState>) -> Response {
    let s = state.read();
    let items: Vec<Value> = s
        .geofence_collections
        .values()
        .map(|r| {
            json!({
                "CollectionName": r.name,
                "Description": r.description,
                "CreateTime": chrono::Utc::now().to_rfc3339(),
                "UpdateTime": chrono::Utc::now().to_rfc3339(),
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "Entries": items }))
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_error(err: AwsError) -> Response {
    let body = json!({ "Message": err.message });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Location::new()));
}
