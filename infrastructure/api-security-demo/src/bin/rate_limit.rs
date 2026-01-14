//! Chapter 5: Rate Limiting and Brute Force Protection Demonstration
//!
//! This example demonstrates:
//! - Vulnerable endpoint: No rate limiting, allows unlimited login attempts
//! - Secure endpoint: Rate limiting with IP-based and account-based lockout
//!
//! Run: cargo run --bin ch05-rate-limit
//! Test:
//!   # Try brute force on vulnerable endpoint (will succeed)
//!   for i in {1..20}; do
//!     curl -X POST http://localhost:8080/vulnerable/login \
//!       -H "Content-Type: application/json" \
//!       -d '{"email": "user@example.com", "password": "wrong"}' &
//!   done
//!
//!   # Try brute force on secure endpoint (will be rate limited)
//!   for i in {1..20}; do
//!     curl -X POST http://localhost:8080/login \
//!       -H "Content-Type: application/json" \
//!       -d '{"email": "user@example.com", "password": "wrong"}'
//!   done

use api_security_demo::{
    error::AppError,
    models::{LoginRequest, LoginResponse},
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    num::NonZeroU32,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Tracks login attempts per IP address
#[derive(Debug, Clone)]
struct LoginAttemptTracker {
    /// IP -> (attempt_count, first_attempt_time)
    ip_attempts: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    /// Email -> (attempt_count, first_attempt_time)
    account_attempts: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    /// Blocked IPs
    blocked_ips: Arc<RwLock<Vec<String>>>,
    /// Locked accounts
    locked_accounts: Arc<RwLock<Vec<String>>>,
}

impl LoginAttemptTracker {
    fn new() -> Self {
        Self {
            ip_attempts: Arc::new(RwLock::new(HashMap::new())),
            account_attempts: Arc::new(RwLock::new(HashMap::new())),
            blocked_ips: Arc::new(RwLock::new(Vec::new())),
            locked_accounts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn is_ip_blocked(&self, ip: &str) -> bool {
        self.blocked_ips.read().unwrap().contains(&ip.to_string())
    }

    fn is_account_locked(&self, email: &str) -> bool {
        self.locked_accounts
            .read()
            .unwrap()
            .contains(&email.to_string())
    }

    fn record_attempt(&self, ip: &str, email: &str) -> (u32, u32) {
        let window = Duration::from_secs(300); // 5 minute window
        let now = Instant::now();

        // Track IP attempts
        let ip_count = {
            let mut attempts = self.ip_attempts.write().unwrap();
            let entry = attempts.entry(ip.to_string()).or_insert((0, now));
            if now.duration_since(entry.1) > window {
                *entry = (1, now);
            } else {
                entry.0 += 1;
            }
            entry.0
        };

        // Track account attempts
        let account_count = {
            let mut attempts = self.account_attempts.write().unwrap();
            let entry = attempts.entry(email.to_string()).or_insert((0, now));
            if now.duration_since(entry.1) > window {
                *entry = (1, now);
            } else {
                entry.0 += 1;
            }
            entry.0
        };

        // Block IP after 10 attempts
        if ip_count >= 10 {
            let mut blocked = self.blocked_ips.write().unwrap();
            if !blocked.contains(&ip.to_string()) {
                blocked.push(ip.to_string());
                tracing::warn!(ip = ip, "IP blocked due to too many attempts");
            }
        }

        // Lock account after 5 attempts
        if account_count >= 5 {
            let mut locked = self.locked_accounts.write().unwrap();
            if !locked.contains(&email.to_string()) {
                locked.push(email.to_string());
                tracing::warn!(email = email, "Account locked due to too many attempts");
            }
        }

        (ip_count, account_count)
    }

    fn reset_on_success(&self, ip: &str, email: &str) {
        self.ip_attempts.write().unwrap().remove(ip);
        self.account_attempts.write().unwrap().remove(email);
    }
}

#[derive(Clone)]
struct AppState {
    tracker: LoginAttemptTracker,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ch05_rate_limit=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Global rate limiter: 10 requests per second
    let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_second(
        NonZeroU32::new(10).unwrap(),
    )));

    let state = AppState {
        tracker: LoginAttemptTracker::new(),
        rate_limiter,
    };

    let app = Router::new()
        // Vulnerable endpoint - No rate limiting
        .route("/vulnerable/login", post(vulnerable_login))
        // Secure endpoint - With rate limiting
        .route("/login", post(secure_login))
        // Status endpoint
        .route("/status", post(get_status))
        // Subtle vulnerabilities
        .route("/subtle/login/xff", post(subtle_xff_bypass))
        .route("/subtle/login/case", post(subtle_case_sensitivity))
        .route("/subtle/login/timing", post(subtle_timing_leak))
        .route("/subtle/login/race", post(subtle_race_condition))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    tracing::info!(
        "Chapter 5: Rate Limiting demonstration server running on http://127.0.0.1:8080"
    );
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  POST /vulnerable/login  - VULNERABLE: No rate limiting");
    tracing::info!("  POST /login             - SECURE: Rate limited with lockout");
    tracing::info!("  POST /status            - Check IP/account lockout status");
    tracing::info!("");
    tracing::info!("Test credentials:");
    tracing::info!("  email: user@example.com");
    tracing::info!("  password: password123");
    tracing::info!("");
    tracing::info!("Rate limits:");
    tracing::info!("  - 10 requests/second globally");
    tracing::info!("  - 10 failed attempts per IP -> IP blocked");
    tracing::info!("  - 5 failed attempts per account -> Account locked");
    tracing::info!("");
    tracing::info!("Subtle vulnerability endpoints:");
    tracing::info!("  POST /subtle/login/xff    - Trusts X-Forwarded-For header");
    tracing::info!("  POST /subtle/login/case   - Case-sensitive email comparison");
    tracing::info!("  POST /subtle/login/timing - Timing leak on account existence");
    tracing::info!("  POST /subtle/login/race   - Race condition on counter update");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// VULNERABLE: Login endpoint without rate limiting
///
/// An attacker can make unlimited login attempts to brute force passwords.
async fn vulnerable_login(Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>, AppError> {
    tracing::warn!(email = req.email, "VULNERABLE login - no rate limiting!");

    // Simulate authentication check
    if req.email == "user@example.com" && req.password == "password123" {
        Ok(Json(LoginResponse {
            access_token: "vulnerable-token-12345".to_string(),
            token_type: "Bearer".to_string(),
        }))
    } else {
        Err(AppError::Unauthorized)
    }
}

/// SECURE: Login endpoint with rate limiting and lockout
async fn secure_login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<RateLimitError>)> {
    let ip = addr.ip().to_string();

    // Check global rate limit
    if state.rate_limiter.check().is_err() {
        tracing::warn!("Global rate limit exceeded");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Rate limit exceeded".to_string(),
                retry_after_seconds: 1,
            }),
        ));
    }

    // Check if IP is blocked
    if state.tracker.is_ip_blocked(&ip) {
        tracing::warn!(ip = ip, "Blocked IP attempted login");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "IP address blocked due to too many failed attempts".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Check if account is locked
    if state.tracker.is_account_locked(&req.email) {
        tracing::warn!(email = req.email, "Locked account attempted login");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Account locked due to too many failed attempts".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Simulate authentication check
    if req.email == "user@example.com" && req.password == "password123" {
        // Reset counters on successful login
        state.tracker.reset_on_success(&ip, &req.email);

        tracing::info!(email = req.email, "Successful login");
        Ok(Json(LoginResponse {
            access_token: "secure-token-67890".to_string(),
            token_type: "Bearer".to_string(),
        }))
    } else {
        // Record failed attempt
        let (ip_count, account_count) = state.tracker.record_attempt(&ip, &req.email);

        tracing::info!(
            email = req.email,
            ip = ip,
            ip_attempts = ip_count,
            account_attempts = account_count,
            "Failed login attempt"
        );

        Err((
            StatusCode::UNAUTHORIZED,
            Json(RateLimitError {
                error: format!(
                    "Invalid credentials. {} IP attempts, {} account attempts remaining",
                    10 - ip_count.min(10),
                    5 - account_count.min(5)
                ),
                retry_after_seconds: 0,
            }),
        ))
    }
}

#[derive(Serialize)]
struct RateLimitError {
    error: String,
    retry_after_seconds: u32,
}

#[derive(Serialize)]
struct StatusResponse {
    ip_blocked: bool,
    account_locked: bool,
}

/// Check lockout status
async fn get_status(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<StatusRequest>,
) -> Json<StatusResponse> {
    let ip = addr.ip().to_string();
    Json(StatusResponse {
        ip_blocked: state.tracker.is_ip_blocked(&ip),
        account_locked: state.tracker.is_account_locked(&req.email),
    })
}

#[derive(serde::Deserialize)]
struct StatusRequest {
    email: String,
}

// ============ SUBTLE VULNERABILITIES ============

/// SUBTLE VULNERABILITY #1: Trusting X-Forwarded-For header
///
/// Developer thought: "We're behind a load balancer, so we need to use X-Forwarded-For"
/// Reality: Attackers can set arbitrary X-Forwarded-For values to bypass IP-based rate limiting
///
/// Attack: curl -H "X-Forwarded-For: 1.2.3.4" -X POST .../subtle/login/xff
async fn subtle_xff_bypass(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<RateLimitError>)> {
    // BUG: Trusting the first IP in X-Forwarded-For without validation
    // In reality, this should only trust XFF from known proxies
    let ip = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    tracing::info!(
        real_ip = addr.ip().to_string(),
        xff_ip = ip,
        "Using X-Forwarded-For for rate limiting (subtle vulnerability!)"
    );

    // Check global rate limit (still applied, but IP-based checks are bypassable)
    if state.rate_limiter.check().is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Rate limit exceeded".to_string(),
                retry_after_seconds: 1,
            }),
        ));
    }

    // BUG: Attacker can bypass by using different X-Forwarded-For values
    if state.tracker.is_ip_blocked(&ip) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "IP address blocked".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Simulate authentication check
    if req.email == "user@example.com" && req.password == "password123" {
        state.tracker.reset_on_success(&ip, &req.email);
        Ok(Json(LoginResponse {
            access_token: "token-via-xff".to_string(),
            token_type: "Bearer".to_string(),
        }))
    } else {
        let (ip_count, _account_count) = state.tracker.record_attempt(&ip, &req.email);

        tracing::warn!(
            spoofed_ip = ip,
            real_ip = addr.ip().to_string(),
            attempts = ip_count,
            "Failed login with potentially spoofed IP"
        );

        Err((
            StatusCode::UNAUTHORIZED,
            Json(RateLimitError {
                error: "Invalid credentials".to_string(),
                retry_after_seconds: 0,
            }),
        ))
    }
}

/// SUBTLE VULNERABILITY #2: Case-sensitive email handling
///
/// Developer thought: "Email addresses should be case-insensitive for login"
/// Reality: Rate limiting uses case-sensitive comparison, allowing bypasses
///
/// Attack: Try User@Example.com, USER@example.com, user@EXAMPLE.com...
async fn subtle_case_sensitivity(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<RateLimitError>)> {
    let ip = addr.ip().to_string();

    if state.rate_limiter.check().is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Rate limit exceeded".to_string(),
                retry_after_seconds: 1,
            }),
        ));
    }

    // BUG: Rate limiting tracks the EXACT email (case-sensitive)
    // But authentication might be case-insensitive!
    if state.tracker.is_account_locked(&req.email) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Account locked".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Authentication is CASE-INSENSITIVE
    // Attacker can use: User@Example.com, USER@EXAMPLE.COM, etc.
    // Each variation is tracked separately for rate limiting!
    let email_lower = req.email.to_lowercase();

    if email_lower == "user@example.com" && req.password == "password123" {
        state.tracker.reset_on_success(&ip, &req.email);
        Ok(Json(LoginResponse {
            access_token: "token-case-bypass".to_string(),
            token_type: "Bearer".to_string(),
        }))
    } else {
        // BUG: Recording attempt with ORIGINAL case
        // user@example.com and User@Example.com are tracked separately!
        let (_ip_count, account_count) = state.tracker.record_attempt(&ip, &req.email);

        tracing::warn!(
            email = req.email,
            normalized = email_lower,
            attempts = account_count,
            "Case-sensitive rate limiting (subtle vulnerability!)"
        );

        Err((
            StatusCode::UNAUTHORIZED,
            Json(RateLimitError {
                error: "Invalid credentials".to_string(),
                retry_after_seconds: 0,
            }),
        ))
    }
}

/// SUBTLE VULNERABILITY #3: Timing leak revealing account existence
///
/// Developer thought: "We check if account is locked before attempting auth"
/// Reality: Response timing differs based on whether account exists
///
/// Attack: Measure response times - locked accounts respond faster
async fn subtle_timing_leak(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<RateLimitError>)> {
    let ip = addr.ip().to_string();
    let start = std::time::Instant::now();

    // BUG: This check happens BEFORE the expensive authentication
    // An attacker can tell if an account exists by timing the response
    if state.tracker.is_account_locked(&req.email) {
        // Fast response! Account exists and is locked
        let elapsed = start.elapsed();
        tracing::info!(
            email = req.email,
            elapsed_us = elapsed.as_micros(),
            "Fast rejection - account locked (timing leak!)"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Account locked".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Simulate expensive authentication (password hashing)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Valid accounts get extra processing
    let valid_emails = ["user@example.com", "admin@example.com"];
    let account_exists = valid_emails.contains(&req.email.as_str());

    if account_exists {
        // Additional check for valid accounts
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        if req.password == "password123" {
            state.tracker.reset_on_success(&ip, &req.email);
            return Ok(Json(LoginResponse {
                access_token: "token-timing".to_string(),
                token_type: "Bearer".to_string(),
            }));
        }
    }

    // BUG: Different timing for existing vs non-existing accounts
    let elapsed = start.elapsed();
    tracing::info!(
        email = req.email,
        exists = account_exists,
        elapsed_ms = elapsed.as_millis(),
        "Response timing varies by account existence!"
    );

    state.tracker.record_attempt(&ip, &req.email);

    Err((
        StatusCode::UNAUTHORIZED,
        Json(RateLimitError {
            error: "Invalid credentials".to_string(),
            retry_after_seconds: 0,
        }),
    ))
}

/// SUBTLE VULNERABILITY #4: Race condition in counter update
///
/// Developer thought: "We check the count before incrementing"
/// Reality: TOCTOU race condition between check and increment
///
/// Attack: Send many concurrent requests before counter is updated
async fn subtle_race_condition(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<RateLimitError>)> {
    let ip = addr.ip().to_string();

    // BUG: Race condition between check and increment
    // Multiple concurrent requests can pass the check before any increment

    // Check current count (but don't lock yet!)
    let current_count = {
        let attempts = state.tracker.ip_attempts.read().unwrap();
        attempts.get(&ip).map(|(count, _)| *count).unwrap_or(0)
    };
    // Lock is released here!

    tracing::info!(
        ip = ip,
        current_count = current_count,
        "Checking rate limit (race condition window!)"
    );

    // Simulate some processing time between check and update
    // This widens the race window
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Now check if limit exceeded (using stale count!)
    if current_count >= 10 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "Rate limit exceeded".to_string(),
                retry_after_seconds: 300,
            }),
        ));
    }

    // Simulate authentication
    if req.email == "user@example.com" && req.password == "password123" {
        Ok(Json(LoginResponse {
            access_token: "token-race".to_string(),
            token_type: "Bearer".to_string(),
        }))
    } else {
        // Increment counter AFTER the check (race condition!)
        state.tracker.record_attempt(&ip, &req.email);

        // BUG: Multiple concurrent requests passed the check with current_count=0
        // Now they all increment, but the damage is done - they all got through!

        Err((
            StatusCode::UNAUTHORIZED,
            Json(RateLimitError {
                error: "Invalid credentials".to_string(),
                retry_after_seconds: 0,
            }),
        ))
    }
}
