use std::{collections::HashMap, sync::Arc};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::{RngCore, rngs::OsRng};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::error::AppError;

const SCOPE: &str = "openid profile email offline_access";
const SESSION_TTL_SECONDS: i64 = 60 * 60 * 8;

#[derive(Debug, Clone)]
pub struct BffConfig {
    pub hydra_public_url: String,
    pub authorization_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub frontend_origin: String,
    pub api_upstream_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BffUser {
    pub sub: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BffSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub user: BffUser,
    pub csrf_token: String,
}

#[derive(Debug, Clone)]
struct PendingOAuth {
    code_verifier: String,
    redirect_to: String,
}

#[derive(Clone)]
pub struct BffService {
    client: Client,
    config: BffConfig,
    sessions: Arc<RwLock<HashMap<String, BffSession>>>,
    pending_oauth: Arc<RwLock<HashMap<String, PendingOAuth>>>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    id_token: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: Option<String>,
    email: Option<String>,
    role: Option<String>,
    tenant_id: Option<String>,
}

impl BffService {
    pub fn new(config: BffConfig) -> Self {
        Self {
            client: Client::new(),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pending_oauth: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn config(&self) -> &BffConfig {
        &self.config
    }

    pub async fn authorization_url(&self, redirect_to: Option<String>) -> Result<String, AppError> {
        let state = random_token(32);
        let code_verifier = random_token(32);
        let code_challenge = pkce_challenge(&code_verifier);
        let redirect_to = sanitize_redirect_path(redirect_to.as_deref());

        self.pending_oauth.write().await.insert(
            state.clone(),
            PendingOAuth {
                code_verifier,
                redirect_to,
            },
        );

        let mut url = Url::parse(&format!(
            "{}/oauth2/auth",
            self.config.authorization_url.trim_end_matches('/')
        ))
        .map_err(|e| AppError::Internal(format!("Invalid Hydra public URL: {}", e)))?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("response_type", "code")
            .append_pair("scope", SCOPE)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("state", &state)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256");

        Ok(url.to_string())
    }

    pub async fn finish_login(
        &self,
        code: &str,
        state: &str,
    ) -> Result<(String, String, BffSession), AppError> {
        let pending = self
            .pending_oauth
            .write()
            .await
            .remove(state)
            .ok_or_else(|| AppError::BadRequest("Invalid OAuth state".to_string()))?;

        let tokens = self
            .exchange_authorization_code(code, &pending.code_verifier)
            .await?;
        let now = Utc::now();
        let user = tokens
            .id_token
            .as_deref()
            .and_then(user_from_id_token)
            .unwrap_or_else(|| BffUser {
                sub: "unknown".to_string(),
                email: None,
                role: None,
                tenant_id: None,
            });

        let session = BffSession {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: now + Duration::seconds(tokens.expires_in.unwrap_or(3600)),
            user,
            csrf_token: random_token(32),
        };

        let session_id = random_token(32);
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());

        Ok((session_id, pending.redirect_to, session))
    }

    pub async fn session(&self, session_id: &str) -> Result<BffSession, AppError> {
        self.refresh_if_needed(session_id).await
    }

    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    pub fn map_proxy_path(&self, path: &str) -> Result<String, AppError> {
        let normalized = path.trim_start_matches('/');
        if !is_allowed_proxy_path(normalized) {
            return Err(AppError::Forbidden(format!(
                "BFF proxy target is not allowlisted: {}",
                normalized
            )));
        }

        Ok(format!(
            "{}/{}",
            self.config.api_upstream_url.trim_end_matches('/'),
            normalized
        ))
    }

    async fn refresh_if_needed(&self, session_id: &str) -> Result<BffSession, AppError> {
        let current = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::AuthenticationFailed("Missing BFF session".to_string()))?;

        if current.expires_at > Utc::now() + Duration::seconds(60) {
            return Ok(current);
        }

        let refresh_token = current
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::AuthenticationFailed("Session expired".to_string()))?;
        let tokens = self.exchange_refresh_token(&refresh_token).await?;

        let refreshed = BffSession {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token.or(current.refresh_token),
            expires_at: Utc::now() + Duration::seconds(tokens.expires_in.unwrap_or(3600)),
            user: current.user,
            csrf_token: current.csrf_token,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), refreshed.clone());

        Ok(refreshed)
    }

    async fn exchange_authorization_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenResponse, AppError> {
        self.exchange_token(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .await
    }

    async fn exchange_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, AppError> {
        self.exchange_token(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .await
    }

    async fn exchange_token(&self, form: &[(&str, &str)]) -> Result<OAuthTokenResponse, AppError> {
        let url = format!(
            "{}/oauth2/token",
            self.config.hydra_public_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Token endpoint returned {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }
}

pub fn is_allowed_proxy_path(path: &str) -> bool {
    path == "api/v1/tenants"
        || path.starts_with("api/v1/tenants/")
        || path == "api/v1/tenant"
        || path.starts_with("api/v1/tenant/")
}

pub fn sanitize_redirect_path(redirect_to: Option<&str>) -> String {
    match redirect_to {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path.to_string(),
        _ => "/".to_string(),
    }
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn user_from_id_token(id_token: &str) -> Option<BffUser> {
    let payload = id_token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims = serde_json::from_slice::<IdTokenClaims>(&decoded).ok()?;

    Some(BffUser {
        sub: claims.sub.unwrap_or_else(|| "unknown".to_string()),
        email: claims.email,
        role: claims.role,
        tenant_id: claims.tenant_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_only_internal_api_paths() {
        assert!(is_allowed_proxy_path("api/v1/tenants"));
        assert!(is_allowed_proxy_path("api/v1/tenant/incidents"));
        assert!(!is_allowed_proxy_path("https://evil.example/token"));
        assert!(!is_allowed_proxy_path("api/v2/tenant/incidents"));
        assert!(!is_allowed_proxy_path("admin/oauth2/introspect"));
    }

    #[test]
    fn redirect_path_must_be_relative() {
        assert_eq!(sanitize_redirect_path(Some("/dashboard")), "/dashboard");
        assert_eq!(sanitize_redirect_path(Some("//evil.example")), "/");
        assert_eq!(sanitize_redirect_path(Some("https://evil.example")), "/");
        assert_eq!(sanitize_redirect_path(None), "/");
    }

    #[test]
    fn pkce_challenge_is_url_safe() {
        let challenge = pkce_challenge("verifier");
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert!(!challenge.contains('='));
    }
}
