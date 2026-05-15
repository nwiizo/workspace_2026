//! Smithy RPC v2 CBOR dispatcher.
//!
//! Routes via `POST /service/{service}/operation/{operation}` with a CBOR body.
//! Used by newer services like CloudWatch's GraniteService.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use bytes::Bytes;

use crate::aws_error::AwsError;
use crate::registry::Registry;
use crate::service::ServiceContext;

pub async fn handler(
    State((registry, ctx)): State<(Arc<Registry>, ServiceContext)>,
    Path((service, operation)): Path<(String, String)>,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let svc = match registry.cbor_for_service(&service) {
        Some(s) => s,
        None => {
            return error_response(AwsError::new(
                "UnknownService",
                format!("no CBOR service registered as {service}"),
            ));
        }
    };

    match svc.dispatch(ctx, &operation, body).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/cbor")
            .header("smithy-protocol", "rpc-v2-cbor")
            .body(Body::from(bytes))
            .unwrap(),
        Err(err) => error_response(err),
    }
}

fn error_response(err: AwsError) -> Response {
    let mut buf = Vec::new();
    let v = err.to_json();
    if ciborium::ser::into_writer(&v, &mut buf).is_err() {
        buf = serde_json::to_vec(&v).unwrap_or_default();
    }
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/cbor")
        .header("smithy-protocol", "rpc-v2-cbor")
        .header("x-amzn-requestid", err.request_id())
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(buf))
        .unwrap()
}
