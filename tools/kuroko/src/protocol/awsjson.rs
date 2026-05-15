//! AWS JSON 1.0 / 1.1 dispatcher.
//!
//! AWS SDKs put the action in `X-Amz-Target: <TargetPrefix>.<Action>` and POST
//! a JSON body to `/`. We look the prefix up in the registry, hand the action
//! to the service, then wrap the result in either an `application/x-amz-json-1.0`
//! or `application/x-amz-json-1.1` envelope (the content type is echoed from
//! the request).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use bytes::Bytes;

use crate::aws_error::AwsError;
use crate::registry::Registry;
use crate::service::ServiceContext;

pub async fn dispatch(
    registry: Arc<Registry>,
    ctx: ServiceContext,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = match headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        Some(t) => t.to_string(),
        None => {
            return error_response(
                &headers,
                AwsError::new(
                    "InvalidAction",
                    "missing X-Amz-Target header for AWS JSON protocol",
                ),
            );
        }
    };

    let (svc, action) = match registry.json_for_target(&target) {
        Some(v) => v,
        None => {
            return error_response(
                &headers,
                AwsError::new(
                    "UnknownOperationException",
                    format!("no service registered for target {target}"),
                ),
            );
        }
    };

    match svc.dispatch(ctx, &action, body).await {
        Ok(value) => success_response(&headers, &value),
        Err(err) => error_response(&headers, err),
    }
}

/// Handler for axum: extracts state and forwards to `dispatch`.
pub async fn handler(
    State((registry, ctx)): State<(Arc<Registry>, ServiceContext)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    dispatch(registry, ctx, headers, body).await
}

fn content_type_for(headers: &HeaderMap) -> String {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/x-amz-json-1.0".to_string())
}

fn success_response(headers: &HeaderMap, value: &serde_json::Value) -> Response {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for(headers))
        .header("x-amzn-requestid", uuid::Uuid::new_v4().to_string())
        .body(Body::from(body))
        .unwrap()
}

fn error_response(headers: &HeaderMap, err: AwsError) -> Response {
    let body = serde_json::to_vec(&err.to_json()).unwrap_or_default();
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, content_type_for(headers))
        .header("x-amzn-requestid", err.request_id())
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(body))
        .unwrap()
}
