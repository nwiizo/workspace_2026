//! Chapter 5: SSRF (Server-Side Request Forgery) Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: Fetches any URL provided by user
//! - Secure endpoint: Validates and restricts URLs to approved domains
//!
//! Run: cargo run --bin ch05-ssrf
//! Test:
//!   # Vulnerable: Can access internal services
//!   curl -X POST http://localhost:8080/vulnerable/fetch \
//!     -H "Content-Type: application/json" \
//!     -d '{"url": "http://localhost:8080/internal/secrets"}'
//!
//!   # Vulnerable: Can access cloud metadata
//!   curl -X POST http://localhost:8080/vulnerable/fetch \
//!     -H "Content-Type: application/json" \
//!     -d '{"url": "http://169.254.169.254/latest/meta-data/"}'
//!
//!   # Secure: Blocks internal URLs
//!   curl -X POST http://localhost:8080/fetch \
//!     -H "Content-Type: application/json" \
//!     -d '{"url": "http://localhost:8080/internal/secrets"}'

use api_security_demo::{error::AppError, models::FetchUrlRequest};
use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

/// List of allowed domains for outbound requests
const ALLOWED_DOMAINS: &[&str] = &["api.github.com", "api.stripe.com", "api.example.com"];

/// List of blocked IP ranges (internal networks)
const BLOCKED_IP_PREFIXES: &[&str] = &[
    "127.",    // Localhost
    "10.",     // Private Class A
    "172.16.", // Private Class B
    "172.17.", "172.18.", "172.19.", "172.20.", "172.21.", "172.22.", "172.23.", "172.24.",
    "172.25.", "172.26.", "172.27.", "172.28.", "172.29.", "172.30.", "172.31.",
    "192.168.", // Private Class C
    "169.254.", // Link-local / AWS metadata
    "0.",       // Invalid
];

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch05_ssrf=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        // Simulated internal endpoints (attack targets)
        .route("/internal/secrets", get(internal_secrets))
        .route("/internal/config", get(internal_config))
        // Vulnerable endpoint - SSRF vulnerability
        .route("/vulnerable/fetch", post(vulnerable_fetch))
        // Secure endpoint - URL validation
        .route("/fetch", post(secure_fetch))
        // Subtle vulnerabilities
        .route("/subtle/fetch/redirect", post(subtle_redirect_ssrf))
        .route("/subtle/fetch/dns-rebind", post(subtle_dns_rebinding))
        .route("/subtle/fetch/protocol", post(subtle_protocol_smuggling))
        .route("/subtle/fetch/parser-diff", post(subtle_parser_differential));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!("Chapter 5: SSRF demonstration server running on http://127.0.0.1:8080");
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  POST /vulnerable/fetch  - VULNERABLE: Fetches any URL");
    tracing::info!("  POST /fetch             - SECURE: Validates URLs");
    tracing::info!("");
    tracing::info!("Internal endpoints (attack targets):");
    tracing::info!("  GET /internal/secrets   - Simulated internal secrets");
    tracing::info!("  GET /internal/config    - Simulated internal config");
    tracing::info!("");
    tracing::info!("Allowed domains for secure endpoint:");
    for domain in ALLOWED_DOMAINS {
        tracing::info!("  - {}", domain);
    }
    tracing::info!("");
    tracing::info!("Subtle vulnerability endpoints:");
    tracing::info!("  POST /subtle/fetch/redirect     - Follows redirects to internal URLs");
    tracing::info!("  POST /subtle/fetch/dns-rebind   - Vulnerable to DNS rebinding");
    tracing::info!("  POST /subtle/fetch/protocol     - Protocol smuggling via URL encoding");
    tracing::info!("  POST /subtle/fetch/parser-diff  - URL parser differential");

    axum::serve(listener, app).await.unwrap();
}

/// Simulated internal secrets endpoint
async fn internal_secrets() -> Json<InternalSecrets> {
    Json(InternalSecrets {
        database_password: "super_secret_db_password_123".to_string(),
        api_key: "sk_live_supersecretapikey".to_string(),
        aws_access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
        aws_secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
    })
}

#[derive(Serialize)]
struct InternalSecrets {
    database_password: String,
    api_key: String,
    aws_access_key: String,
    aws_secret_key: String,
}

/// Simulated internal config endpoint
async fn internal_config() -> Json<InternalConfig> {
    Json(InternalConfig {
        environment: "production".to_string(),
        debug_mode: false,
        internal_services: vec![
            "http://database:5432".to_string(),
            "http://redis:6379".to_string(),
            "http://elasticsearch:9200".to_string(),
        ],
    })
}

#[derive(Serialize)]
struct InternalConfig {
    environment: String,
    debug_mode: bool,
    internal_services: Vec<String>,
}

/// VULNERABLE: Fetches any URL provided by the user
///
/// This demonstrates SSRF - an attacker can:
/// - Access internal services (localhost, internal IPs)
/// - Access cloud metadata endpoints (169.254.169.254)
/// - Port scan internal networks
/// - Exfiltrate data through the server
async fn vulnerable_fetch(
    Json(req): Json<FetchUrlRequest>,
) -> Result<Json<FetchResponse>, AppError> {
    tracing::warn!(
        url = req.url,
        "VULNERABLE: Fetching user-provided URL without validation!"
    );

    // VULNERABLE: No URL validation!
    let response = reqwest::get(&req.url)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch URL: {}", e)))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read response: {}", e)))?;

    tracing::warn!(
        url = req.url,
        status = status,
        body_length = body.len(),
        "VULNERABLE: Returned fetched content to user"
    );

    Ok(Json(FetchResponse {
        url: req.url,
        status,
        body,
        validated: false,
    }))
}

#[derive(Serialize)]
struct FetchResponse {
    url: String,
    status: u16,
    body: String,
    validated: bool,
}

/// SECURE: Validates URL before fetching
///
/// This demonstrates proper SSRF protection:
/// - Only allows HTTPS protocol
/// - Only allows specific approved domains
/// - Blocks private IP addresses
/// - Blocks cloud metadata endpoints
async fn secure_fetch(Json(req): Json<FetchUrlRequest>) -> Result<Json<FetchResponse>, AppError> {
    tracing::info!(url = req.url, "Validating URL before fetch");

    // Parse the URL
    let parsed_url =
        Url::parse(&req.url).map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    // Validate protocol (HTTPS only)
    if parsed_url.scheme() != "https" {
        tracing::warn!(
            url = req.url,
            scheme = parsed_url.scheme(),
            "Blocked: Only HTTPS allowed"
        );
        return Err(AppError::BadRequest(
            "Only HTTPS URLs are allowed".to_string(),
        ));
    }

    // Get the host
    let host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("URL must have a host".to_string()))?;

    // Check if host is an IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        let ip_str = ip.to_string();
        for prefix in BLOCKED_IP_PREFIXES {
            if ip_str.starts_with(prefix) {
                tracing::warn!(
                    url = req.url,
                    ip = ip_str,
                    "Blocked: Private/internal IP address"
                );
                return Err(AppError::BadRequest(
                    "Access to internal IP addresses is not allowed".to_string(),
                ));
            }
        }
    }

    // Check against allowlist
    if !ALLOWED_DOMAINS.contains(&host) {
        tracing::warn!(
            url = req.url,
            host = host,
            "Blocked: Domain not in allowlist"
        );
        return Err(AppError::BadRequest(format!(
            "Domain '{}' is not in the allowed list. Allowed domains: {:?}",
            host, ALLOWED_DOMAINS
        )));
    }

    tracing::info!(
        url = req.url,
        host = host,
        "URL validated, proceeding with fetch"
    );

    // Perform the fetch with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get(&req.url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch URL: {}", e)))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read response: {}", e)))?;

    Ok(Json(FetchResponse {
        url: req.url,
        status,
        body,
        validated: true,
    }))
}

// ============ SUBTLE VULNERABILITIES ============

/// Naive percent-decoding (intentionally simplistic for demonstration)
fn naive_percent_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

/// SUBTLE VULNERABILITY #1: Following redirects to internal URLs
///
/// Developer thought: "We validate the initial URL, so it's safe"
/// Reality: The validated URL could redirect to an internal URL
///
/// Attack:
///   1. Attacker controls https://evil.com which redirects to http://localhost:8080/internal/secrets
///   2. Initial validation passes (evil.com is... well, let's say it got on the allowlist)
///   3. HTTP client follows redirect to internal URL
async fn subtle_redirect_ssrf(
    Json(req): Json<FetchUrlRequest>,
) -> Result<Json<FetchResponse>, AppError> {
    let parsed_url = Url::parse(&req.url)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    // Check protocol
    if parsed_url.scheme() != "https" {
        return Err(AppError::BadRequest("Only HTTPS allowed".to_string()));
    }

    let host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("URL must have a host".to_string()))?;

    // Check against "extended" allowlist (includes partner domains)
    // BUG: One of these partner domains might redirect!
    let extended_allowlist = [
        "api.github.com",
        "api.stripe.com",
        "api.example.com",
        "webhook.partner.com",  // Partner can configure redirects!
        "cdn.trusted.com",       // CDN might have open redirects
    ];

    if !extended_allowlist.contains(&host) {
        return Err(AppError::BadRequest("Domain not allowed".to_string()));
    }

    tracing::info!(
        url = req.url,
        host = host,
        "URL passed validation, fetching with redirects enabled..."
    );

    // BUG: Following redirects without re-validating the destination!
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))  // Follows up to 10 redirects
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let response = client
        .get(&req.url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Fetch failed: {}", e)))?;

    // BUG: We don't check what URL we actually ended up at!
    let final_url = response.url().to_string();
    if final_url != req.url {
        tracing::warn!(
            original = req.url,
            final_url = final_url,
            "Request was redirected (subtle SSRF vulnerability!)"
        );
    }

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(FetchResponse {
        url: final_url,  // Returning the final URL reveals the redirect happened
        status,
        body,
        validated: true,  // But was the FINAL url validated? No!
    }))
}

/// SUBTLE VULNERABILITY #2: DNS Rebinding
///
/// Developer thought: "We resolve the IP and block internal ranges"
/// Reality: DNS can return different IPs on subsequent lookups
///
/// Attack:
///   1. Attacker's DNS returns 1.2.3.4 (public IP) on first lookup
///   2. Validation passes
///   3. Before the actual request, attacker's DNS changes to return 127.0.0.1
///   4. Request goes to localhost!
async fn subtle_dns_rebinding(
    Json(req): Json<FetchUrlRequest>,
) -> Result<Json<FetchResponse>, AppError> {
    let parsed_url = Url::parse(&req.url)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("URL must have a host".to_string()))?;

    // Resolve DNS and check if it's internal
    // BUG: This check happens BEFORE the actual request
    // DNS could return a different IP by the time we make the request!
    let resolved_ips: Vec<_> = tokio::net::lookup_host(format!("{}:80", host))
        .await
        .map_err(|e| AppError::BadRequest(format!("DNS lookup failed: {}", e)))?
        .collect();

    tracing::info!(host = host, ips = ?resolved_ips, "DNS resolution result");

    for addr in &resolved_ips {
        let ip = addr.ip();
        let ip_str = ip.to_string();

        for prefix in BLOCKED_IP_PREFIXES {
            if ip_str.starts_with(prefix) {
                tracing::warn!(host = host, ip = ip_str, "Blocked internal IP");
                return Err(AppError::BadRequest(
                    "Resolved to internal IP address".to_string(),
                ));
            }
        }
    }

    tracing::info!(host = host, "DNS check passed, making request...");

    // Time passes... DNS TTL could have expired, new lookup might return different IP
    // BUG: The actual request might go to a different IP!

    // Adding a simulated delay makes the attack more likely
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let response = reqwest::get(&req.url)
        .await
        .map_err(|e| AppError::Internal(format!("Fetch failed: {}", e)))?;

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(FetchResponse {
        url: req.url,
        status,
        body,
        validated: true,  // We validated the FIRST DNS response, not the actual one used
    }))
}

/// SUBTLE VULNERABILITY #3: Protocol Smuggling via URL encoding
///
/// Developer thought: "We check that scheme is 'https'"
/// Reality: URL parsing inconsistencies between validator and HTTP client
#[derive(Deserialize)]
struct EncodedUrlRequest {
    /// URL that might contain encoded characters
    url: String,
    /// Whether to decode before validation (dangerous!)
    decode_first: Option<bool>,
}

async fn subtle_protocol_smuggling(
    Json(req): Json<EncodedUrlRequest>,
) -> Result<Json<FetchResponse>, AppError> {
    // BUG: Developer added a "helpful" feature to decode URLs first
    // "Some clients send encoded URLs, let's be flexible!"
    let url_to_validate = if req.decode_first.unwrap_or(false) {
        // Naive percent-decoding implementation
        // Attack: http%73://localhost -> https://localhost after decoding
        // But what about: http://evil.com%23@localhost -> ?
        naive_percent_decode(&req.url)
    } else {
        req.url.clone()
    };

    let parsed_url = Url::parse(&url_to_validate)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    // Validate scheme
    if parsed_url.scheme() != "https" {
        return Err(AppError::BadRequest("Only HTTPS allowed".to_string()));
    }

    let host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("URL must have a host".to_string()))?;

    // Check against allowlist
    if !ALLOWED_DOMAINS.contains(&host) {
        return Err(AppError::BadRequest(format!(
            "Domain '{}' not allowed",
            host
        )));
    }

    tracing::info!(
        original = req.url,
        validated = url_to_validate,
        host = host,
        "URL validated (but was original or decoded URL used for request?)"
    );

    // BUG: Do we use the original URL or the decoded URL for the actual request?
    // This inconsistency can lead to bypasses
    let response = reqwest::get(&req.url)  // Using ORIGINAL, not validated URL!
        .await
        .map_err(|e| AppError::Internal(format!("Fetch failed: {}", e)))?;

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(FetchResponse {
        url: req.url,
        status,
        body,
        validated: true,
    }))
}

/// SUBTLE VULNERABILITY #4: URL Parser Differential
///
/// Developer thought: "We use a proper URL parser, it's secure"
/// Reality: Different parsers interpret URLs differently
///
/// Examples of parser differentials:
/// - http://localhost\@allowed.com -> Some parsers see localhost, others see allowed.com
/// - http://allowed.com#@localhost -> Fragment vs authority confusion
/// - http://allowed.com:password@localhost -> Userinfo parsing differences
async fn subtle_parser_differential(
    Json(req): Json<FetchUrlRequest>,
) -> Result<Json<FetchResponse>, AppError> {
    // Parse with the 'url' crate
    let parsed_url = Url::parse(&req.url)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("URL must have a host".to_string()))?;

    tracing::info!(
        original = req.url,
        parsed_host = host,
        parsed_path = parsed_url.path(),
        "URL parsed by 'url' crate"
    );

    // Check against allowlist
    if !ALLOWED_DOMAINS.contains(&host) {
        return Err(AppError::BadRequest(format!(
            "Domain '{}' not allowed. Allowed: {:?}",
            host, ALLOWED_DOMAINS
        )));
    }

    // BUG: We validated with 'url' crate, but reqwest might parse differently!
    // The URL spec is complex and different libraries have different interpretations
    //
    // Example attack URLs:
    // - "https://api.github.com@localhost/internal/secrets"
    //   -> url crate sees github.com as host
    //   -> but some HTTP clients might see localhost as actual host
    //
    // - "https://api.github.com#@localhost"
    //   -> Fragment parsing differences
    //
    // - "https://api.github.com\t.\tlocalhost"
    //   -> Whitespace handling differences

    tracing::warn!(
        "Making request with reqwest (might parse URL differently!)"
    );

    let response = reqwest::get(&req.url)
        .await
        .map_err(|e| AppError::Internal(format!("Fetch failed: {}", e)))?;

    // Log what URL reqwest actually used
    let actual_url = response.url();
    if actual_url.host_str() != parsed_url.host_str() {
        tracing::error!(
            validated_host = host,
            actual_host = ?actual_url.host_str(),
            "HOST MISMATCH! Parser differential vulnerability exploited!"
        );
    }

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(FetchResponse {
        url: req.url,
        status,
        body,
        validated: true,
    }))
}
