//! HTTP Handlers for Login/Consent Provider
//!
//! Axumを使用したLogin/Consent Providerのエンドポイント実装

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::auth::AuthService;
use crate::error::AppError;
use crate::hydra::HydraService;
use crate::models::{ConsentSession, UserContext};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub hydra: HydraService,
}

// ===========================================
// Login Handlers
// ===========================================

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub login_challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub login_challenge: String,
    pub email: String,
    pub password: String,
}

/// GET /login - Display login form
pub async fn login_page(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
) -> Result<impl IntoResponse, AppError> {
    let challenge = &query.login_challenge;
    let login_request = state.hydra.get_login_request(challenge).await?;

    // skipフラグが立っている場合は既にセッションがある
    // Note: skip時はcontextが既に設定されているためNoneで良い
    if login_request.skip {
        let completed = state
            .hydra
            .accept_login(challenge, &login_request.subject, false, None)
            .await?;
        return Ok(Redirect::to(&completed.redirect_to).into_response());
    }

    // ログインフォームを表示
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Login</title>
    <style>
        body {{ font-family: sans-serif; max-width: 400px; margin: 50px auto; padding: 20px; }}
        form {{ display: flex; flex-direction: column; gap: 15px; }}
        label {{ display: flex; flex-direction: column; gap: 5px; }}
        input {{ padding: 8px; border: 1px solid #ccc; border-radius: 4px; }}
        button {{ padding: 10px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }}
        button:hover {{ background: #0056b3; }}
    </style>
</head>
<body>
    <h1>Login</h1>
    <form method="post" action="/login">
        <input type="hidden" name="login_challenge" value="{}" />
        <label>
            Email
            <input type="email" name="email" required />
        </label>
        <label>
            Password
            <input type="password" name="password" required />
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#,
        challenge
    );

    Ok(Html(html).into_response())
}

/// POST /login - Process login form submission
pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    // 認証処理
    let user = state.auth.authenticate(&form.email, &form.password).await?;

    // Best Practice: ユーザー情報をcontextに保存
    // Consent時にDBルックアップを回避できる
    let user_context = UserContext {
        email: user.email.clone(),
        role: "customer".to_string(), // 実際のアプリではユーザーのロールを取得
        tenant_id: None,              // マルチテナントSaaSの場合はユーザーのテナントIDを設定
    };

    // Hydraに認証成功を通知（contextにユーザー情報を含める）
    let completed =
        state
            .hydra
            .accept_login(
                &form.login_challenge,
                &user.id.to_string(),
                false,
                Some(serde_json::to_value(&user_context).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize context: {}", e))
                })?),
            )
            .await?;

    // Hydraが指示するURLにリダイレクト
    Ok(Redirect::to(&completed.redirect_to))
}

// ===========================================
// Consent Handlers
// ===========================================

#[derive(Debug, Deserialize)]
pub struct ConsentQuery {
    pub consent_challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct ConsentForm {
    pub consent_challenge: String,
    pub grant_scope: Option<String>,
}

/// GET /consent - Display consent form
pub async fn consent_page(
    State(state): State<AppState>,
    Query(query): Query<ConsentQuery>,
) -> Result<impl IntoResponse, AppError> {
    let challenge = &query.consent_challenge;
    let consent_request = state.hydra.get_consent_request(challenge).await?;

    // Best Practice: contextからユーザー情報を取得（DBルックアップ不要）
    let user_context: Option<UserContext> = consent_request
        .context
        .as_ref()
        .and_then(|ctx| serde_json::from_value(ctx.clone()).ok());

    let (user_email, user_role, user_tenant_id) = user_context
        .map(|ctx| (ctx.email, ctx.role, ctx.tenant_id))
        .unwrap_or_default();

    // skipフラグが立っている場合は自動承認
    if consent_request.skip {
        let mut id_token_claims = serde_json::json!({
            "email": user_email,
            "role": user_role,
        });
        // マルチテナントSaaSの場合はtenant_idを含める
        if let Some(ref tenant_id) = user_tenant_id {
            id_token_claims["tenant_id"] = serde_json::json!(tenant_id);
        }
        let session = ConsentSession {
            id_token: id_token_claims,
        };

        let completed = state
            .hydra
            .accept_consent(
                challenge,
                consent_request.requested_scope.unwrap_or_default(),
                consent_request
                    .requested_access_token_audience
                    .unwrap_or_default(),
                Some(session),
            )
            .await?;

        return Ok(Redirect::to(&completed.redirect_to).into_response());
    }

    let client_name = consent_request
        .client
        .as_ref()
        .and_then(|c| c.client_name.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("Unknown Application");

    let scopes = consent_request.requested_scope.unwrap_or_default();
    let scope_html: String = scopes
        .iter()
        .map(|s| {
            format!(
                r#"<li><input type="checkbox" name="grant_scope" value="{}" checked /> {}</li>"#,
                s, s
            )
        })
        .collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorize Application</title>
    <style>
        body {{ font-family: sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; }}
        form {{ display: flex; flex-direction: column; gap: 15px; }}
        ul {{ list-style: none; padding: 0; }}
        li {{ padding: 5px 0; }}
        .buttons {{ display: flex; gap: 10px; }}
        button {{ padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }}
        .allow {{ background: #28a745; color: white; }}
        .deny {{ background: #dc3545; color: white; }}
    </style>
</head>
<body>
    <h1>Authorize Application</h1>
    <p><strong>{}</strong> is requesting access to your account.</p>
    <form method="post" action="/consent">
        <input type="hidden" name="consent_challenge" value="{}" />
        <h3>Requested Permissions:</h3>
        <ul>{}</ul>
        <div class="buttons">
            <button type="submit" name="action" value="allow" class="allow">Allow</button>
            <button type="submit" name="action" value="deny" class="deny">Deny</button>
        </div>
    </form>
</body>
</html>"#,
        client_name, challenge, scope_html
    );

    Ok(Html(html).into_response())
}

/// POST /consent - Process consent form submission
pub async fn consent_submit(
    State(state): State<AppState>,
    Form(form): Form<ConsentForm>,
) -> Result<Redirect, AppError> {
    let challenge = &form.consent_challenge;
    let consent_request = state.hydra.get_consent_request(challenge).await?;

    // Best Practice: contextからユーザー情報を取得（DBルックアップ不要）
    let user_context: Option<UserContext> = consent_request
        .context
        .as_ref()
        .and_then(|ctx| serde_json::from_value(ctx.clone()).ok());

    let (user_email, user_role, user_tenant_id) = user_context
        .map(|ctx| (ctx.email, ctx.role, ctx.tenant_id))
        .unwrap_or_default();

    let grant_scope = consent_request.requested_scope.unwrap_or_default();
    let grant_audience = consent_request
        .requested_access_token_audience
        .unwrap_or_default();

    // IDトークンにカスタムクレームを追加
    let mut id_token_claims = serde_json::json!({
        "email": user_email,
        "role": user_role,
    });
    // マルチテナントSaaSの場合はtenant_idを含める
    if let Some(ref tenant_id) = user_tenant_id {
        id_token_claims["tenant_id"] = serde_json::json!(tenant_id);
    }
    let session = ConsentSession {
        id_token: id_token_claims,
    };

    let completed = state
        .hydra
        .accept_consent(challenge, grant_scope, grant_audience, Some(session))
        .await?;

    Ok(Redirect::to(&completed.redirect_to))
}

// ===========================================
// Logout Handler
// ===========================================

#[derive(Debug, Deserialize)]
pub struct LogoutQuery {
    pub logout_challenge: String,
}

/// GET /logout - Process logout
pub async fn logout_handler(
    State(state): State<AppState>,
    Query(query): Query<LogoutQuery>,
) -> Result<Redirect, AppError> {
    let completed = state.hydra.accept_logout(&query.logout_challenge).await?;
    Ok(Redirect::to(&completed.redirect_to))
}

// ===========================================
// Health Check
// ===========================================

/// GET /health - Health check endpoint
pub async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
