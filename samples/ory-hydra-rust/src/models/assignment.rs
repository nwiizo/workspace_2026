use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Type of entity that can be assigned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AssignableType {
    Incident,
    Project,
}

impl fmt::Display for AssignableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssignableType::Incident => write!(f, "incident"),
            AssignableType::Project => write!(f, "project"),
        }
    }
}

impl FromStr for AssignableType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "incident" => Ok(AssignableType::Incident),
            "project" => Ok(AssignableType::Project),
            _ => Err(format!("Invalid assignable type: {}", s)),
        }
    }
}

/// Assignment entity - links engineers to incidents/projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: Uuid,
    pub assignable_type: AssignableType,
    pub assignable_id: Uuid,
    pub engineer_id: Uuid,
    pub role_in_assignment: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Uuid,
}

/// Assignment row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AssignmentRow {
    pub id: Uuid,
    pub assignable_type: String,
    pub assignable_id: Uuid,
    pub engineer_id: Uuid,
    pub role_in_assignment: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Uuid,
}

impl From<AssignmentRow> for Assignment {
    fn from(row: AssignmentRow) -> Self {
        Self {
            id: row.id,
            assignable_type: row
                .assignable_type
                .parse()
                .unwrap_or(AssignableType::Incident),
            assignable_id: row.assignable_id,
            engineer_id: row.engineer_id,
            role_in_assignment: row.role_in_assignment,
            assigned_at: row.assigned_at,
            assigned_by: row.assigned_by,
        }
    }
}

/// Assignment with engineer details for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentWithEngineer {
    #[serde(flatten)]
    pub assignment: Assignment,
    pub engineer_email: String,
    pub assigned_by_email: String,
}

/// Request to create an assignment
#[derive(Debug, Deserialize)]
pub struct CreateAssignmentRequest {
    pub engineer_id: Uuid,
    #[serde(default = "default_role")]
    pub role_in_assignment: String,
}

fn default_role() -> String {
    "assignee".to_string()
}
