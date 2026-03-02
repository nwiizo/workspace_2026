use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ScanResult {
    pub id: Uuid,
    pub subnet_id: Uuid,
    pub ip_address: String,
    pub is_alive: bool,
    pub last_seen: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
}
