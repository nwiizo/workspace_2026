use serde::{Deserialize, Serialize};

/// OAuth2 Client information from Hydra
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub scope: Option<String>,
}

/// Login request from Hydra
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LoginRequest {
    pub challenge: String,
    pub skip: bool,
    pub subject: String,
    pub client: OAuthClient,
    pub request_url: String,
    pub requested_scope: Vec<String>,
    pub requested_access_token_audience: Option<Vec<String>>,
    pub oidc_context: Option<OidcContext>,
}

/// OIDC context
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OidcContext {
    pub acr_values: Option<Vec<String>>,
    pub display: Option<String>,
    pub ui_locales: Option<Vec<String>>,
}

/// Request to accept login
#[derive(Debug, Serialize)]
pub struct AcceptLoginRequest {
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_for: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Request to reject login
#[derive(Debug, Serialize)]
pub struct RejectRequest {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
}

/// Consent request from Hydra
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ConsentRequest {
    pub challenge: String,
    pub skip: bool,
    pub subject: String,
    pub client: OAuthClient,
    pub request_url: String,
    pub requested_scope: Vec<String>,
    pub requested_access_token_audience: Option<Vec<String>>,
    pub oidc_context: Option<OidcContext>,
    pub login_challenge: Option<String>,
    pub login_session_id: Option<String>,
    /// Context data passed during login accept (contains email, role, tenant_id, etc.)
    pub context: Option<serde_json::Value>,
}

/// Request to accept consent
#[derive(Debug, Serialize)]
pub struct AcceptConsentRequest {
    pub grant_scope: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_access_token_audience: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_for: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<ConsentSession>,
}

/// Session data for consent
#[derive(Debug, Serialize)]
pub struct ConsentSession {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<serde_json::Value>,
}

/// Redirect response from Hydra
#[derive(Debug, Deserialize)]
pub struct RedirectResponse {
    pub redirect_to: String,
}

/// Logout request from Hydra
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LogoutRequest {
    pub challenge: String,
    pub subject: String,
    pub sid: Option<String>,
    pub request_url: String,
    pub rp_initiated: bool,
}
