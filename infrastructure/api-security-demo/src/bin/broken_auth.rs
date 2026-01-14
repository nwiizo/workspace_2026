//! Chapter 4: Broken Authentication Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: Validates JWT signature but not claims (exp, aud, iss)
//! - Secure endpoint: Properly validates all JWT claims
//!
//! Run: cargo run --bin ch04-broken-auth
//! Test:
//!   # Get a valid token
//!   curl http://localhost:8080/token/valid
//!
//!   # Get an expired token (vulnerable endpoint will accept it!)
//!   curl http://localhost:8080/token/expired
//!
//!   # Get a token with wrong audience
//!   curl http://localhost:8080/token/wrong-audience
//!
//!   # Test vulnerable endpoint (accepts all tokens)
//!   curl -H "Authorization: Bearer <expired_token>" http://localhost:8080/vulnerable/validate
//!
//!   # Test secure endpoint (rejects invalid tokens)
//!   curl -H "Authorization: Bearer <expired_token>" http://localhost:8080/validate

use api_security_demo::{
    auth::{JWT_AUDIENCE, JWT_ISSUER, JWT_SECRET},
    error::AppError,
    models::UserClaims,
};
use axum::{Json, Router, extract::Path, http::header::AUTHORIZATION, routing::get};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize)]
struct TokenValidationResponse {
    valid: bool,
    claims: Option<UserClaims>,
    validation_checks: ValidationChecks,
}

#[derive(Serialize)]
struct ValidationChecks {
    signature_valid: bool,
    expiration_checked: bool,
    audience_checked: bool,
    issuer_checked: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch04_broken_auth=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        // Token generation endpoints for testing
        .route("/token/{type}", get(generate_test_token))
        // Vulnerable validation endpoint
        .route("/vulnerable/validate", get(vulnerable_validate_token))
        // Secure validation endpoint
        .route("/validate", get(secure_validate_token))
        // Subtle vulnerabilities
        .route("/subtle/validate/alg-confusion", get(subtle_alg_confusion))
        .route("/subtle/validate/kid-injection", get(subtle_kid_injection))
        .route("/subtle/validate/jku-bypass", get(subtle_jku_bypass))
        .route("/subtle/validate/nbf-skip", get(subtle_nbf_skip));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!(
        "Chapter 4: Broken Authentication demonstration server running on http://127.0.0.1:8080"
    );
    tracing::info!("");
    tracing::info!("Token generation endpoints:");
    tracing::info!("  GET /token/valid           - Valid token (1 hour expiry)");
    tracing::info!("  GET /token/expired         - Expired token (1 hour ago)");
    tracing::info!("  GET /token/wrong-audience  - Token with wrong audience");
    tracing::info!("  GET /token/wrong-issuer    - Token with wrong issuer");
    tracing::info!("");
    tracing::info!("Validation endpoints:");
    tracing::info!("  GET /vulnerable/validate   - VULNERABLE: Only checks signature");
    tracing::info!("  GET /validate              - SECURE: Checks all claims");
    tracing::info!("");
    tracing::info!("Subtle vulnerability endpoints:");
    tracing::info!("  GET /subtle/validate/alg-confusion - Algorithm confusion attack");
    tracing::info!("  GET /subtle/validate/kid-injection - Key ID header injection");
    tracing::info!("  GET /subtle/validate/jku-bypass    - JKU claim bypass");
    tracing::info!("  GET /subtle/validate/nbf-skip      - Not-before claim skip");

    axum::serve(listener, app).await.unwrap();
}

/// Generate different types of test tokens
async fn generate_test_token(Path(token_type): Path<String>) -> Result<Json<TokenInfo>, AppError> {
    let (claims, description) = match token_type.as_str() {
        "valid" => {
            let claims = UserClaims {
                sub: "user123".to_string(),
                permissions: vec!["read".to_string()],
                exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
                iat: Utc::now().timestamp() as usize,
                aud: Some(JWT_AUDIENCE.to_string()),
                iss: Some(JWT_ISSUER.to_string()),
            };
            (claims, "Valid token - expires in 1 hour")
        }
        "expired" => {
            let claims = UserClaims {
                sub: "user123".to_string(),
                permissions: vec!["read".to_string()],
                exp: (Utc::now() - Duration::hours(1)).timestamp() as usize, // Expired!
                iat: (Utc::now() - Duration::hours(2)).timestamp() as usize,
                aud: Some(JWT_AUDIENCE.to_string()),
                iss: Some(JWT_ISSUER.to_string()),
            };
            (claims, "Expired token - expired 1 hour ago")
        }
        "wrong-audience" => {
            let claims = UserClaims {
                sub: "user123".to_string(),
                permissions: vec!["read".to_string()],
                exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
                iat: Utc::now().timestamp() as usize,
                aud: Some("https://wrong-audience.com".to_string()), // Wrong audience!
                iss: Some(JWT_ISSUER.to_string()),
            };
            (claims, "Token with wrong audience")
        }
        "wrong-issuer" => {
            let claims = UserClaims {
                sub: "user123".to_string(),
                permissions: vec!["read".to_string()],
                exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
                iat: Utc::now().timestamp() as usize,
                aud: Some(JWT_AUDIENCE.to_string()),
                iss: Some("https://malicious-issuer.com".to_string()), // Wrong issuer!
            };
            (claims, "Token with wrong issuer")
        }
        _ => {
            return Err(AppError::BadRequest(
                "Unknown token type. Use: valid, expired, wrong-audience, wrong-issuer".to_string(),
            ));
        }
    };

    let header = Header::new(Algorithm::HS256);
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )?;

    Ok(Json(TokenInfo {
        access_token: token,
        token_type: "Bearer".to_string(),
        description: description.to_string(),
        claims,
    }))
}

#[derive(Serialize)]
struct TokenInfo {
    access_token: String,
    token_type: String,
    description: String,
    claims: UserClaims,
}

/// VULNERABLE: Validates JWT signature but skips claim validation
///
/// This demonstrates broken authentication - the endpoint:
/// - Verifies the signature is valid ✓
/// - Does NOT check if token is expired ✗
/// - Does NOT check the audience claim ✗
/// - Does NOT check the issuer claim ✗
async fn vulnerable_validate_token(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    tracing::warn!("VULNERABLE validation - skipping claim checks!");

    // VULNERABLE: Disable all validation except signature
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Don't check expiration!
    validation.validate_aud = false; // Don't check audience!
    validation.required_spec_claims.clear(); // Don't require any claims!

    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            tracing::warn!(
                sub = token_data.claims.sub,
                exp = token_data.claims.exp,
                "VULNERABLE: Accepted token without proper validation"
            );

            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: false,
                    audience_checked: false,
                    issuer_checked: false,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Ok(Json(TokenValidationResponse {
                valid: false,
                claims: None,
                validation_checks: ValidationChecks {
                    signature_valid: false,
                    expiration_checked: false,
                    audience_checked: false,
                    issuer_checked: false,
                },
            }))
        }
    }
}

/// SECURE: Properly validates all JWT claims
///
/// This demonstrates proper JWT validation:
/// - Verifies the signature is valid ✓
/// - Checks if token is expired ✓
/// - Validates the audience claim ✓
/// - Validates the issuer claim ✓
async fn secure_validate_token(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    tracing::info!("Secure validation - checking all claims");

    // SECURE: Enable all validation
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.validate_exp = true;

    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            tracing::info!(
                sub = token_data.claims.sub,
                "Token validated successfully with all checks"
            );

            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: true,
                    audience_checked: true,
                    issuer_checked: true,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Err(AppError::Unauthorized)
        }
    }
}

// ============ SUBTLE VULNERABILITIES ============

/// SUBTLE VULNERABILITY #1: Algorithm Confusion
///
/// Developer thought: "We accept both HS256 and RS256 for flexibility"
/// Reality: Attacker can sign with HS256 using the public RSA key as secret
///
/// This is a classic JWT vulnerability where asymmetric/symmetric algorithm
/// confusion allows attackers to forge tokens.
async fn subtle_alg_confusion(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    // BUG: Accepting multiple algorithms without proper key management
    // If RS256 public key is known, attacker can use it as HS256 secret

    // First, peek at the token header to see what algorithm is claimed
    let header = jsonwebtoken::decode_header(token)
        .map_err(|_| AppError::Unauthorized)?;

    tracing::info!(
        algorithm = ?header.alg,
        "Token claims to use algorithm"
    );

    // BUG: Dynamically choosing validation based on token's claimed algorithm
    // Attacker can claim HS256 and sign with the RS256 public key as secret
    let mut validation = Validation::new(header.alg); // Using CLAIMED algorithm!
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_issuer(&[JWT_ISSUER]);

    // Using the same secret for both algorithms is the vulnerability
    // In a real RS256 setup, you'd have a public key that an attacker could
    // try to use as an HS256 secret
    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            tracing::warn!(
                sub = token_data.claims.sub,
                alg = ?header.alg,
                "Token validated with algorithm confusion vulnerability!"
            );

            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: true,
                    audience_checked: true,
                    issuer_checked: true,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Err(AppError::Unauthorized)
        }
    }
}

/// SUBTLE VULNERABILITY #2: Key ID (kid) Injection
///
/// Developer thought: "We use 'kid' header to select the right key"
/// Reality: Attacker can inject arbitrary values in kid to manipulate key selection
///
/// Common attacks:
/// - SQL injection via kid: {"kid": "key1' OR '1'='1"}
/// - Path traversal: {"kid": "../../../etc/passwd"}
/// - Null key: {"kid": "../../dev/null"}
#[derive(Deserialize)]
struct KeyStore {
    keys: std::collections::HashMap<String, String>,
}

async fn subtle_kid_injection(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let header = jsonwebtoken::decode_header(token)
        .map_err(|_| AppError::Unauthorized)?;

    // BUG: Using 'kid' header value without proper validation
    let kid = header.kid.clone().unwrap_or_else(|| "default".to_string());

    tracing::info!(kid = kid, "Looking up key by kid header");

    // Simulated key store lookup
    // In real code, this might be:
    // - SQL query: SELECT key FROM keys WHERE id = '$kid'
    // - File read: open("/keys/" + kid + ".pem")
    // - Directory traversal vulnerable!

    // BUG: No sanitization of kid value
    // Attack: kid = "../../../etc/passwd" or kid = "key1' UNION SELECT secret FROM admin --"
    if kid.contains("..") {
        tracing::error!(
            kid = kid,
            "PATH TRAVERSAL DETECTED in kid header!"
        );
        // In a real vulnerable implementation, this would have already done damage
    }

    if kid.contains("'") || kid.contains("\"") || kid.contains(";") {
        tracing::error!(
            kid = kid,
            "POTENTIAL SQL INJECTION in kid header!"
        );
    }

    // Proceed with validation using hardcoded key (demo only)
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_issuer(&[JWT_ISSUER]);

    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            tracing::warn!(
                sub = token_data.claims.sub,
                kid = kid,
                "Token validated with potentially malicious kid"
            );

            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: true,
                    audience_checked: true,
                    issuer_checked: true,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Err(AppError::Unauthorized)
        }
    }
}

/// SUBTLE VULNERABILITY #3: JKU (JWK Set URL) Bypass
///
/// Developer thought: "We fetch the public key from the URL in the token"
/// Reality: Attacker can specify their own key server URL
///
/// Attack: Set jku to attacker-controlled server that returns attacker's public key
async fn subtle_jku_bypass(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let header = jsonwebtoken::decode_header(token)
        .map_err(|_| AppError::Unauthorized)?;

    // BUG: Reading jku from token header without validation
    // In real vulnerable code, this URL would be fetched to get the signing key!
    if let Some(ref jku) = header.jku {
        tracing::warn!(
            jku = jku,
            "Token contains JKU header - potential bypass!"
        );

        // BUG: Weak allowlist check
        // Attacker can use: https://trusted.com.evil.com or https://evil.com/trusted.com
        let allowed_jku_prefix = "https://auth.example.com";

        if !jku.starts_with(allowed_jku_prefix) {
            // This check is easily bypassed!
            // - https://auth.example.com.attacker.com
            // - https://auth.example.com@attacker.com
            // - https://auth.example.com%2F@attacker.com

            tracing::info!("JKU not from trusted source");
            // In a truly vulnerable implementation, we'd fetch from the URL anyway
            // or the check would be bypassable
        }

        // Simulating what vulnerable code would do:
        // let key = fetch_jwk_from_url(jku).await?;  // Fetching attacker's key!
    }

    // For demo, we validate normally
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_issuer(&[JWT_ISSUER]);

    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: true,
                    audience_checked: true,
                    issuer_checked: true,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Err(AppError::Unauthorized)
        }
    }
}

/// SUBTLE VULNERABILITY #4: Not-Before (nbf) Claim Skip
///
/// Developer thought: "We check expiration, that's enough"
/// Reality: Tokens meant for future use can be used immediately
///
/// This allows using pre-generated tokens before their intended start time
async fn subtle_nbf_skip(
    headers: axum::http::HeaderMap,
) -> Result<Json<TokenValidationResponse>, AppError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    // BUG: Not validating the 'nbf' (not before) claim
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.validate_exp = true;
    validation.validate_nbf = false; // BUG: Not checking nbf!

    // This allows tokens like:
    // {
    //   "sub": "admin",
    //   "nbf": 1893456000,  // Year 2030
    //   "exp": 1893542400   // Year 2030
    // }
    // These "future tokens" might be pre-generated for scheduled access
    // but can be used immediately if nbf is not checked

    let result = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            // Check if this token has a future nbf
            let now = Utc::now().timestamp() as usize;
            if token_data.claims.iat > now + 60 {
                tracing::warn!(
                    iat = token_data.claims.iat,
                    now = now,
                    "Token appears to be issued in the future (subtle vulnerability!)"
                );
            }

            Ok(Json(TokenValidationResponse {
                valid: true,
                claims: Some(token_data.claims),
                validation_checks: ValidationChecks {
                    signature_valid: true,
                    expiration_checked: true,
                    audience_checked: true,
                    issuer_checked: true,
                },
            }))
        }
        Err(e) => {
            tracing::info!("Token validation failed: {}", e);
            Err(AppError::Unauthorized)
        }
    }
}
