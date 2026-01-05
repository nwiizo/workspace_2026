use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// Query parameters for logout page
#[derive(Debug, Deserialize)]
pub struct LogoutQuery {
    pub logout_challenge: String,
}

/// GET /logout - Handle logout request
pub async fn handle_logout(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogoutQuery>,
) -> Result<Html<String>, AppError> {
    info!(challenge = %query.logout_challenge, "Processing logout");

    // Get logout request from Hydra
    let logout_request = state
        .hydra
        .get_logout_request(&query.logout_challenge)
        .await?;

    info!(
        subject = %logout_request.subject,
        rp_initiated = logout_request.rp_initiated,
        "Logout request received"
    );

    // For RP-initiated logout, show confirmation page
    // For other cases, accept immediately
    if logout_request.rp_initiated {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Logout</title>
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
        .logout-container {{
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.2);
            width: 100%;
            max-width: 400px;
            text-align: center;
        }}
        h1 {{
            color: #333;
            margin-bottom: 15px;
            font-size: 24px;
        }}
        p {{
            color: #666;
            margin-bottom: 25px;
        }}
        .button-group {{
            display: flex;
            gap: 12px;
            justify-content: center;
        }}
        a {{
            padding: 12px 24px;
            border-radius: 6px;
            font-size: 16px;
            font-weight: 600;
            text-decoration: none;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        a.confirm {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        a.confirm:hover {{
            transform: translateY(-2px);
            box-shadow: 0 5px 20px rgba(102, 126, 234, 0.4);
        }}
        a.cancel {{
            background: #f1f3f4;
            color: #666;
        }}
        a.cancel:hover {{
            background: #e8eaed;
        }}
    </style>
</head>
<body>
    <div class="logout-container">
        <h1>Sign Out</h1>
        <p>Are you sure you want to sign out?</p>
        <div class="button-group">
            <a href="/logout/cancel?logout_challenge={}" class="cancel">Cancel</a>
            <a href="/logout/confirm?logout_challenge={}" class="confirm">Sign Out</a>
        </div>
    </div>
</body>
</html>"#,
            query.logout_challenge, query.logout_challenge
        );

        Ok(Html(html))
    } else {
        // Accept logout immediately for non-RP-initiated
        let redirect = state.hydra.accept_logout(&query.logout_challenge).await?;

        Ok(Html(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0;url={}">
</head>
<body>Signing out...</body>
</html>"#,
            redirect.redirect_to
        )))
    }
}

/// GET /logout/confirm - Confirm logout
pub async fn confirm_logout(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogoutQuery>,
) -> Result<Redirect, AppError> {
    info!(challenge = %query.logout_challenge, "Confirming logout");

    let redirect = state.hydra.accept_logout(&query.logout_challenge).await?;

    Ok(Redirect::to(&redirect.redirect_to))
}

/// GET /logout/cancel - Cancel logout
pub async fn cancel_logout(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LogoutQuery>,
) -> Result<Redirect, AppError> {
    info!(challenge = %query.logout_challenge, "Cancelling logout");

    state.hydra.reject_logout(&query.logout_challenge).await?;

    // Redirect to home or back to the application
    Ok(Redirect::to("/"))
}
