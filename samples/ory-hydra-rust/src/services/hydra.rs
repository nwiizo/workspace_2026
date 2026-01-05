use crate::error::AppError;
use crate::models::{
    AcceptConsentRequest, AcceptLoginRequest, ConsentRequest, ConsentSession, LoginRequest,
    LogoutRequest, RedirectResponse, RejectRequest,
};
use reqwest::Client;
use tracing::instrument;

/// Client for Ory Hydra Admin API
#[derive(Clone)]
pub struct HydraClient {
    client: Client,
    admin_url: String,
}

impl HydraClient {
    /// Create a new Hydra client
    pub fn new(admin_url: String) -> Self {
        Self {
            client: Client::new(),
            admin_url,
        }
    }

    /// Get login request information
    #[instrument(skip(self))]
    pub async fn get_login_request(&self, challenge: &str) -> Result<LoginRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login?login_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept login request
    #[instrument(skip(self, body))]
    pub async fn accept_login(
        &self,
        challenge: &str,
        body: AcceptLoginRequest,
    ) -> Result<RedirectResponse, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login/accept?login_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Reject login request
    #[instrument(skip(self, body))]
    pub async fn reject_login(
        &self,
        challenge: &str,
        body: RejectRequest,
    ) -> Result<RedirectResponse, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login/reject?login_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Get consent request information
    #[instrument(skip(self))]
    pub async fn get_consent_request(&self, challenge: &str) -> Result<ConsentRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent?consent_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept consent request
    #[instrument(skip(self))]
    pub async fn accept_consent(
        &self,
        challenge: &str,
        grant_scope: Vec<String>,
        session: Option<ConsentSession>,
        remember: bool,
        remember_for: i64,
    ) -> Result<RedirectResponse, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent/accept?consent_challenge={}",
            self.admin_url, challenge
        );

        let body = AcceptConsentRequest {
            grant_scope,
            grant_access_token_audience: None,
            remember: Some(remember),
            remember_for: Some(remember_for),
            session,
        };

        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Reject consent request
    #[instrument(skip(self, body))]
    pub async fn reject_consent(
        &self,
        challenge: &str,
        body: RejectRequest,
    ) -> Result<RedirectResponse, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent/reject?consent_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Get logout request information
    #[instrument(skip(self))]
    pub async fn get_logout_request(&self, challenge: &str) -> Result<LogoutRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/logout?logout_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept logout request
    #[instrument(skip(self))]
    pub async fn accept_logout(&self, challenge: &str) -> Result<RedirectResponse, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/logout/accept?logout_challenge={}",
            self.admin_url, challenge
        );

        let resp = self
            .client
            .put(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        resp.json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Reject logout request
    #[instrument(skip(self))]
    pub async fn reject_logout(&self, challenge: &str) -> Result<(), AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/logout/reject?logout_challenge={}",
            self.admin_url, challenge
        );

        self.client
            .put(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        Ok(())
    }
}
