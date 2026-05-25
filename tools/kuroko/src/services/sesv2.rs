//! SES v2 — REST protocol under `/v2/email/*`.

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
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{Service, ServiceContext, persistence_error};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    identities: HashMap<String, bool>,
    sent_messages: usize,
}

pub struct SesV2 {
    state: Arc<RwLock<State_>>,
}

impl SesV2 {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}
impl Default for SesV2 {
    fn default() -> Self {
        Self::new()
    }
}

type SesState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for SesV2 {
    fn name(&self) -> &'static str {
        "sesv2"
    }
    fn reset(&self) {
        *self.state.write() = State_::default();
    }
    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("sesv2").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }
    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("sesv2", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }
    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        Router::new()
            .route(
                "/v2/email/identities",
                post(create_identity).get(list_identities),
            )
            .route(
                "/v2/email/identities/{name}",
                get(get_identity).delete(delete_identity),
            )
            .route("/v2/email/outbound-emails", post(send_email))
            .with_state(state)
    }
}

async fn create_identity(State(state): State<SesState>, body: Bytes) -> Response {
    let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let email = match req.get("EmailIdentity").and_then(Value::as_str) {
        Some(e) => e.to_string(),
        None => return rest_err("EmailIdentity required"),
    };
    state.write().identities.insert(email.clone(), true);
    rest_json(
        StatusCode::OK,
        &json!({ "IdentityType": "EMAIL_ADDRESS", "VerifiedForSendingStatus": true }),
    )
}

async fn list_identities(State(state): State<SesState>) -> Response {
    let s = state.read();
    let identities: Vec<_> = s
        .identities
        .keys()
        .map(|i| {
            json!({
                "IdentityName": i,
                "IdentityType": "EMAIL_ADDRESS",
                "SendingEnabled": true,
            })
        })
        .collect();
    rest_json(StatusCode::OK, &json!({ "EmailIdentities": identities }))
}

async fn get_identity(State(state): State<SesState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    if !s.identities.contains_key(&name) {
        return rest_err_404("identity not found");
    }
    rest_json(
        StatusCode::OK,
        &json!({
            "IdentityType": "EMAIL_ADDRESS",
            "VerifiedForSendingStatus": true,
            "FeedbackForwardingStatus": false,
        }),
    )
}

async fn delete_identity(State(state): State<SesState>, Path(name): Path<String>) -> Response {
    state.write().identities.remove(&name);
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("{}"))
        .unwrap()
}

async fn send_email(State(state): State<SesState>, _body: Bytes) -> Response {
    let mut s = state.write();
    s.sent_messages += 1;
    rest_json(
        StatusCode::OK,
        &json!({ "MessageId": Uuid::new_v4().to_string() }),
    )
}

fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

fn rest_err(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", "BadRequestException")
        .body(Body::from(
            serde_json::to_vec(&json!({ "Type": "BadRequestException", "message": msg }))
                .unwrap_or_default(),
        ))
        .unwrap()
}

fn rest_err_404(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", "NotFoundException")
        .body(Body::from(
            serde_json::to_vec(&json!({ "Type": "NotFoundException", "message": msg }))
                .unwrap_or_default(),
        ))
        .unwrap()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(SesV2::new()));
}
