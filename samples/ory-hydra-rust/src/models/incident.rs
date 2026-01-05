use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use super::engineer::Difficulty;

/// Incident severity levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// P1 - Immediate response required
    Critical,
    /// P2 - Response within 4 hours
    High,
    /// P3 - Response within 24 hours
    #[default]
    Medium,
    /// P4 - Response when available
    Low,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "critical"),
            Severity::High => write!(f, "high"),
            Severity::Medium => write!(f, "medium"),
            Severity::Low => write!(f, "low"),
        }
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Severity::Critical),
            "high" => Ok(Severity::High),
            "medium" => Ok(Severity::Medium),
            "low" => Ok(Severity::Low),
            _ => Err(format!("Invalid severity: {}", s)),
        }
    }
}

/// Incident entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: Severity,
    pub difficulty: Difficulty,
    pub reward: i64,
    pub status_id: Uuid,
    pub assigned_engineer_id: Option<Uuid>,
    pub reporter_id: Uuid,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// Incident row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IncidentRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: String,
    pub difficulty: String,
    pub reward: i64,
    pub status_id: Uuid,
    pub assigned_engineer_id: Option<Uuid>,
    pub reporter_id: Uuid,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl From<IncidentRow> for Incident {
    fn from(row: IncidentRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            description: row.description,
            severity: row.severity.parse().unwrap_or_default(),
            difficulty: row.difficulty.parse().unwrap_or_default(),
            reward: row.reward,
            status_id: row.status_id,
            assigned_engineer_id: row.assigned_engineer_id,
            reporter_id: row.reporter_id,
            required_specialty_id: row.required_specialty_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            resolved_at: row.resolved_at,
            closed_at: row.closed_at,
        }
    }
}

/// Incident with status name for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentWithStatus {
    #[serde(flatten)]
    pub incident: Incident,
    pub status_name: String,
    pub status_color: String,
    /// Whether the incident is in a terminal/resolved state
    pub is_resolved: bool,
}

/// Request to create an incident
#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub reward: Option<i64>,
    #[serde(default)]
    pub required_specialty_id: Option<Uuid>,
}

/// Request to update an incident
#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub difficulty: Option<String>,
    pub reward: Option<i64>,
    pub required_specialty_id: Option<Uuid>,
}

/// Request to assign an engineer to an incident
#[derive(Debug, Deserialize)]
pub struct AssignIncidentRequest {
    pub engineer_id: Uuid,
}

/// Request to change incident status
#[derive(Debug, Deserialize)]
pub struct ChangeIncidentStatusRequest {
    pub status_id: Uuid,
}
