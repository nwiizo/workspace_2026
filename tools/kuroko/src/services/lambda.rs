//! Lambda — REST protocol (restJson1), mounted under `/2015-03-31/functions/*`.
//!
//! kuroko stores function configurations and code metadata but does **not**
//! execute Lambda code. Invoke returns an "echo" response: the payload sent
//! to the function is returned verbatim, wrapped in a 200 response with
//! `X-Amz-Executed-Version: $LATEST`. This is enough for tests that only need
//! Lambda's API surface (e.g. infrastructure-as-code provisioning, IAM role
//! wiring, function listing).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, Service, ServiceContext, persistence_error,
};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State_ {
    functions: HashMap<String, Function>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Function {
    name: String,
    arn: String,
    runtime: String,
    role: String,
    handler: String,
    description: String,
    timeout: i32,
    memory_size: i32,
    code_size: i64,
    code_sha256: String,
    last_modified: chrono::DateTime<chrono::Utc>,
    version: String,
    environment: HashMap<String, String>,
}

pub struct Lambda {
    state: Arc<RwLock<State_>>,
}

impl Lambda {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State_::default())),
        }
    }
}

impl Default for Lambda {
    fn default() -> Self {
        Self::new()
    }
}

type LambdaState = Arc<RwLock<State_>>;

#[async_trait]
impl Service for Lambda {
    fn name(&self) -> &'static str {
        "lambda"
    }

    fn reset(&self) {
        self.state.write().functions.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State_>("lambda").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("lambda", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        let state = self.state.clone();
        // AWS SDK sends both with and without a trailing slash on the
        // collection endpoint; axum 0.8 treats them as distinct, so we
        // register both shapes.
        Router::new()
            .route(
                "/2015-03-31/functions/",
                post(create_function).get(list_functions),
            )
            .route(
                "/2015-03-31/functions",
                post(create_function).get(list_functions),
            )
            .route(
                "/2015-03-31/functions/{name}",
                get(get_function).delete(delete_function),
            )
            .route(
                "/2015-03-31/functions/{name}/configuration",
                axum::routing::put(update_function_configuration),
            )
            .route("/2015-03-31/functions/{name}/invocations", post(invoke))
            .with_state(state)
    }
}

async fn create_function(State(state): State<LambdaState>, body: Bytes) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return rest_error(AwsError::new(
                "InvalidRequestContentException",
                e.to_string(),
            ));
        }
    };
    let name = match req.get("FunctionName").and_then(Value::as_str) {
        Some(n) => n.to_string(),
        None => {
            return rest_error(AwsError::new(
                "ValidationException",
                "FunctionName required",
            ));
        }
    };
    let mut s = state.write();
    if s.functions.contains_key(&name) {
        return rest_error(
            AwsError::new(
                "ResourceConflictException",
                format!("function '{name}' already exists"),
            )
            .status(StatusCode::CONFLICT),
        );
    }
    let code_size = req
        .get("Code")
        .and_then(|c| c.get("ZipFile"))
        .and_then(Value::as_str)
        .map(|s| s.len() as i64)
        .unwrap_or(0);
    let code_sha256 = req
        .get("Code")
        .and_then(|c| c.get("ZipFile"))
        .and_then(Value::as_str)
        .map(sha256_of)
        .unwrap_or_else(|| "0".repeat(64));
    let function = Function {
        name: name.clone(),
        arn: function_arn(&name),
        runtime: str_or(&req, "Runtime", "provided.al2"),
        role: str_or(&req, "Role", ""),
        handler: str_or(&req, "Handler", "index.handler"),
        description: str_or(&req, "Description", ""),
        timeout: req.get("Timeout").and_then(Value::as_i64).unwrap_or(3) as i32,
        memory_size: req.get("MemorySize").and_then(Value::as_i64).unwrap_or(128) as i32,
        code_size,
        code_sha256,
        last_modified: chrono::Utc::now(),
        version: "$LATEST".into(),
        environment: parse_env(&req),
    };
    let resp = function_json(&function);
    s.functions.insert(name, function);
    rest_json(StatusCode::CREATED, &resp)
}

async fn list_functions(State(state): State<LambdaState>) -> Response {
    let s = state.read();
    let functions: Vec<_> = s.functions.values().map(function_json).collect();
    rest_json(StatusCode::OK, &json!({ "Functions": functions }))
}

async fn get_function(State(state): State<LambdaState>, Path(name): Path<String>) -> Response {
    let s = state.read();
    let Some(f) = s.functions.get(&name) else {
        return rest_error(not_found(&name));
    };
    rest_json(
        StatusCode::OK,
        &json!({
            "Configuration": function_json(f),
            "Code": {
                "RepositoryType": "S3",
                "Location": "https://kuroko/none",
            },
        }),
    )
}

async fn delete_function(State(state): State<LambdaState>, Path(name): Path<String>) -> Response {
    let mut s = state.write();
    if s.functions.remove(&name).is_none() {
        return rest_error(not_found(&name));
    }
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap()
}

async fn update_function_configuration(
    State(state): State<LambdaState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return rest_error(AwsError::new(
                "InvalidRequestContentException",
                e.to_string(),
            ));
        }
    };
    let mut s = state.write();
    let Some(f) = s.functions.get_mut(&name) else {
        return rest_error(not_found(&name));
    };
    if let Some(r) = req.get("Runtime").and_then(Value::as_str) {
        f.runtime = r.to_string();
    }
    if let Some(h) = req.get("Handler").and_then(Value::as_str) {
        f.handler = h.to_string();
    }
    if let Some(d) = req.get("Description").and_then(Value::as_str) {
        f.description = d.to_string();
    }
    if let Some(t) = req.get("Timeout").and_then(Value::as_i64) {
        f.timeout = t as i32;
    }
    if let Some(m) = req.get("MemorySize").and_then(Value::as_i64) {
        f.memory_size = m as i32;
    }
    if req.get("Environment").is_some() {
        f.environment = parse_env(&req);
    }
    f.last_modified = chrono::Utc::now();
    rest_json(StatusCode::OK, &function_json(f))
}

#[derive(serde::Deserialize)]
struct InvokeParams {
    // AWS SDKs sometimes pass `?Qualifier=$LATEST` to Invoke. We accept and
    // ignore it — kuroko has no concept of function versions.
    #[serde(rename = "Qualifier")]
    #[allow(dead_code)]
    qualifier: Option<String>,
}

async fn invoke(
    State(state): State<LambdaState>,
    Path(name): Path<String>,
    Query(_qs): Query<InvokeParams>,
    body: Bytes,
) -> Response {
    let s = state.read();
    if !s.functions.contains_key(&name) {
        return rest_error(not_found(&name));
    }
    // Echo the payload verbatim. Real Lambda would execute user code; kuroko
    // intentionally does not — tests that just exercise the API contract
    // (e.g. SDK builders, Invoke parameter shapes) still pass.
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Amz-Executed-Version", "$LATEST")
        .body(Body::from(body))
        .unwrap()
}

fn function_arn(name: &str) -> String {
    format!("arn:aws:lambda:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:function:{name}")
}

fn function_json(f: &Function) -> Value {
    json!({
        "FunctionName": f.name,
        "FunctionArn": f.arn,
        "Runtime": f.runtime,
        "Role": f.role,
        "Handler": f.handler,
        "CodeSize": f.code_size,
        "Description": f.description,
        "Timeout": f.timeout,
        "MemorySize": f.memory_size,
        "LastModified": f.last_modified.to_rfc3339(),
        "CodeSha256": f.code_sha256,
        "Version": f.version,
        "Environment": { "Variables": f.environment },
        "PackageType": "Zip",
        "State": "Active",
    })
}

fn parse_env(req: &Value) -> HashMap<String, String> {
    req.get("Environment")
        .and_then(|e| e.get("Variables"))
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn str_or(req: &Value, key: &str, default: &str) -> String {
    req.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn not_found(name: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("function '{name}' not found"),
    )
    .status(StatusCode::NOT_FOUND)
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

fn sha256_of(input: &str) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(input.as_bytes());
    BASE64.encode(h.finalize())
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register(Arc::new(Lambda::new()));
}
