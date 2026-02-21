use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "ssr")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub password_hash: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub header_url: Option<String>,
    pub followers_count: i32,
    pub following_count: i32,
    pub posts_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight user info for embedding in posts/lists
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Public profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub header_url: Option<String>,
    pub followers_count: i32,
    pub following_count: i32,
    pub posts_count: i32,
    pub created_at: DateTime<Utc>,
    pub is_following: bool,
    pub is_followed_by: bool,
}
