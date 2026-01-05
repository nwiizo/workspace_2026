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
