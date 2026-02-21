use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user::UserSummary;

#[cfg(feature = "ssr")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub reply_to_id: Option<Uuid>,
    pub reply_count: i32,
    pub like_count: i32,
    pub rechirp_count: i32,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Post with author info and interaction state for timeline display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostWithMeta {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub reply_to_id: Option<Uuid>,
    pub reply_count: i32,
    pub like_count: i32,
    pub rechirp_count: i32,
    pub created_at: DateTime<Utc>,
    pub author: UserSummary,
    pub liked_by_me: bool,
    pub rechirped_by_me: bool,
}

/// Detailed post view including thread context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDetail {
    pub post: PostWithMeta,
    pub parent: Option<Box<PostWithMeta>>,
    pub replies: Vec<PostWithMeta>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimelineTab {
    ForYou,
    Following,
}

/// Media attached to a post
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]
pub struct PostMedia {
    pub id: Uuid,
    pub post_id: Uuid,
    pub media_url: String,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub position: i16,
}
