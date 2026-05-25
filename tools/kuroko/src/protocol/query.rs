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

/// User-Agent disambiguates AWS Query actions shared by multiple services
/// (e.g. `CreateDBCluster` lives in rds, neptune, docdb). AWS SDKs across
/// languages and major versions ship several different UA token shapes:
///
/// - `aws-sdk-rust/1.4 service/sqs lang/rust/1.78` (older Rust SDK)
/// - `aws-sdk-rust/1.x ua/2.0 lib/rds#1.x os/macos lang/rust/1.x` (newer)
/// - `aws-sdk-java/2.x ... api/rds ...`
/// - The dedicated `x-amz-user-agent` header (always present on newer SDKs)
///   carries the same token shapes, sometimes more reliably than `user-agent`
///   itself.
///
/// We check both headers and accept `service/X`, `api/X`, `lib/X#…`, and
/// `aws-sdk-X/…` forms. Returns the *first* match found, lower-cased.
fn parse_sdk_id(headers: &HeaderMap) -> Option<String> {
    let amz = headers
        .get("x-amz-user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    for source in [amz, ua] {
        for token in source.split_whitespace() {
            for prefix in ["service/", "api/", "lib/"] {
                if let Some(rest) = token.strip_prefix(prefix) {
                    let id = rest.split(['/', '#']).next().unwrap_or(rest);
                    if !id.is_empty() {
                        return Some(id.to_ascii_lowercase());
                    }
                }
            }
            // `aws-sdk-rds/1.x.x` — strip the `aws-sdk-` family prefix but
            // skip the meta `aws-sdk-rust` / `aws-sdk-go` etc. tokens that
            // name the language, not the AWS service.
            if let Some(rest) = token.strip_prefix("aws-sdk-")
                && let Some(id) = rest.split('/').next()
                && !id.is_empty()
                && !["rust", "go", "java", "js", "python", "php", "ruby", "cpp"].contains(&id)
            {
                return Some(id.to_ascii_lowercase());
            }
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
