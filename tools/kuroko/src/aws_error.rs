//! AWS-style error envelopes for the four supported wire protocols.
//!
//! AWS SDKs do not parse a single "error JSON" — every protocol carries its own
//! shape. We centralize the formatting so each service can throw a
//! `AwsError::new(code, message).status(404)` and the dispatcher does the right
//! thing for the inbound protocol.

use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

/// Wire-level error returned by a service handler. The protocol layer converts
/// this into the right envelope (REST XML, JSON `__type`, query XML, CBOR).
///
/// `request_id` is generated once at construction and is not externally
/// settable — callers that need to override it (test fixtures, recorded
/// snapshots) go through `with_request_id`.
#[derive(Debug, Clone)]
pub struct AwsError {
    pub code: String,
    pub message: String,
    pub status: StatusCode,
    pub(crate) request_id: String,
}

impl AwsError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message).status(StatusCode::NOT_FOUND)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("InternalFailure", message).status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn unsupported(operation: impl AsRef<str>) -> Self {
        Self::new(
            "UnsupportedOperation",
            format!(
                "Operation '{}' is not yet implemented by kuroko.",
                operation.as_ref()
            ),
        )
        .status(StatusCode::NOT_IMPLEMENTED)
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "__type": self.code,
            "message": self.message,
            "Message": self.message,
        })
    }

    pub fn to_query_xml(&self) -> String {
        format!(
            "<ErrorResponse xmlns=\"https://kuroko.local/\">\
<Error><Type>Sender</Type><Code>{code}</Code><Message>{msg}</Message></Error>\
<RequestId>{rid}</RequestId>\
</ErrorResponse>",
            code = xml_escape(&self.code),
            msg = xml_escape(&self.message),
            rid = self.request_id,
        )
    }

    pub fn to_rest_xml(&self) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Error><Code>{code}</Code><Message>{msg}</Message><RequestId>{rid}</RequestId></Error>",
            code = xml_escape(&self.code),
            msg = xml_escape(&self.message),
            rid = self.request_id,
        )
    }

    /// Build an axum `Response` carrying the REST/XML envelope. Used by
    /// REST-protocol services (S3 and friends).
    pub fn to_rest_xml_response(&self) -> Response {
        Response::builder()
            .status(self.status)
            .header(CONTENT_TYPE, "application/xml")
            .header("x-amz-request-id", &self.request_id)
            .body(Body::from(self.to_rest_xml()))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}

/// `IntoResponse` default uses the JSON envelope; protocol-specific dispatchers
/// override by calling `to_query_xml` / `to_rest_xml` directly.
impl IntoResponse for AwsError {
    fn into_response(self) -> Response {
        let body = serde_json::to_vec(&self.to_json()).unwrap_or_default();
        Response::builder()
            .status(self.status)
            .header(CONTENT_TYPE, "application/x-amz-json-1.0")
            .header("x-amzn-requestid", &self.request_id)
            .header("x-amzn-errortype", &self.code)
            .body(Body::from(body))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_envelope_includes_type_and_message() {
        let err = AwsError::new("ResourceNotFoundException", "missing");
        let v = err.to_json();
        assert_eq!(v["__type"], "ResourceNotFoundException");
        assert_eq!(v["message"], "missing");
    }

    #[test]
    fn xml_escapes_special_chars() {
        assert_eq!(xml_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
    }
}
