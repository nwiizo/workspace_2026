use std::sync::Arc;

use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{Path, Query, Request, State},
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{ACCEPT, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, ORIGIN, SET_COOKIE},
    },
    response::{Redirect, Response},
};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, services::BffSession, state::AppState};

const SESSION_COOKIE: &str = "__Host-bff-session";
const CSRF_COOKIE: &str = "__Host-bff-csrf";
const CSRF_HEADER: &str = "x-csrf-token";
const COOKIE_MAX_AGE_SECONDS: i64 = 60 * 60 * 8;
const MAX_PROXY_BODY_BYTES: usize = 1024 * 1024 * 10;

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    pub user: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CsrfResponse {
    pub csrf_token: String,
}

pub async fn bff_login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AppError> {
    let authorization_url = state.bff.authorization_url(query.redirect).await?;
    Ok(Redirect::temporary(&authorization_url))
}

pub async fn bff_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        let redirect_to = format!(
            "{}/?error={}&description={}",
            state.bff.config().frontend_origin.trim_end_matches('/'),
            urlencoding::encode(&error),
            urlencoding::encode(&description)
        );
        return redirect_with_headers(&redirect_to, Vec::new());
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("Missing authorization code".to_string()))?;
    let oauth_state = query
        .state
        .ok_or_else(|| AppError::BadRequest("Missing OAuth state".to_string()))?;
    let (session_id, redirect_to, session) = state.bff.finish_login(&code, &oauth_state).await?;

    let frontend_redirect = format!(
        "{}{}",
        state.bff.config().frontend_origin.trim_end_matches('/'),
        redirect_to
    );
    redirect_with_headers(
        &frontend_redirect,
        vec![
            session_cookie(&session_id, COOKIE_MAX_AGE_SECONDS),
            csrf_cookie(&session.csrf_token, COOKIE_MAX_AGE_SECONDS),
        ],
    )
}

pub async fn bff_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let session_id = session_id_from_headers(&headers)?;
    let session = state.bff.session(&session_id).await?;
    let introspection = state.hydra.introspect_token(&session.access_token).await?;
    let ext = introspection.ext.as_ref();

    Ok(Json(SessionResponse {
        authenticated: true,
        user: serde_json::json!({
            "sub": introspection.sub.unwrap_or_else(|| session.user.sub.clone()),
            "email": ext
                .and_then(|value| value.get("email"))
                .and_then(|value| value.as_str())
                .or(session.user.email.as_deref()),
            "role": ext
                .and_then(|value| value.get("role"))
                .and_then(|value| value.as_str())
                .or(session.user.role.as_deref()),
            "tenant_id": ext
                .and_then(|value| value.get("tenant_id"))
                .and_then(|value| value.as_str())
                .or(session.user.tenant_id.as_deref()),
        }),
    }))
}

pub async fn bff_csrf(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<CsrfResponse>, AppError> {
    let session_id = session_id_from_headers(&headers)?;
    let session = state.bff.session(&session_id).await?;

    Ok(Json(CsrfResponse {
        csrf_token: session.csrf_token,
    }))
}

pub async fn bff_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(session_id) = cookie_value(&headers, SESSION_COOKIE) {
        state.bff.remove_session(&session_id).await;
    }

    let redirect_to = format!(
        "{}/",
        state.bff.config().frontend_origin.trim_end_matches('/')
    );
    redirect_with_headers(
        &redirect_to,
        vec![expired_cookie(SESSION_COOKIE), expired_cookie(CSRF_COOKIE)],
    )
}

pub async fn bff_proxy(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    request: Request,
) -> Result<Response, AppError> {
    let method = request.method().clone();
    let headers = request.headers().clone();
    validate_origin(&headers, &method, &state.bff.config().frontend_origin)?;

    let session_id = session_id_from_headers(&headers)?;
    let session = state.bff.session(&session_id).await?;
    validate_csrf(&headers, &method, &session)?;

    let query = request
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let upstream_url = format!("{}{}", state.bff.map_proxy_path(&path)?, query);
    let body = to_bytes(request.into_body(), MAX_PROXY_BODY_BYTES)
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid request body: {}", e)))?;

    let mut upstream = reqwest::Client::new()
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes())
                .map_err(|e| AppError::Internal(format!("Invalid HTTP method: {}", e)))?,
            upstream_url,
        )
        .bearer_auth(session.access_token);

    if let Some(content_type) = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
        upstream = upstream.header(CONTENT_TYPE.as_str(), content_type);
    }
    if let Some(accept) = headers.get(ACCEPT).and_then(|v| v.to_str().ok()) {
        upstream = upstream.header(ACCEPT.as_str(), accept);
    }
    if let Some(tenant_slug) = headers.get("x-tenant-slug").and_then(|v| v.to_str().ok()) {
        upstream = upstream.header("x-tenant-slug", tenant_slug);
    }

    let upstream_response = upstream.body(body).send().await?;
    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .map_err(|e| AppError::Internal(format!("Invalid upstream status: {}", e)))?;
    let content_type = upstream_response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let response_body = upstream_response.bytes().await?;

    let mut response = Response::builder().status(status);
    if let Some(content_type) = content_type {
        response = response.header(CONTENT_TYPE, content_type);
    }
    response = response.header(CACHE_CONTROL, "no-store");

    response
        .body(Body::from(response_body))
        .map_err(|e| AppError::Internal(format!("Failed to build proxy response: {}", e)))
}

fn session_id_from_headers(headers: &HeaderMap) -> Result<String, AppError> {
    cookie_value(headers, SESSION_COOKIE)
        .ok_or_else(|| AppError::AuthenticationFailed("Missing BFF session cookie".to_string()))
}

fn validate_origin(
    headers: &HeaderMap,
    method: &Method,
    allowed_origin: &str,
) -> Result<(), AppError> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }

    let origin = headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Forbidden("Missing Origin header".to_string()))?;

    if origin != allowed_origin {
        return Err(AppError::Forbidden("Origin is not allowed".to_string()));
    }

    Ok(())
}

fn validate_csrf(
    headers: &HeaderMap,
    method: &Method,
    session: &BffSession,
) -> Result<(), AppError> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }

    let header_token = headers
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Forbidden("Missing CSRF header".to_string()))?;
    let cookie_token = cookie_value(headers, CSRF_COOKIE)
        .ok_or_else(|| AppError::Forbidden("Missing CSRF cookie".to_string()))?;

    if header_token != session.csrf_token || cookie_token != session.csrf_token {
        return Err(AppError::Forbidden("Invalid CSRF token".to_string()));
    }

    Ok(())
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;

    cookie_header.split(';').find_map(|cookie| {
        let (cookie_name, cookie_value) = cookie.trim().split_once('=')?;
        (cookie_name == name).then(|| cookie_value.to_string())
    })
}

fn session_cookie(value: &str, max_age: i64) -> String {
    format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; Secure; SameSite=Strict",
        SESSION_COOKIE, value, max_age
    )
}

fn csrf_cookie(value: &str, max_age: i64) -> String {
    format!(
        "{}={}; Path=/; Max-Age={}; Secure; SameSite=Strict",
        CSRF_COOKIE, value, max_age
    )
}

fn expired_cookie(name: &str) -> String {
    format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Strict",
        name
    )
}

fn redirect_with_headers(location: &str, cookies: Vec<String>) -> Result<Response, AppError> {
    let mut response = Response::builder()
        .status(StatusCode::FOUND)
        .header(LOCATION, location)
        .body(Body::empty())
        .map_err(|e| AppError::Internal(format!("Failed to build redirect: {}", e)))?;

    for cookie in cookies {
        let value = HeaderValue::from_str(&cookie)
            .map_err(|e| AppError::Internal(format!("Invalid cookie value: {}", e)))?;
        response.headers_mut().append(SET_COOKIE, value);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cookie_uses_host_prefix_requirements() {
        let cookie = session_cookie("sid", 60);
        assert!(cookie.starts_with("__Host-bff-session=sid;"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("Domain="));
    }

    #[test]
    fn csrf_cookie_is_secure_but_readable_by_javascript() {
        let cookie = csrf_cookie("csrf", 60);
        assert!(cookie.starts_with("__Host-bff-csrf=csrf;"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("HttpOnly"));
        assert!(!cookie.contains("Domain="));
    }

    #[test]
    fn cookie_parser_finds_named_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("a=1; __Host-bff-session=sid; b=2"),
        );
        assert_eq!(
            cookie_value(&headers, SESSION_COOKIE),
            Some("sid".to_string())
        );
    }

    #[test]
    fn unsafe_method_requires_exact_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static("https://www.example.com"));
        assert!(validate_origin(&headers, &Method::POST, "https://www.example.com").is_ok());
        assert!(validate_origin(&headers, &Method::POST, "https://evil.example.com").is_err());
        assert!(
            validate_origin(&HeaderMap::new(), &Method::GET, "https://www.example.com").is_ok()
        );
    }
}
