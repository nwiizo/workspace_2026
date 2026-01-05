use crate::error::AppError;
use crate::models::{AcceptLoginRequest, RejectRequest};
use crate::state::AppState;
use axum::{
    Form,
    extract::{Query, State},
    response::{Html, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Query parameters for login page
#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub login_challenge: String,
}

/// Form data for login submission
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub login_challenge: String,
}

/// GET /login - Display login page
pub async fn show_login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<Html<String>, AppError> {
    info!(challenge = %query.login_challenge, "Showing login page");

    // Get login request from Hydra
    let login_request = state
        .hydra
        .get_login_request(&query.login_challenge)
        .await?;

    // If the user has already authenticated (skip=true), accept immediately
    // But we still need to fetch user info from DB to pass to consent handler
    if login_request.skip {
        info!(
            subject = %login_request.subject,
            "Skipping login - user already authenticated"
        );

        // Fetch user info from DB using subject (user ID)
        let user_id = Uuid::parse_str(&login_request.subject)
            .map_err(|e| AppError::BadRequest(format!("Invalid subject: {}", e)))?;
        let user = state.auth.get_user_by_id(&user_id).await?;

        // Build context with user info for consent handler
        let mut context = serde_json::json!({
            "email": user.email,
            "email_verified": user.email_verified,
            "role": user.role.to_string(),
        });

        // Add tenant_id if present
        if let Some(tenant_id) = user.tenant_id {
            context["tenant_id"] = serde_json::json!(tenant_id.to_string());
        }

        info!(
            user_id = %user.id,
            email = %user.email,
            role = %user.role.to_string(),
            "Fetched user info for skipped login"
        );

        let redirect = state
            .hydra
            .accept_login(
                &query.login_challenge,
                AcceptLoginRequest {
                    subject: login_request.subject,
                    remember: Some(false),
                    remember_for: None,
                    acr: None,
                    context: Some(context),
                },
            )
            .await?;

        // Return a redirect response as HTML with meta refresh
        return Ok(Html(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0;url={}">
</head>
<body>Redirecting...</body>
</html>"#,
            redirect.redirect_to
        )));
    }

    // Render login form
    let client_name = login_request
        .client
        .client_name
        .unwrap_or_else(|| login_request.client.client_id.clone());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login</title>
    <style>
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .login-container {{
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.2);
            width: 100%;
            max-width: 400px;
        }}
        h1 {{
            color: #333;
            margin-bottom: 10px;
            font-size: 24px;
        }}
        .client-name {{
            color: #666;
            margin-bottom: 30px;
            font-size: 14px;
        }}
        .form-group {{
            margin-bottom: 20px;
        }}
        label {{
            display: block;
            margin-bottom: 8px;
            color: #555;
            font-weight: 500;
        }}
        input[type="email"],
        input[type="password"] {{
            width: 100%;
            padding: 12px 15px;
            border: 2px solid #e1e1e1;
            border-radius: 6px;
            font-size: 16px;
            transition: border-color 0.3s;
        }}
        input[type="email"]:focus,
        input[type="password"]:focus {{
            outline: none;
            border-color: #667eea;
        }}
        button {{
            width: 100%;
            padding: 14px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 6px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        button:hover {{
            transform: translateY(-2px);
            box-shadow: 0 5px 20px rgba(102, 126, 234, 0.4);
        }}
        .demo-info {{
            margin-top: 20px;
            padding: 15px;
            background: #f8f9fa;
            border-radius: 6px;
            font-size: 13px;
            color: #666;
        }}
        .demo-info strong {{
            color: #333;
        }}
    </style>
</head>
<body>
    <div class="login-container">
        <h1>Sign In</h1>
        <p class="client-name">Requested by: {}</p>
        <form action="/login" method="POST">
            <input type="hidden" name="login_challenge" value="{}">
            <div class="form-group">
                <label for="email">Email</label>
                <input type="email" id="email" name="email" required placeholder="Enter your email">
            </div>
            <div class="form-group">
                <label for="password">Password</label>
                <input type="password" id="password" name="password" required placeholder="Enter your password">
            </div>
            <button type="submit">Sign In</button>
        </form>
        <div class="demo-info">
            <strong>Demo credentials:</strong><br>
            Email: demo@example.com<br>
            Password: password123
        </div>
    </div>
</body>
</html>"#,
        client_name, query.login_challenge
    );

    Ok(Html(html))
}

/// POST /login - Process login form submission
pub async fn handle_login(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    info!(email = %form.email, challenge = %form.login_challenge, "Processing login");

    // Authenticate user with existing auth service
    match state.auth.authenticate(&form.email, &form.password).await {
        Ok(user) => {
            info!(user_id = %user.id, role = ?user.role, tenant_id = ?user.tenant_id, "Authentication successful");

            // Build context with role and tenant information for multi-tenant support
            let mut context = serde_json::json!({
                "email": user.email,
                "email_verified": user.email_verified,
                "role": user.role.to_string(),
            });

            // Add tenant_id if present
            if let Some(tenant_id) = user.tenant_id {
                context["tenant_id"] = serde_json::json!(tenant_id.to_string());
            }

            // Accept login with Hydra
            let redirect = state
                .hydra
                .accept_login(
                    &form.login_challenge,
                    AcceptLoginRequest {
                        subject: user.id.to_string(),
                        remember: Some(true),
                        remember_for: Some(3600),
                        acr: None,
                        context: Some(context),
                    },
                )
                .await?;

            Ok(Redirect::to(&redirect.redirect_to))
        }
        Err(e) => {
            warn!(email = %form.email, error = %e, "Authentication failed");

            // Reject login with Hydra
            let redirect = state
                .hydra
                .reject_login(
                    &form.login_challenge,
                    RejectRequest {
                        error: "access_denied".to_string(),
                        error_description: Some("Invalid email or password".to_string()),
                        error_hint: None,
                        status_code: Some(401),
                    },
                )
                .await?;

            Ok(Redirect::to(&redirect.redirect_to))
        }
    }
}
