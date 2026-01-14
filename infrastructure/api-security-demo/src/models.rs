//! Data models used across the API examples

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User claims extracted from JWT token
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Permissions/roles
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Expiration time
    pub exp: usize,
    /// Issued at
    #[serde(default)]
    pub iat: usize,
    /// Audience
    #[serde(default)]
    pub aud: Option<String>,
    /// Issuer
    #[serde(default)]
    pub iss: Option<String>,
}

/// Order model for BOLA demonstration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub user: String,
    pub product: String,
    pub quantity: i32,
}

/// Create order request
#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub product: String,
    pub quantity: i32,
}

/// Payment model for mass assignment demonstration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub created_at: String,
}

/// Safe payment creation request (whitelisted fields only)
#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub amount: f64,
    pub currency: String,
}

/// Unsafe payment creation - accepts any fields (vulnerable to mass assignment)
#[derive(Debug, Deserialize)]
pub struct UnsafePaymentRequest {
    pub amount: f64,
    pub currency: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

/// User model for authentication examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
}

/// User response (without sensitive data)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub role: String,
}

/// User response with excessive data exposure (vulnerable)
#[derive(Debug, Serialize)]
pub struct UserResponseVulnerable {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub internal_notes: String,
    pub created_at: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
}

/// URL fetch request for SSRF examples
#[derive(Debug, Deserialize)]
pub struct FetchUrlRequest {
    pub url: String,
}

impl Payment {
    pub fn new(amount: f64, currency: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            amount,
            currency,
            status: "pending".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ============================================
// API6: Unrestricted Access to Sensitive Business Flows
// ============================================

/// Coupon model for business flow demonstration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coupon {
    pub code: String,
    pub discount_percent: u32,
    pub max_uses: u32,
    pub current_uses: u32,
    pub expires_at: String,
    pub single_use_per_user: bool,
}

/// Coupon redemption request
#[derive(Debug, Deserialize)]
pub struct RedeemCouponRequest {
    pub coupon_code: String,
    pub order_total: f64,
}

/// Coupon redemption response
#[derive(Debug, Serialize)]
pub struct RedeemCouponResponse {
    pub success: bool,
    pub discount_amount: f64,
    pub final_total: f64,
    pub message: String,
}

/// Ticket/Event model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub event_name: String,
    pub price: f64,
    pub available_quantity: u32,
    pub max_per_user: u32,
}

/// Ticket purchase request
#[derive(Debug, Deserialize)]
pub struct PurchaseTicketRequest {
    pub ticket_id: String,
    pub quantity: u32,
}

/// Ticket purchase response
#[derive(Debug, Serialize)]
pub struct PurchaseTicketResponse {
    pub success: bool,
    pub purchase_id: String,
    pub quantity: u32,
    pub total_price: f64,
    pub message: String,
}

/// Referral model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Referral {
    pub id: String,
    pub referrer_id: String,
    pub referred_id: String,
    pub bonus_amount: f64,
    pub created_at: String,
}

/// Referral request
#[derive(Debug, Deserialize)]
pub struct ReferralRequest {
    pub referral_code: String,
}

/// Referral response
#[derive(Debug, Serialize)]
pub struct ReferralResponse {
    pub success: bool,
    pub bonus_amount: f64,
    pub message: String,
}

// ============================================
// API8: Security Misconfiguration
// ============================================

/// Debug information (should never be exposed in production)
#[derive(Debug, Serialize)]
pub struct DebugInfo {
    pub environment: String,
    pub database_url: String,
    pub api_keys: Vec<String>,
    pub internal_ips: Vec<String>,
    pub stack_trace: Option<String>,
}

/// Safe health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Vulnerable health response with excessive information
#[derive(Debug, Serialize)]
pub struct HealthResponseVulnerable {
    pub status: String,
    pub database_status: String,
    pub database_version: String,
    pub server_version: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub active_connections: u32,
}

// ============================================
// API9: Improper Inventory Management
// ============================================

/// API version information
#[derive(Debug, Serialize)]
pub struct ApiVersionInfo {
    pub version: String,
    pub deprecated: bool,
    pub sunset_date: Option<String>,
    pub migration_guide: Option<String>,
}

/// Deprecation notice
#[derive(Debug, Serialize)]
pub struct DeprecationNotice {
    pub endpoint: String,
    pub deprecated_since: String,
    pub sunset_date: String,
    pub replacement: String,
}

// ============================================
// API10: Unsafe Consumption of APIs
// ============================================

/// Webhook payload (external)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
    #[serde(default)]
    pub signature: Option<String>,
}

/// Payment callback from external provider
#[derive(Debug, Deserialize)]
pub struct PaymentCallback {
    pub transaction_id: String,
    pub status: String,
    pub amount: f64,
    pub timestamp: String,
    pub signature: String,
}

/// External user data (for enrichment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalUserData {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub credit_score: Option<u32>,
    pub risk_level: Option<String>,
}
