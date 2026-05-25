//! Shared REST/JSON response helpers used by minimal stub services.

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};

use crate::aws_error::AwsError;

pub(crate) fn rest_json(status: StatusCode, body: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap_or_default()))
        .unwrap()
}

pub(crate) fn rest_error(err: AwsError) -> Response {
    let body = json!({ "Message": err.message });
    Response::builder()
        .status(err.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-amzn-errortype", &err.code)
        .body(Body::from(serde_json::to_vec(&body).unwrap_or_default()))
        .unwrap()
}
