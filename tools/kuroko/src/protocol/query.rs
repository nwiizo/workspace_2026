//! AWS Query protocol dispatcher.
//!
//! Query services accept form-urlencoded bodies with `Action=Foo&Version=...`
//! and respond with an XML envelope. SQS technically uses AWS JSON 1.0 now, so
//! Query is mostly EC2, RDS, IAM, ELB, SNS (legacy), etc.

use std::collections::HashMap;
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
    let params = match parse_form(&body) {
        Ok(p) => p,
        Err(err) => return error_response(err),
    };

    let action = match params.get("Action") {
        Some(a) => a.clone(),
        None => {
            return error_response(AwsError::new(
                "InvalidAction",
                "missing Action parameter for AWS Query protocol",
            ));
        }
    };

    let sdk_id = parse_sdk_id(&headers);
    let svc = match registry.query_for_action(&action, sdk_id.as_deref()) {
        Some(s) => s,
        None => {
            return error_response(AwsError::new(
                "InvalidAction",
                format!("no service handles action {action}"),
            ));
        }
    };

    match svc.dispatch(ctx, &action, &params).await {
        Ok(xml) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/xml")
            .header("x-amzn-requestid", uuid::Uuid::new_v4().to_string())
            .body(Body::from(xml))
            .unwrap(),
        Err(err) => error_response(err),
    }
}

pub async fn handler(
    State((registry, ctx)): State<(Arc<Registry>, ServiceContext)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    dispatch(registry, ctx, headers, body).await
}

fn parse_form(body: &Bytes) -> Result<HashMap<String, String>, AwsError> {
    serde_urlencoded::from_bytes::<Vec<(String, String)>>(body)
        .map(|pairs| pairs.into_iter().collect())
        .map_err(|e| AwsError::new("MalformedQueryString", e.to_string()))
}

/// User-Agent contains tokens like `aws-sdk-rust/1.4 service/sqs ...` — used to
/// disambiguate Query action names shared by multiple services.
fn parse_sdk_id(headers: &HeaderMap) -> Option<String> {
    let ua = headers.get(header::USER_AGENT)?.to_str().ok()?;
    for token in ua.split_whitespace() {
        if let Some(rest) = token.strip_prefix("service/") {
            return Some(rest.split('/').next().unwrap_or(rest).to_string());
        }
        if let Some(rest) = token.strip_prefix("api/") {
            return Some(rest.split('/').next().unwrap_or(rest).to_string());
        }
    }
    None
}

fn error_response(err: AwsError) -> Response {
    let xml = err.to_query_xml();
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "text/xml")
        .header("x-amzn-requestid", err.request_id())
        .body(Body::from(xml))
        .unwrap()
}
