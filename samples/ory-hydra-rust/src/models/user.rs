use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::role::UserRole;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: Option<String>,
    pub role: UserRole,
    pub tenant_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub status: UserStatus,
}

/// User from database (with string role for sqlx compatibility)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: Option<String>,
    pub role: String,
    pub tenant_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            email_verified: row.email_verified,
            password_hash: row.password_hash,
            role: row.role.parse().unwrap_or_default(),
            tenant_id: row.tenant_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_login_at: row.last_login_at,
            status: row.status.parse().unwrap_or_default(),
        }
    }
}

/// User status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    #[default]
    Active,
    Suspended,
    Deleted,
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStatus::Active => write!(f, "active"),
            UserStatus::Suspended => write!(f, "suspended"),
            UserStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl std::str::FromStr for UserStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(UserStatus::Active),
            "suspended" => Ok(UserStatus::Suspended),
            "deleted" => Ok(UserStatus::Deleted),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

/// Session data stored in cookie/redis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserSession {
    pub user_id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub tenant_id: Option<Uuid>,
    pub authenticated_at: i64,
}

/// JWT Claims with tenant context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
}

impl Claims {
    /// Get the role as UserRole enum
    pub fn get_role(&self) -> UserRole {
        self.role
            .as_ref()
            .and_then(|r| r.parse().ok())
            .unwrap_or_default()
    }
}

/// Token response for API
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Request to register a new user
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub role: Option<String>,
    pub tenant_id: Option<Uuid>,
}

/// Request to login via API
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct UserLoginRequest {
    pub email: String,
    pub password: String,
}

/// User profile (safe to return to client)
#[derive(Debug, Serialize)]
#[allow(unused)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub role: UserRole,
    pub tenant_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            email_verified: user.email_verified,
            role: user.role,
            tenant_id: user.tenant_id,
            created_at: user.created_at,
        }
    }
}
