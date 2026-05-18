//! ACM — AWS JSON 1.1, target prefix `CertificateManager`.
//!
//! Certificate lifecycle metadata only — kuroko does not actually issue or
//! validate TLS certificates. Tests verify request/response shapes that IaC
//! provisioning depends on.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use parking_lot::RwLock;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::aws_error::AwsError;
use crate::service::{
    EMULATED_ACCOUNT_ID, EMULATED_REGION, JsonProtocolService, Service, ServiceContext,
    persistence_error,
};

const TARGET_PREFIX: &str = "CertificateManager";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct State {
    certificates: HashMap<String, Certificate>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Certificate {
    arn: String,
    domain_name: String,
    subject_alternative_names: Vec<String>,
    status: String,
    type_: String,
    issued_at: chrono::DateTime<chrono::Utc>,
    not_after: chrono::DateTime<chrono::Utc>,
    serial: String,
    certificate_pem: Option<String>,
    private_key_pem: Option<String>,
}

pub struct Acm {
    state: Arc<RwLock<State>>,
}

impl Acm {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }
}

impl Default for Acm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for Acm {
    fn name(&self) -> &'static str {
        "acm"
    }

    fn reset(&self) {
        self.state.write().certificates.clear();
    }

    fn restore(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot
            && let Some(data) = snap.load::<State>("acm").map_err(persistence_error)?
        {
            *self.state.write() = data;
        }
        Ok(())
    }

    fn snapshot(&self, ctx: &ServiceContext) -> Result<(), AwsError> {
        if let Some(snap) = &ctx.snapshot {
            let data = self.state.read();
            snap.save("acm", &*data).map_err(persistence_error)?;
        }
        Ok(())
    }

    fn router(&self, _ctx: ServiceContext) -> Router {
        Router::new()
    }
}

#[async_trait]
impl JsonProtocolService for Acm {
    fn target_prefix(&self) -> &'static str {
        TARGET_PREFIX
    }

    async fn dispatch(
        &self,
        _ctx: ServiceContext,
        action: &str,
        body: Bytes,
    ) -> Result<Value, AwsError> {
        let req: Value = if body.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&body)
                .map_err(|e| AwsError::new("InvalidRequest", e.to_string()))?
        };
        match action {
            "RequestCertificate" => self.request_certificate(&req),
            "DescribeCertificate" => self.describe_certificate(&req),
            "ListCertificates" => self.list_certificates(&req),
            "DeleteCertificate" => self.delete_certificate(&req),
            "GetCertificate" => self.get_certificate(&req),
            "ImportCertificate" => self.import_certificate(&req),
            other => Err(AwsError::unsupported(format!("ACM::{other}"))),
        }
    }
}

impl Acm {
    fn request_certificate(&self, req: &Value) -> Result<Value, AwsError> {
        let domain = req
            .get("DomainName")
            .and_then(Value::as_str)
            .ok_or_else(|| AwsError::new("ValidationException", "DomainName required"))?
            .to_string();
        let sans: Vec<String> = req
            .get("SubjectAlternativeNames")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let arn = cert_arn();
        let now = chrono::Utc::now();
        let cert = Certificate {
            arn: arn.clone(),
            domain_name: domain,
            subject_alternative_names: sans,
            // kuroko marks every requested cert as ISSUED immediately so that
            // CI pipelines don't have to poll for validation.
            status: "ISSUED".into(),
            type_: "AMAZON_ISSUED".into(),
            issued_at: now,
            not_after: now + chrono::Duration::days(365),
            serial: short_id(),
            certificate_pem: None,
            private_key_pem: None,
        };
        self.state.write().certificates.insert(arn.clone(), cert);
        Ok(json!({ "CertificateArn": arn }))
    }

    fn describe_certificate(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = required(req, "CertificateArn")?;
        let s = self.state.read();
        let cert = s.certificates.get(&arn).ok_or_else(|| not_found(&arn))?;
        Ok(json!({ "Certificate": cert_json(cert) }))
    }

    fn list_certificates(&self, _req: &Value) -> Result<Value, AwsError> {
        let s = self.state.read();
        let summaries: Vec<_> = s
            .certificates
            .values()
            .map(|c| {
                json!({
                    "CertificateArn": c.arn,
                    "DomainName": c.domain_name,
                    "Status": c.status,
                    "Type": c.type_,
                    "CreatedAt": c.issued_at.timestamp(),
                    "NotAfter": c.not_after.timestamp(),
                })
            })
            .collect();
        Ok(json!({ "CertificateSummaryList": summaries }))
    }

    fn delete_certificate(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = required(req, "CertificateArn")?;
        self.state
            .write()
            .certificates
            .remove(&arn)
            .ok_or_else(|| not_found(&arn))?;
        Ok(json!({}))
    }

    fn get_certificate(&self, req: &Value) -> Result<Value, AwsError> {
        let arn = required(req, "CertificateArn")?;
        let s = self.state.read();
        let cert = s.certificates.get(&arn).ok_or_else(|| not_found(&arn))?;
        Ok(json!({
            "Certificate": cert.certificate_pem.clone().unwrap_or_else(stub_pem),
            "CertificateChain": stub_pem(),
        }))
    }

    fn import_certificate(&self, req: &Value) -> Result<Value, AwsError> {
        // AWS SDKs base64-encode blob fields in JSON; decode before storing
        // so callers get back the original PEM bytes on GetCertificate.
        let certificate_pem = req
            .get("Certificate")
            .and_then(Value::as_str)
            .and_then(decode_blob)
            .ok_or_else(|| AwsError::new("ValidationException", "Certificate required"))?;
        let private_key_pem = req
            .get("PrivateKey")
            .and_then(Value::as_str)
            .and_then(decode_blob);
        let domain = extract_cn(&certificate_pem).unwrap_or_else(|| "imported.kuroko.local".into());
        let existing_arn = req
            .get("CertificateArn")
            .and_then(Value::as_str)
            .map(String::from);
        let now = chrono::Utc::now();
        let arn = existing_arn.unwrap_or_else(cert_arn);
        let cert = Certificate {
            arn: arn.clone(),
            domain_name: domain,
            subject_alternative_names: Vec::new(),
            status: "ISSUED".into(),
            type_: "IMPORTED".into(),
            issued_at: now,
            not_after: now + chrono::Duration::days(365),
            serial: short_id(),
            certificate_pem: Some(certificate_pem),
            private_key_pem,
        };
        self.state.write().certificates.insert(arn.clone(), cert);
        Ok(json!({ "CertificateArn": arn }))
    }
}

fn cert_arn() -> String {
    format!(
        "arn:aws:acm:{EMULATED_REGION}:{EMULATED_ACCOUNT_ID}:certificate/{id}",
        id = Uuid::new_v4()
    )
}

fn cert_json(c: &Certificate) -> Value {
    json!({
        "CertificateArn": c.arn,
        "DomainName": c.domain_name,
        "SubjectAlternativeNames": c.subject_alternative_names,
        "Status": c.status,
        "Type": c.type_,
        "Serial": c.serial,
        "Subject": format!("CN={}", c.domain_name),
        "Issuer": "Amazon",
        "CreatedAt": c.issued_at.timestamp(),
        "IssuedAt": c.issued_at.timestamp(),
        "NotBefore": c.issued_at.timestamp(),
        "NotAfter": c.not_after.timestamp(),
        "KeyAlgorithm": "RSA-2048",
        "SignatureAlgorithm": "SHA256WITHRSA",
    })
}

/// AWS SDK serializes Blob fields as base64 in JSON. Decode back to a UTF-8
/// string (PEM is ASCII). Returns None on either invalid base64 or non-UTF-8
/// bytes — the caller treats that as a missing field.
fn decode_blob(b64: &str) -> Option<String> {
    let bytes = BASE64.decode(b64.as_bytes()).ok()?;
    String::from_utf8(bytes).ok()
}

fn extract_cn(pem: &str) -> Option<String> {
    let cn_marker = "CN=";
    let start = pem.find(cn_marker)? + cn_marker.len();
    let tail = &pem[start..];
    let end = tail.find([',', '\n', '/', ' ']).unwrap_or(tail.len());
    Some(tail[..end].trim().to_string())
}

fn stub_pem() -> String {
    "-----BEGIN CERTIFICATE-----\nkuroko\n-----END CERTIFICATE-----\n".to_string()
}

fn required(req: &Value, key: &str) -> Result<String, AwsError> {
    req.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AwsError::new("ValidationException", format!("{key} required")))
}

fn not_found(arn: &str) -> AwsError {
    AwsError::new(
        "ResourceNotFoundException",
        format!("certificate '{arn}' not found"),
    )
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

pub fn register(registry: &Arc<crate::registry::Registry>) {
    registry.register_json(Arc::new(Acm::new()));
}
