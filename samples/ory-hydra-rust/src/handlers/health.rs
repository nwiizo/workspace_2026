use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// GET /health - Health check endpoint
pub async fn health() -> impl IntoResponse {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(response))
}

/// GET / - Home page
pub async fn home() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ory Hydra Rust Auth Provider</title>
    <style>
        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.2);
            width: 100%;
            max-width: 600px;
        }
        h1 {
            color: #333;
            margin-bottom: 10px;
            font-size: 28px;
        }
        .subtitle {
            color: #666;
            margin-bottom: 30px;
        }
        h2 {
            color: #333;
            font-size: 18px;
            margin-top: 25px;
            margin-bottom: 15px;
        }
        .endpoints {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 6px;
        }
        .endpoint {
            display: flex;
            margin-bottom: 10px;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 14px;
        }
        .method {
            width: 60px;
            font-weight: bold;
        }
        .method.get { color: #28a745; }
        .method.post { color: #007bff; }
        .path {
            color: #333;
        }
        .description {
            color: #666;
            font-family: inherit;
            margin-left: 10px;
        }
        .footer {
            margin-top: 30px;
            color: #666;
            font-size: 13px;
            text-align: center;
        }
        a {
            color: #667eea;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Ory Hydra Rust Auth Provider</h1>
        <p class="subtitle">OAuth2/OIDC Login and Consent Provider</p>

        <h2>Hydra Provider Endpoints</h2>
        <div class="endpoints">
            <div class="endpoint">
                <span class="method get">GET</span>
                <span class="path">/login</span>
                <span class="description">- Login page</span>
            </div>
            <div class="endpoint">
                <span class="method post">POST</span>
                <span class="path">/login</span>
                <span class="description">- Process login</span>
            </div>
            <div class="endpoint">
                <span class="method get">GET</span>
                <span class="path">/consent</span>
                <span class="description">- Consent page</span>
            </div>
            <div class="endpoint">
                <span class="method post">POST</span>
                <span class="path">/consent</span>
                <span class="description">- Process consent</span>
            </div>
            <div class="endpoint">
                <span class="method get">GET</span>
                <span class="path">/logout</span>
                <span class="description">- Logout page</span>
            </div>
        </div>

        <h2>API Endpoints</h2>
        <div class="endpoints">
            <div class="endpoint">
                <span class="method post">POST</span>
                <span class="path">/api/auth/register</span>
                <span class="description">- Register user</span>
            </div>
            <div class="endpoint">
                <span class="method post">POST</span>
                <span class="path">/api/auth/login</span>
                <span class="description">- Login (get tokens)</span>
            </div>
            <div class="endpoint">
                <span class="method post">POST</span>
                <span class="path">/api/auth/refresh</span>
                <span class="description">- Refresh token</span>
            </div>
            <div class="endpoint">
                <span class="method get">GET</span>
                <span class="path">/health</span>
                <span class="description">- Health check</span>
            </div>
        </div>

        <p class="footer">
            Powered by <a href="https://www.ory.sh/hydra/">Ory Hydra</a> and Rust
        </p>
    </div>
</body>
</html>"#;

    axum::response::Html(html)
}
