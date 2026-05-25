use crate::config::Config;
use crate::mock_as::MockAsState;
use axum::{
    Router,
    body::Body,
    extract::{FromRef, Request, State},
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::any,
};
use tracing::warn;

#[derive(Clone)]
pub struct AppState {
    pub mock_as: MockAsState,
    pub upstream_mcp: Option<String>,
    pub upstream_oauth: Option<String>,
    pub mcp_path: String,
    pub forwarded_proto: String,
    pub http_client: reqwest::Client,
}

impl FromRef<AppState> for MockAsState {
    fn from_ref(input: &AppState) -> Self {
        input.mock_as.clone()
    }
}

pub fn router(cfg: &Config, mock_as: MockAsState) -> Router {
    let upstream_mcp = cfg.upstreams.mcp.as_ref().map(|u| u.url.clone());
    let upstream_oauth = cfg.upstreams.oauth.as_ref().map(|u| u.url.clone());
    let http_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("build reqwest client");

    let state = AppState {
        mock_as,
        upstream_mcp,
        upstream_oauth: upstream_oauth.clone(),
        mcp_path: cfg.profile.mcp_path.clone(),
        forwarded_proto: cfg.profile.forwarded_proto.clone(),
        http_client,
    };

    let mcp_path = cfg.profile.mcp_path.clone();
    let wildcard_path = format!("{}/{{*rest}}", mcp_path.trim_end_matches('/'));

    // PRM always served locally (PRM is a property of the MCP resource).
    let mut router = Router::new().route(
        "/.well-known/oauth-protected-resource",
        any(crate::mock_as::prm_handler),
    );

    if upstream_oauth.is_some() {
        // Pass-through mode: forward OAuth surface to the real AS.
        router = router
            .route(
                "/.well-known/oauth-authorization-server",
                any(oauth_passthrough),
            )
            .route("/oauth/{*rest}", any(oauth_passthrough))
            .route("/oauth", any(oauth_passthrough));
    } else {
        // Mock AS mode: serve AS metadata + /oauth/* in-process.
        router = router.merge(crate::mock_as::router::<AppState>());
    }

    router
        .route(&mcp_path, any(mcp_handler))
        .route(&wildcard_path, any(mcp_handler))
        .fallback(fallback)
        .with_state(state)
}

async fn fallback() -> Response {
    (StatusCode::NOT_FOUND, "not found\n").into_response()
}

async fn oauth_passthrough(State(state): State<AppState>, req: Request) -> Response {
    let Some(upstream) = state.upstream_oauth.as_deref() else {
        return (StatusCode::BAD_GATEWAY, "oauth upstream not configured\n").into_response();
    };
    match forward(state.clone(), upstream, req).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "oauth upstream forward failed");
            (
                StatusCode::BAD_GATEWAY,
                format!("oauth upstream error: {e}\n"),
            )
                .into_response()
        }
    }
}

async fn mcp_handler(State(state): State<AppState>, req: Request) -> Response {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token = auth_header
        .as_deref()
        .and_then(|s| s.strip_prefix("Bearer "));

    let Some(token) = token else {
        return challenge_401(&state, "missing Authorization header");
    };

    if state.mock_as.validate_token(token).is_none() {
        return challenge_401(&state, "invalid or expired token");
    }

    let Some(upstream) = state.upstream_mcp.as_deref() else {
        // No upstream — return a minimal MCP-friendly echo so smoke tests can still
        // verify the post-auth path. This is sufficient for OAuth conformance checks.
        return authorized_echo(req).await;
    };

    match forward(state.clone(), upstream, req).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "upstream forward failed");
            (StatusCode::BAD_GATEWAY, format!("upstream error: {e}\n")).into_response()
        }
    }
}

fn challenge_401(state: &AppState, reason: &str) -> Response {
    let prm_url = state.mock_as.protected_resource_url();
    let www_auth = format!(
        r#"Bearer error="invalid_token", error_description="{reason}", resource_metadata="{prm_url}""#
    );
    let mut resp = (
        StatusCode::UNAUTHORIZED,
        format!("{{\"error\":\"invalid_token\",\"error_description\":\"{reason}\"}}\n"),
    )
        .into_response();
    resp.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_str(&www_auth).unwrap_or_else(|_| HeaderValue::from_static("Bearer")),
    );
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}

async fn authorized_echo(req: Request) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let body = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .ok();
    let body_str = body
        .as_ref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("");
    let payload = serde_json::json!({
        "authorized": true,
        "method": method.as_str(),
        "path": path,
        "echo": body_str,
        "note": "no MCP upstream configured; authorized echo response from remote-mcp-devkit"
    });
    (StatusCode::OK, axum::Json(payload)).into_response()
}

async fn forward(state: AppState, upstream: &str, req: Request) -> anyhow::Result<Response> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let upstream_url = build_upstream_url(upstream, &uri)?;

    let mut req_headers = req.headers().clone();
    // Derive forwarded host BEFORE we strip Host.
    let forwarded_host = req_headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            state
                .mock_as
                .base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .to_string()
        });
    req_headers.remove(header::HOST);
    req_headers.insert(
        "x-forwarded-proto",
        HeaderValue::from_str(&state.forwarded_proto)?,
    );
    if let Ok(v) = HeaderValue::from_str(&forwarded_host) {
        req_headers.insert("x-forwarded-host", v);
    }

    let body_bytes = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024).await?;

    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())?;
    let mut builder = state
        .http_client
        .request(reqwest_method, upstream_url)
        .body(body_bytes.to_vec());
    for (k, v) in req_headers.iter() {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes())
            && let Ok(val) = reqwest::header::HeaderValue::from_bytes(v.as_bytes())
        {
            builder = builder.header(name, val);
        }
    }

    let resp = builder.send().await?;
    let status = StatusCode::from_u16(resp.status().as_u16())?;
    let mut out_headers = axum::http::HeaderMap::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes())
            && let Ok(val) = axum::http::HeaderValue::from_bytes(v.as_bytes())
        {
            out_headers.insert(name, val);
        }
    }
    let bytes = resp.bytes().await?;
    let mut response = (status, Body::from(bytes)).into_response();
    *response.headers_mut() = out_headers;
    Ok(response)
}

fn build_upstream_url(upstream: &str, uri: &Uri) -> anyhow::Result<String> {
    let base = upstream.trim_end_matches('/');
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    Ok(format!("{base}{path}{query}"))
}
