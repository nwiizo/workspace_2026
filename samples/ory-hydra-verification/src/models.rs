//! Hydra API models
//!
//! Ory Hydra Admin APIのリクエスト/レスポンス型

use serde::{Deserialize, Serialize};

/// Login Request from Hydra
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub challenge: String,
    pub skip: bool,
    pub subject: String,
    pub client: Option<Client>,
    pub request_url: Option<String>,
    pub requested_scope: Option<Vec<String>>,
}

/// Consent Request from Hydra
#[derive(Debug, Deserialize)]
pub struct ConsentRequest {
    pub challenge: String,
    pub skip: bool,
    pub subject: String,
    pub client: Option<Client>,
    pub requested_scope: Option<Vec<String>>,
    pub requested_access_token_audience: Option<Vec<String>>,
    /// Context passed from login accept
    /// Best Practice: Contains user metadata to avoid DB lookup
    pub context: Option<serde_json::Value>,
}

/// OAuth2 Client information
#[derive(Debug, Deserialize)]
pub struct Client {
    pub client_id: Option<String>,
    pub client_name: Option<String>,
}

/// Accept Login Request body
#[derive(Debug, Serialize)]
pub struct AcceptLoginRequest {
    pub subject: String,
    pub remember: bool,
    pub remember_for: i64,
    /// Context to pass to consent request
    /// Best Practice: Store user metadata here to avoid DB lookup in consent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// User context passed from login to consent
/// Best Practice: Store all user metadata needed in consent to avoid DB lookup
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserContext {
    pub email: String,
    pub role: String,
    /// For multi-tenant SaaS applications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Accept Consent Request body
#[derive(Debug, Serialize)]
pub struct AcceptConsentRequest {
    pub grant_scope: Vec<String>,
    pub grant_access_token_audience: Vec<String>,
    pub remember: bool,
    pub remember_for: i64,
    pub session: Option<ConsentSession>,
}

/// Consent Session with ID token claims
#[derive(Debug, Serialize)]
pub struct ConsentSession {
    pub id_token: serde_json::Value,
}

/// Reject Request body
#[derive(Debug, Serialize)]
pub struct RejectRequest {
    pub error: String,
    pub error_description: String,
}

/// Completed Request response from Hydra
#[derive(Debug, Deserialize)]
pub struct CompletedRequest {
    pub redirect_to: String,
}

/// Logout Request from Hydra
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub subject: Option<String>,
    pub sid: Option<String>,
    pub request_url: Option<String>,
    pub rp_initiated: Option<bool>,
}
