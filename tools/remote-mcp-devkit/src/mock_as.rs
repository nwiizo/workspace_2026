use crate::pkce;
use axum::{
    Json, Router,
    extract::{FromRef, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct MockAsState {
    pub base_url: String,
    pub mcp_path: String,
    inner: Arc<RwLock<Inner>>,
}

#[derive(Default)]
struct Inner {
    clients: HashMap<String, RegisteredClient>,
    auth_codes: HashMap<String, AuthCode>,
    tokens: HashMap<String, AccessTokenRecord>,
}

#[derive(Debug, Clone)]
pub struct RegisteredClient {
    pub client_id: String,
    pub redirect_uris: Vec<String>,
    pub client_id_metadata_document: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct AuthCode {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: Option<String>,
    resource: Option<String>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AccessTokenRecord {
    pub client_id: String,
    pub scope: Option<String>,
    pub resource: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl MockAsState {
    pub fn new(base_url: String, mcp_path: String) -> Self {
        Self {
            base_url,
            mcp_path,
            inner: Arc::new(RwLock::new(Inner::default())),
        }
    }

    pub fn validate_token(&self, token: &str) -> Option<AccessTokenRecord> {
        let guard = self.inner.read();
        let rec = guard.tokens.get(token)?.clone();
        if rec.expires_at < Utc::now() {
            return None;
        }
        Some(rec)
    }

    pub fn issue_token_direct(
        &self,
        client_id: &str,
        scope: Option<String>,
        resource: Option<String>,
    ) -> String {
        let token = format!("mldk_{}", uuid::Uuid::new_v4().simple());
        let rec = AccessTokenRecord {
            client_id: client_id.to_string(),
            scope,
            resource,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };
        self.inner.write().tokens.insert(token.clone(), rec);
        token
    }

    pub fn protected_resource_url(&self) -> String {
        format!(
            "{}/.well-known/oauth-protected-resource",
            self.base_url.trim_end_matches('/')
        )
    }

    pub fn resource_url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), self.mcp_path)
    }
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    MockAsState: FromRef<S>,
{
    Router::new()
        .route("/.well-known/oauth-protected-resource", get(prm_metadata))
        .route("/.well-known/oauth-authorization-server", get(as_metadata))
        .route("/oauth/register", post(register))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/revoke", post(revoke))
        // Introspection: mock AS internals exposed verbatim. This is intentional
        // — devkit runs on localhost in trusted dev contexts and full visibility
        // is the local-strength payoff. Never expose this surface to a public host.
        .route("/_devkit/state", get(devkit_state))
        .route("/_devkit/clients", post(devkit_seed_client))
        .route("/_devkit/tokens", post(devkit_seed_token))
}

async fn devkit_state(State(s): State<MockAsState>) -> Response {
    let guard = s.inner.read();
    let clients: Vec<serde_json::Value> = guard
        .clients
        .values()
        .map(|c| {
            serde_json::json!({
                "client_id": c.client_id,
                "redirect_uris": c.redirect_uris,
                "client_id_metadata_document": c.client_id_metadata_document,
                "created_at": c.created_at,
            })
        })
        .collect();
    let auth_codes: Vec<serde_json::Value> = guard
        .auth_codes
        .iter()
        .map(|(code, ac)| {
            serde_json::json!({
                "code": code,
                "client_id": ac.client_id,
                "redirect_uri": ac.redirect_uri,
                "scope": ac.scope,
                "resource": ac.resource,
                "code_challenge": ac.code_challenge,
                "code_challenge_method": ac.code_challenge_method,
                "expires_at": ac.expires_at,
            })
        })
        .collect();
    let tokens: Vec<serde_json::Value> = guard
        .tokens
        .iter()
        .map(|(tok, rec)| {
            serde_json::json!({
                "access_token": tok,
                "client_id": rec.client_id,
                "scope": rec.scope,
                "resource": rec.resource,
                "expires_at": rec.expires_at,
            })
        })
        .collect();
    let body = json!({
        "base_url": s.base_url,
        "mcp_path": s.mcp_path,
        "clients": clients,
        "auth_codes": auth_codes,
        "tokens": tokens,
        "now": Utc::now()
    });
    (StatusCode::OK, Json(body)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SeedClientRequest {
    pub client_id: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub client_id_metadata_document: Option<String>,
}

async fn devkit_seed_client(
    State(s): State<MockAsState>,
    Json(body): Json<SeedClientRequest>,
) -> Response {
    let client = RegisteredClient {
        client_id: body.client_id.clone(),
        redirect_uris: body.redirect_uris.clone(),
        client_id_metadata_document: body.client_id_metadata_document.clone(),
        created_at: Utc::now(),
    };
    s.inner
        .write()
        .clients
        .insert(body.client_id.clone(), client);
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": body.client_id,
            "redirect_uris": body.redirect_uris,
            "seeded": true
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct SeedTokenRequest {
    pub client_id: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
}

async fn devkit_seed_token(
    State(s): State<MockAsState>,
    Json(body): Json<SeedTokenRequest>,
) -> Response {
    let token = format!("mldk_seed_{}", uuid::Uuid::new_v4().simple());
    let rec = AccessTokenRecord {
        client_id: body.client_id.clone(),
        scope: body.scope.clone(),
        resource: body.resource.clone(),
        expires_at: Utc::now() + chrono::Duration::seconds(body.ttl_seconds.unwrap_or(3600)),
    };
    s.inner.write().tokens.insert(token.clone(), rec);
    (
        StatusCode::CREATED,
        Json(json!({
            "access_token": token,
            "token_type": "Bearer",
            "client_id": body.client_id,
            "expires_in": body.ttl_seconds.unwrap_or(3600)
        })),
    )
        .into_response()
}

pub async fn prm_handler(State(s): State<MockAsState>) -> Response {
    prm_metadata(State(s)).await
}

async fn prm_metadata(State(s): State<MockAsState>) -> Response {
    let body = json!({
        "resource": s.resource_url(),
        "authorization_servers": [s.base_url.trim_end_matches('/')],
        "bearer_methods_supported": ["header"],
        "resource_documentation": format!("{}/", s.base_url.trim_end_matches('/'))
    });
    (StatusCode::OK, Json(body)).into_response()
}

async fn as_metadata(State(s): State<MockAsState>) -> Response {
    let base = s.base_url.trim_end_matches('/');
    let body = json!({
        "issuer": base,
        "authorization_endpoint": format!("{}/oauth/authorize", base),
        "token_endpoint": format!("{}/oauth/token", base),
        "revocation_endpoint": format!("{}/oauth/revoke", base),
        "registration_endpoint": format!("{}/oauth/register", base),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_basic"],
        "scopes_supported": ["mcp:read", "mcp:write"],
        "client_id_metadata_document_supported": true
    });
    (StatusCode::OK, Json(body)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub client_id_metadata_document: Option<String>,
    #[serde(default)]
    pub client_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub client_id: String,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: &'static str,
}

async fn register(State(s): State<MockAsState>, Json(body): Json<RegisterRequest>) -> Response {
    let client_id = if let Some(doc) = body.client_id_metadata_document.as_deref() {
        doc.to_string()
    } else {
        format!("mldk-client-{}", uuid::Uuid::new_v4().simple())
    };
    let client = RegisteredClient {
        client_id: client_id.clone(),
        redirect_uris: body.redirect_uris.clone(),
        client_id_metadata_document: body.client_id_metadata_document.clone(),
        created_at: Utc::now(),
    };
    s.inner.write().clients.insert(client_id.clone(), client);
    (
        StatusCode::CREATED,
        Json(RegisterResponse {
            client_id,
            redirect_uris: body.redirect_uris,
            token_endpoint_auth_method: "none",
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub code_challenge: Option<String>,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
    /// If "true", render an HTML approval page (useful for Playwright).
    #[serde(default)]
    pub interactive: Option<String>,
}

async fn authorize(State(s): State<MockAsState>, Query(p): Query<AuthorizeParams>) -> Response {
    if p.response_type != "code" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_response_type",
            "only response_type=code supported",
        );
    }
    let Some(challenge) = p.code_challenge.as_ref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_challenge required",
        );
    };
    let method = p
        .code_challenge_method
        .clone()
        .unwrap_or_else(|| "plain".to_string());
    if method != "S256" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_challenge_method must be S256",
        );
    }

    let code = format!("code_{}", uuid::Uuid::new_v4().simple());
    let record = AuthCode {
        client_id: p.client_id.clone(),
        redirect_uri: p.redirect_uri.clone(),
        code_challenge: challenge.clone(),
        code_challenge_method: method,
        scope: p.scope.clone(),
        resource: p.resource.clone(),
        expires_at: Utc::now() + chrono::Duration::minutes(5),
    };
    s.inner.write().auth_codes.insert(code.clone(), record);

    let mut redirect = url::Url::parse(&p.redirect_uri)
        .unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
    redirect.query_pairs_mut().append_pair("code", &code);
    if let Some(state) = p.state.as_ref() {
        redirect.query_pairs_mut().append_pair("state", state);
    }

    if p.interactive.as_deref() == Some("true") {
        let html = format!(
            r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Mock IdP Consent</title></head>
<body style="font-family: system-ui; max-width:560px; margin:60px auto;">
<h1>Mock Identity Provider</h1>
<p>Client <code>{client}</code> is requesting access to <code>{scopes}</code>.</p>
<form method="GET" action="{redirect}">
  <button type="submit" id="approve" style="padding:8px 16px;">Approve</button>
</form>
<p style="color:#888">This is a mock IdP for local conformance testing only.</p>
</body></html>"#,
            client = html_escape(&p.client_id),
            scopes = html_escape(p.scope.as_deref().unwrap_or("(default)")),
            redirect = html_escape(redirect.as_str()),
        );
        return Html(html).into_response();
    }

    Redirect::to(redirect.as_str()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

async fn token(State(s): State<MockAsState>, body: String) -> Response {
    let parsed: TokenRequest = match serde_urlencoded::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                &format!("parse: {e}"),
            );
        }
    };

    match parsed.grant_type.as_str() {
        "authorization_code" => exchange_code(s, parsed).await,
        "refresh_token" => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "refresh_token not implemented in mock",
        ),
        other => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            &format!("grant_type={other} not supported"),
        ),
    }
}

async fn exchange_code(s: MockAsState, req: TokenRequest) -> Response {
    let Some(code) = req.code.as_ref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "code required");
    };
    let record = { s.inner.write().auth_codes.remove(code) };
    let Some(record) = record else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "code unknown or used",
        );
    };
    if record.expires_at < Utc::now() {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "code expired");
    }
    if let Some(cid) = req.client_id.as_deref()
        && cid != record.client_id
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client",
            "client_id mismatch",
        );
    }
    if let Some(ru) = req.redirect_uri.as_deref()
        && ru != record.redirect_uri
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "redirect_uri mismatch",
        );
    }
    let Some(verifier) = req.code_verifier.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_verifier required (PKCE)",
        );
    };
    if record.code_challenge_method != "S256"
        || !pkce::verify_s256(verifier, &record.code_challenge)
    {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "PKCE mismatch");
    }

    let access_token = s.issue_token_direct(
        &record.client_id,
        record.scope.clone(),
        record.resource.clone(),
    );

    let body = json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": 3600,
        "scope": record.scope.unwrap_or_default()
    });
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    (StatusCode::OK, headers, Json(body)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
}

async fn revoke(State(s): State<MockAsState>, body: String) -> Response {
    let parsed: RevokeRequest = match serde_urlencoded::from_str(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::OK.into_response(),
    };
    s.inner.write().tokens.remove(&parsed.token);
    StatusCode::OK.into_response()
}

fn oauth_error(status: StatusCode, code: &str, description: &str) -> Response {
    let body = json!({
        "error": code,
        "error_description": description
    });
    (status, Json(body)).into_response()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
