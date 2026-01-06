use crate::error::AppError;
use crate::models::{ConsentSession, RejectRequest};
use crate::state::AppState;
use axum::{
    Form,
    extract::{Query, State},
    response::{Html, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};

/// Query parameters for consent page
#[derive(Debug, Deserialize)]
pub struct ConsentQuery {
    pub consent_challenge: String,
}

/// Form data for consent submission
#[derive(Debug, Deserialize)]
pub struct ConsentForm {
    pub consent_challenge: String,
    pub accept: Option<String>,
    // Note: We don't parse scopes from form - we use requested_scope from Hydra
    // This avoids issues with HTML form multiple checkbox handling
}

/// GET /consent - Display consent page
pub async fn show_consent(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConsentQuery>,
) -> Result<Html<String>, AppError> {
    info!(challenge = %query.consent_challenge, "Showing consent page");

    // Get consent request from Hydra
    let consent_request = state
        .hydra
        .get_consent_request(&query.consent_challenge)
        .await?;

    // If consent was already given (skip=true), accept immediately
    // But still include session data from login context
    if consent_request.skip {
        info!(
            subject = %consent_request.subject,
            "Skipping consent - already granted"
        );

        // Extract user data from login context even when skipping
        let context = consent_request.context.clone().unwrap_or_default();
        let email = context
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown@example.com");
        let email_verified = context
            .get("email_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let role = context
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("customer");
        let tenant_id = context.get("tenant_id").and_then(|v| v.as_str());

        let mut id_token_data = serde_json::json!({
            "sub": consent_request.subject,
            "email": email,
            "email_verified": email_verified,
            "role": role,
        });
        if let Some(tid) = tenant_id {
            id_token_data["tenant_id"] = serde_json::json!(tid);
        }

        let mut access_token_data = serde_json::json!({
            "role": role,
        });
        if let Some(tid) = tenant_id {
            access_token_data["tenant_id"] = serde_json::json!(tid);
        }

        let session = ConsentSession {
            access_token: Some(access_token_data),
            id_token: Some(id_token_data),
        };

        let redirect = state
            .hydra
            .accept_consent(
                &query.consent_challenge,
                consent_request.requested_scope,
                Some(session),
                true,
                3600,
            )
            .await?;

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

    // Build scope checkboxes
    let scope_items: Vec<String> = consent_request
        .requested_scope
        .iter()
        .map(|scope| {
            let description = get_scope_description(scope);
            format!(
                r#"<div class="scope-item">
                    <input type="checkbox" id="scope_{}" name="scopes" value="{}" checked>
                    <label for="scope_{}">
                        <strong>{}</strong>
                        <span>{}</span>
                    </label>
                </div>"#,
                scope, scope, scope, scope, description
            )
        })
        .collect();

    let client_name = consent_request
        .client
        .client_name
        .unwrap_or_else(|| consent_request.client.client_id.clone());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Authorize Application</title>
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
        .consent-container {{
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.2);
            width: 100%;
            max-width: 450px;
        }}
        h1 {{
            color: #333;
            margin-bottom: 10px;
            font-size: 24px;
        }}
        .client-info {{
            color: #666;
            margin-bottom: 20px;
            font-size: 14px;
        }}
        .client-info strong {{
            color: #667eea;
        }}
        .scopes-section {{
            margin-bottom: 25px;
        }}
        .scopes-section h2 {{
            font-size: 16px;
            color: #333;
            margin-bottom: 15px;
        }}
        .scope-item {{
            display: flex;
            align-items: flex-start;
            padding: 12px;
            background: #f8f9fa;
            border-radius: 6px;
            margin-bottom: 10px;
        }}
        .scope-item input[type="checkbox"] {{
            margin-top: 3px;
            margin-right: 12px;
        }}
        .scope-item label {{
            cursor: pointer;
        }}
        .scope-item label strong {{
            display: block;
            color: #333;
            margin-bottom: 4px;
        }}
        .scope-item label span {{
            color: #666;
            font-size: 13px;
        }}
        .button-group {{
            display: flex;
            gap: 12px;
        }}
        button {{
            flex: 1;
            padding: 14px;
            border: none;
            border-radius: 6px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        button[type="submit"][name="accept"] {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        button[type="submit"][name="accept"]:hover {{
            transform: translateY(-2px);
            box-shadow: 0 5px 20px rgba(102, 126, 234, 0.4);
        }}
        button.deny {{
            background: #f1f3f4;
            color: #666;
        }}
        button.deny:hover {{
            background: #e8eaed;
        }}
        .warning {{
            margin-top: 20px;
            padding: 12px;
            background: #fff3cd;
            border-radius: 6px;
            font-size: 13px;
            color: #856404;
        }}
    </style>
</head>
<body>
    <div class="consent-container">
        <h1>Authorize Application</h1>
        <p class="client-info">
            <strong>{}</strong> is requesting access to your account.
        </p>

        <form action="/consent" method="POST">
            <input type="hidden" name="consent_challenge" value="{}">

            <div class="scopes-section">
                <h2>This application would like to:</h2>
                {}
            </div>

            <div class="button-group">
                <button type="submit" id="deny-btn" name="accept" value="deny" class="deny">Deny</button>
                <button type="submit" id="allow-btn" name="accept" value="accept">Allow</button>
            </div>
        </form>

        <p class="warning">
            By clicking Allow, you authorize this application to access the selected information.
        </p>
    </div>
</body>
</html>"#,
        client_name,
        query.consent_challenge,
        scope_items.join("\n")
    );

    Ok(Html(html))
}

/// POST /consent - Process consent form submission
pub async fn handle_consent(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ConsentForm>,
) -> Result<Redirect, AppError> {
    info!(challenge = %form.consent_challenge, "Processing consent");

    // Get consent request to verify permissions
    let consent_request = state
        .hydra
        .get_consent_request(&form.consent_challenge)
        .await?;

    // Check if user accepted
    if form.accept.as_deref() != Some("accept") {
        warn!(
            subject = %consent_request.subject,
            "User denied consent"
        );

        let redirect = state
            .hydra
            .reject_consent(
                &form.consent_challenge,
                RejectRequest {
                    error: "access_denied".to_string(),
                    error_description: Some("The user denied the request".to_string()),
                    error_hint: None,
                    status_code: Some(403),
                },
            )
            .await?;

        return Ok(Redirect::to(&redirect.redirect_to));
    }

    // Extract user data from login context (passed during login accept)
    // This avoids needing to look up the user again, which is important for in-memory stores
    let context = consent_request.context.clone().unwrap_or_default();
    let email = context
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown@example.com");
    let email_verified = context
        .get("email_verified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let role = context
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("customer");
    let tenant_id = context.get("tenant_id").and_then(|v| v.as_str());

    // Build session data for ID token with role and tenant information
    let mut id_token_data = serde_json::json!({
        "sub": consent_request.subject,
        "email": email,
        "email_verified": email_verified,
        "role": role,
    });

    // Add tenant_id if present
    if let Some(tid) = tenant_id {
        id_token_data["tenant_id"] = serde_json::json!(tid);
    }

    // Build access token with role and tenant for API authorization
    let mut access_token_data = serde_json::json!({
        "role": role,
    });

    if let Some(tid) = tenant_id {
        access_token_data["tenant_id"] = serde_json::json!(tid);
    }

    let session = ConsentSession {
        access_token: Some(access_token_data),
        id_token: Some(id_token_data),
    };

    // Accept consent with all requested scopes
    // (User clicked Allow, so we grant all requested scopes)
    let granted_scopes = consent_request.requested_scope.clone();

    info!(
        subject = %consent_request.subject,
        scopes = ?granted_scopes,
        "Accepting consent"
    );

    let redirect = state
        .hydra
        .accept_consent(
            &form.consent_challenge,
            granted_scopes,
            Some(session),
            true,
            3600,
        )
        .await?;

    Ok(Redirect::to(&redirect.redirect_to))
}

/// Get human-readable description for OAuth scopes
fn get_scope_description(scope: &str) -> &'static str {
    match scope {
        "openid" => "Verify your identity",
        "profile" => "Access your basic profile information",
        "email" => "Access your email address",
        "address" => "Access your address",
        "phone" => "Access your phone number",
        "offline_access" => "Maintain access when you're not using the app",
        "offline" => "Maintain access when you're not using the app",
        _ => "Access additional information",
    }
}
