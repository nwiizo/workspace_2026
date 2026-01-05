use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// Query parameters for OAuth2 callback
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// OAuth2 token response
#[derive(Debug, Deserialize)]
#[allow(unused)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    id_token: Option<String>,
    refresh_token: Option<String>,
}

/// GET /callback - Handle OAuth2 callback and redirect to dashboard
pub async fn handle_callback(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, AppError> {
    // Check for errors
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        let encoded_error = urlencoding::encode(&error);
        let encoded_desc = urlencoding::encode(&description);
        return Ok(Redirect::to(&format!(
            "/error?error={}&description={}",
            encoded_error, encoded_desc
        )));
    }

    // Get authorization code
    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing authorization code".to_string()))?;

    info!(code = %code, "Processing OAuth2 callback");

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let token_response = client
        .post("http://hydra:4444/oauth2/token")
        .basic_auth(
            "1008a0cc-705e-4516-8bdd-639e83cf9725",
            Some("demo-secret-12345"),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", "http://localhost:3000/callback"),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to exchange token: {}", e)))?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Token exchange failed: {}",
            error_text
        )));
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse token response: {}", e)))?;

    // Parse ID token to get user info
    let (email, role) = if let Some(id_token) = &tokens.id_token {
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() >= 2 {
            let payload = parts[1];
            let decoded = URL_SAFE_NO_PAD
                .decode(payload)
                .or_else(|_| {
                    let padded = format!("{}{}", payload, "=".repeat((4 - payload.len() % 4) % 4));
                    URL_SAFE_NO_PAD.decode(&padded)
                })
                .unwrap_or_default();

            if let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&decoded) {
                let email = claims
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("guest");
                let role = claims
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("customer");
                (email.to_string(), role.to_string())
            } else {
                ("guest".to_string(), "customer".to_string())
            }
        } else {
            ("guest".to_string(), "customer".to_string())
        }
    } else {
        ("guest".to_string(), "customer".to_string())
    };

    info!(email = %email, role = %role, "User authenticated successfully");

    // Redirect to dashboard with user info
    let encoded_email = urlencoding::encode(&email);
    let encoded_role = urlencoding::encode(&role);
    Ok(Redirect::to(&format!(
        "/dashboard?email={}&role={}",
        encoded_email, encoded_role
    )))
}

/// GET /error - Show error page
pub async fn show_error(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Html<String> {
    let error = params
        .get("error")
        .map(|s| s.as_str())
        .unwrap_or("Unknown error");
    let description = params.get("description").map(|s| s.as_str()).unwrap_or("");

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0;
        }}
        .container {{
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.2);
            max-width: 500px;
            text-align: center;
        }}
        h1 {{ color: #e74c3c; margin-bottom: 20px; }}
        p {{ color: #666; margin-bottom: 10px; }}
        a {{
            display: inline-block;
            margin-top: 20px;
            padding: 12px 30px;
            background: #667eea;
            color: white;
            text-decoration: none;
            border-radius: 6px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Authentication Error</h1>
        <p><strong>{}</strong></p>
        <p>{}</p>
        <a href="/">Back to Home</a>
    </div>
</body>
</html>"#,
        error, description
    ))
}
