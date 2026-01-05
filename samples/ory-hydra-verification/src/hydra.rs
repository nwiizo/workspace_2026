//! Hydra Admin API Client
//!
//! Ory Hydra Admin APIとの通信を担当

use crate::error::AppError;
use crate::models::*;

#[derive(Clone)]
pub struct HydraService {
    admin_url: String,
    client: reqwest::Client,
}

impl HydraService {
    pub fn new(admin_url: &str) -> Self {
        Self {
            admin_url: admin_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get Login Request
    pub async fn get_login_request(&self, challenge: &str) -> Result<LoginRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login?login_challenge={}",
            self.admin_url, challenge
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to get login request: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept Login Request
    pub async fn accept_login(
        &self,
        challenge: &str,
        subject: &str,
        remember: bool,
    ) -> Result<CompletedRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login/accept?login_challenge={}",
            self.admin_url, challenge
        );

        let body = AcceptLoginRequest {
            subject: subject.to_string(),
            remember,
            remember_for: 3600,
        };

        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to accept login: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Reject Login Request
    pub async fn reject_login(
        &self,
        challenge: &str,
        error: &str,
        error_description: &str,
    ) -> Result<CompletedRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/login/reject?login_challenge={}",
            self.admin_url, challenge
        );

        let body = RejectRequest {
            error: error.to_string(),
            error_description: error_description.to_string(),
        };

        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to reject login: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Get Consent Request
    pub async fn get_consent_request(&self, challenge: &str) -> Result<ConsentRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent?consent_challenge={}",
            self.admin_url, challenge
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to get consent request: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept Consent Request
    pub async fn accept_consent(
        &self,
        challenge: &str,
        grant_scope: Vec<String>,
        grant_audience: Vec<String>,
        session: Option<ConsentSession>,
    ) -> Result<CompletedRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/consent/accept?consent_challenge={}",
            self.admin_url, challenge
        );

        let body = AcceptConsentRequest {
            grant_scope,
            grant_access_token_audience: grant_audience,
            remember: true,
            remember_for: 3600,
            session,
        };

        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to accept consent: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Get Logout Request
    pub async fn get_logout_request(&self, challenge: &str) -> Result<LogoutRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/logout?logout_challenge={}",
            self.admin_url, challenge
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to get logout request: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }

    /// Accept Logout Request
    pub async fn accept_logout(&self, challenge: &str) -> Result<CompletedRequest, AppError> {
        let url = format!(
            "{}/admin/oauth2/auth/requests/logout/accept?logout_challenge={}",
            self.admin_url, challenge
        );

        let response = self
            .client
            .put(&url)
            .send()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::HydraError(format!(
                "Failed to accept logout: {} - {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::HydraError(e.to_string()))
    }
}
