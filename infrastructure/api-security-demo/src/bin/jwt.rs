//! Chapter 8: JWT Token Generation and Validation
//!
//! This example demonstrates:
//! - HS256 (HMAC-SHA256) symmetric signing
//! - RS256 (RSA-SHA256) asymmetric signing
//! - Proper token validation with all claims
//!
//! Run: cargo run --bin ch08-jwt
//! Test:
//!   # Generate HS256 token
//!   curl http://localhost:8080/token/hs256
//!
//!   # Generate RS256 token
//!   curl http://localhost:8080/token/rs256
//!
//!   # Validate token
//!   curl -H "Authorization: Bearer <token>" http://localhost:8080/validate/hs256

use axum::{Json, Router, extract::Path, http::header::AUTHORIZATION, routing::get};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// HS256 secret key (for demonstration - use env var in production)
const HS256_SECRET: &str = "your-256-bit-secret-key-here-must-be-long-enough";

/// RS256 keys (generated for demonstration)
/// In production, load from files or secret management
const RS256_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDKe/SDC8SXFjom
XVJEhUYBO/3zPTJV8UlZl9Rd3jvdFafYisA371mOqeCTpVDZ1KqY7tlvXz218rzx
bcrzKGx1Ee1vqvqZB4azO/ijDvx1L2jr3JX0E89hQEII6uTknquN0KeTRVnLAEPF
qA4dRCholniAPxafClWI52f3ooq+rBCuacbnGqB4c5/kAYatmyLqycDDQvEHM/fW
60F7Pf+Cq3/BXNPbbluauKgUcYAttb8rEtCqNKkCgLob6FnZG5KEdahKgHCcJKRU
WIRk8I3MB1pP4WPvy7nTY730tgpZ17IuxffOxx7GRknQ+Le6nBUBcmiW0RbLBKJ+
WEJr3fBHAgMBAAECggEAAytf30Hqu1oH4fwu9/nwEmqKez63etHn4y+gYtxs1R/s
Jym7gC9ObbsUqVWlR4Dv2gH5xFJaKG7FbLAFGmZFFateAJP4D+ImxnJyCLdeK0Vp
NTFC5aBTwYLHJdWWF0qxMRXq3EAce4l5sMPZ01/IPs7a0a3AdbqB2T6UtDMb9zJk
0PIoKwBM9Slubw+9y+R/RqgYs8WfKhLZnJ+Sw2Ceks9VlfgqFbmYJ7D177wPe7MJ
k+ZZgL+882xhZ2dimRrSo60PyEsKU6IgxsgFmD/lQMZ00BKShOVAKjXOLjaUbiAY
s3VbegyJEDpDTf+JmVjt1ntwZDPcdFSQRXP4oM960QKBgQD04PBEU8dgt4Pe7D3i
GzUjZis+hpsHwUwGQJJmNlsEKxN5UKO7bqudgxVLBBA6Z/uaiYCy/+jiyVprSLsu
703tl+WXbwI6zh5Ahat7j/+JZj4vxMScR7YyyFLOaAcrGuGzvx3p4JUEtdKRA09/
rpW0UwEe9VvmtnmNB0J8RkbG8QKBgQDTrh7vd0zFW3AKtmu19y+toE9PkaGQQjNj
JAFqm61ht9x4aQyZWMBxfc0hvxFnGycBAJXiKeyFF1Xut7Ht8Q9h1g2Er7L19YHb
qDY8i93NCZC0H4rEtih6ff4p0MmpylRo8KkxjofWPrROeFprRpRpEEgPSHh5286a
rkzyVKVatwKBgQCXJRvH4LooTT085C4SGF8FGXPJpQWdlMaa+VIjeptVCE19zLMy
5k1Q7G7BHaHymunmacahNWmGSWfg3kSC5LwB0YapoKAMsdpkUt0UaD3+jbgGffoo
x+6Ci7joo7cA+RekfWs2RyNTg/KTBSsVkSnf4nfHpwPxdGG0FW4JDMt00QKBgBW+
cRcQHia3uc6f5niOp6siKIN35iy3YCfy7uJQk4LSLCeCQvUNlNcToRqyUctRkrQb
p0nQHKefOgiHfhN/C6F1J3ZVxgBV87zojomxpFsHfIHEK7EBNS8/+fe5pr12Ny2A
ayDYD0QGtObKnh8e5OfV8FEBlL6Pwa1J8kWCRGoJAoGBAPLEVUsmFwt0dTZPwq7e
YuHqzQtP1EwQ/vrT0FW5wLTzkCENOGwjudNWRBjtgxfwwYsjXJIHrE+xI3SnPaze
LK+A77VMK5I5ZfutkRnLQ1mR8r+BBwJ9Vmu7roQRMrnW8Pt4ufVw3Byk5BZJU3EN
7nuhKXezif+MDm/xLvXCfrZW
-----END PRIVATE KEY-----"#;

const RS256_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAynv0gwvElxY6Jl1SRIVG
ATv98z0yVfFJWZfUXd473RWn2IrAN+9Zjqngk6VQ2dSqmO7Zb189tfK88W3K8yhs
dRHtb6r6mQeGszv4ow78dS9o69yV9BPPYUBCCOrk5J6rjdCnk0VZywBDxagOHUQo
aJZ4gD8WnwpViOdn96KKvqwQrmnG5xqgeHOf5AGGrZsi6snAw0LxBzP31utBez3/
gqt/wVzT225bmrioFHGALbW/KxLQqjSpAoC6G+hZ2RuShHWoSoBwnCSkVFiEZPCN
zAdaT+Fj78u502O99LYKWdeyLsX3zscexkZJ0Pi3upwVAXJoltEWywSiflhCa93w
RwIDAQAB
-----END PUBLIC KEY-----"#;

const ISSUER: &str = "secure-apis-rust";
const AUDIENCE: &str = "https://api.example.com";

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject (user ID)
    sub: String,
    /// Expiration time
    exp: usize,
    /// Issued at
    iat: usize,
    /// Issuer
    iss: String,
    /// Audience
    aud: String,
    /// Custom claims
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    algorithm: String,
    claims: Claims,
}

#[derive(Serialize)]
struct ValidationResponse {
    valid: bool,
    algorithm: String,
    claims: Option<Claims>,
    error: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch08_jwt=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        // Token generation
        .route("/token/{algorithm}", get(generate_token))
        .route("/token/{algorithm}/{user_id}", get(generate_token_for_user))
        // Token validation
        .route("/validate/{algorithm}", get(validate_token))
        // Info endpoints
        .route("/algorithms", get(list_algorithms))
        .route("/public-key", get(get_public_key));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!("Chapter 8: JWT demonstration server running on http://127.0.0.1:8080");
    tracing::info!("");
    tracing::info!("Token generation:");
    tracing::info!("  GET /token/hs256              - Generate HS256 token");
    tracing::info!("  GET /token/rs256              - Generate RS256 token");
    tracing::info!("  GET /token/hs256/{{user_id}}    - Generate token for specific user");
    tracing::info!("");
    tracing::info!("Token validation:");
    tracing::info!("  GET /validate/hs256           - Validate HS256 token");
    tracing::info!("  GET /validate/rs256           - Validate RS256 token");
    tracing::info!("");
    tracing::info!("Info:");
    tracing::info!("  GET /algorithms               - List supported algorithms");
    tracing::info!("  GET /public-key               - Get RS256 public key");

    axum::serve(listener, app).await.unwrap();
}

/// Generate a JWT token with specified algorithm
async fn generate_token(
    Path(algorithm): Path<String>,
) -> Result<Json<TokenResponse>, axum::http::StatusCode> {
    generate_token_for_user(Path((algorithm, "user123".to_string()))).await
}

/// Generate a JWT token for a specific user
async fn generate_token_for_user(
    Path((algorithm, user_id)): Path<(String, String)>,
) -> Result<Json<TokenResponse>, axum::http::StatusCode> {
    let now = Utc::now();
    let expires_in = 3600; // 1 hour

    let claims = Claims {
        sub: user_id,
        exp: (now + Duration::seconds(expires_in)).timestamp() as usize,
        iat: now.timestamp() as usize,
        iss: ISSUER.to_string(),
        aud: AUDIENCE.to_string(),
        permissions: vec!["read".to_string(), "write".to_string()],
        email: Some("user@example.com".to_string()),
    };

    let (token, alg_name) = match algorithm.to_lowercase().as_str() {
        "hs256" => {
            let header = Header::new(Algorithm::HS256);
            let token = encode(
                &header,
                &claims,
                &EncodingKey::from_secret(HS256_SECRET.as_bytes()),
            )
            .map_err(|e| {
                tracing::error!("Failed to encode HS256 token: {}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;
            (token, "HS256")
        }
        "rs256" => {
            let header = Header::new(Algorithm::RS256);
            let token = encode(
                &header,
                &claims,
                &EncodingKey::from_rsa_pem(RS256_PRIVATE_KEY.as_bytes()).map_err(|e| {
                    tracing::error!("Failed to load RS256 private key: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?,
            )
            .map_err(|e| {
                tracing::error!("Failed to encode RS256 token: {}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;
            (token, "RS256")
        }
        _ => {
            tracing::warn!("Unsupported algorithm requested: {}", algorithm);
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };

    tracing::info!(
        algorithm = alg_name,
        sub = claims.sub,
        "Generated JWT token"
    );

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        algorithm: alg_name.to_string(),
        claims,
    }))
}

/// Validate a JWT token
async fn validate_token(
    Path(algorithm): Path<String>,
    headers: axum::http::HeaderMap,
) -> Json<ValidationResponse> {
    let auth_header = match headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        Some(h) => h,
        None => {
            return Json(ValidationResponse {
                valid: false,
                algorithm: algorithm.to_uppercase(),
                claims: None,
                error: Some("Missing Authorization header".to_string()),
            });
        }
    };

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            return Json(ValidationResponse {
                valid: false,
                algorithm: algorithm.to_uppercase(),
                claims: None,
                error: Some("Invalid Authorization header format".to_string()),
            });
        }
    };

    let mut validation = Validation::default();
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[AUDIENCE]);
    validation.validate_exp = true;

    let result = match algorithm.to_lowercase().as_str() {
        "hs256" => {
            validation.algorithms = vec![Algorithm::HS256];
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(HS256_SECRET.as_bytes()),
                &validation,
            )
        }
        "rs256" => {
            validation.algorithms = vec![Algorithm::RS256];
            decode::<Claims>(
                token,
                &DecodingKey::from_rsa_pem(RS256_PUBLIC_KEY.as_bytes()).unwrap(),
                &validation,
            )
        }
        _ => {
            return Json(ValidationResponse {
                valid: false,
                algorithm: algorithm.to_uppercase(),
                claims: None,
                error: Some(format!("Unsupported algorithm: {}", algorithm)),
            });
        }
    };

    match result {
        Ok(token_data) => {
            tracing::info!(
                algorithm = algorithm.to_uppercase(),
                sub = token_data.claims.sub,
                "Token validated successfully"
            );
            Json(ValidationResponse {
                valid: true,
                algorithm: algorithm.to_uppercase(),
                claims: Some(token_data.claims),
                error: None,
            })
        }
        Err(e) => {
            tracing::warn!(
                algorithm = algorithm.to_uppercase(),
                error = %e,
                "Token validation failed"
            );
            Json(ValidationResponse {
                valid: false,
                algorithm: algorithm.to_uppercase(),
                claims: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// List supported algorithms
async fn list_algorithms() -> Json<AlgorithmsResponse> {
    Json(AlgorithmsResponse {
        supported: vec![
            AlgorithmInfo {
                name: "HS256".to_string(),
                description: "HMAC using SHA-256 (symmetric)".to_string(),
                key_type: "Shared secret".to_string(),
            },
            AlgorithmInfo {
                name: "RS256".to_string(),
                description: "RSA using SHA-256 (asymmetric)".to_string(),
                key_type: "RSA key pair".to_string(),
            },
        ],
        recommended: "RS256".to_string(),
        note: "RS256 is recommended for production as it allows public key distribution without exposing the signing key".to_string(),
    })
}

#[derive(Serialize)]
struct AlgorithmsResponse {
    supported: Vec<AlgorithmInfo>,
    recommended: String,
    note: String,
}

#[derive(Serialize)]
struct AlgorithmInfo {
    name: String,
    description: String,
    key_type: String,
}

/// Get the RS256 public key
async fn get_public_key() -> Json<PublicKeyResponse> {
    Json(PublicKeyResponse {
        algorithm: "RS256".to_string(),
        public_key: RS256_PUBLIC_KEY.to_string(),
        format: "PEM".to_string(),
        usage: "Use this key to verify RS256 tokens".to_string(),
    })
}

#[derive(Serialize)]
struct PublicKeyResponse {
    algorithm: String,
    public_key: String,
    format: String,
    usage: String,
}
