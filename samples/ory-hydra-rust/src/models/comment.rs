use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::workflow::EntityType;

/// Comment/Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Comment row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentRow {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<CommentRow> for Comment {
    fn from(row: CommentRow) -> Self {
        Self {
            id: row.id,
            entity_type: row.entity_type.parse().unwrap_or(EntityType::Incident),
            entity_id: row.entity_id,
            author_id: row.author_id,
            content: row.content,
            is_internal: row.is_internal,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Comment with author info for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithAuthor {
    #[serde(flatten)]
    pub comment: Comment,
    pub author_email: String,
}

/// Request to create a comment
#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    #[serde(default)]
    pub is_internal: bool,
}

/// Request to update a comment
#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}
