use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user::UserSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationEvent {
    Like,
    Rechirp,
    Follow,
    Reply,
    Mention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub actor: UserSummary,
    pub event_type: NotificationEvent,
    pub post_id: Option<Uuid>,
    pub post_content: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Display for NotificationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Like => write!(f, "like"),
            Self::Rechirp => write!(f, "rechirp"),
            Self::Follow => write!(f, "follow"),
            Self::Reply => write!(f, "reply"),
            Self::Mention => write!(f, "mention"),
        }
    }
}

impl std::str::FromStr for NotificationEvent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "like" => Ok(Self::Like),
            "rechirp" => Ok(Self::Rechirp),
            "follow" => Ok(Self::Follow),
            "reply" => Ok(Self::Reply),
            "mention" => Ok(Self::Mention),
            other => Err(format!("Unknown notification event: {other}")),
        }
    }
}
