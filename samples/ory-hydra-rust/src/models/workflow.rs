use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Entity type for workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Incident,
    Project,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Incident => write!(f, "incident"),
            EntityType::Project => write!(f, "project"),
        }
    }
}

impl FromStr for EntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "incident" => Ok(EntityType::Incident),
            "project" => Ok(EntityType::Project),
            _ => Err(format!("Invalid entity type: {}", s)),
        }
    }
}

/// Workflow status definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_initial: bool,
    pub is_terminal: bool,
    pub created_at: DateTime<Utc>,
}

/// Workflow status row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkflowStatusRow {
    pub id: Uuid,
    pub entity_type: String,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_initial: bool,
    pub is_terminal: bool,
    pub created_at: DateTime<Utc>,
}

impl From<WorkflowStatusRow> for WorkflowStatus {
    fn from(row: WorkflowStatusRow) -> Self {
        Self {
            id: row.id,
            entity_type: row.entity_type.parse().unwrap_or(EntityType::Incident),
            name: row.name,
            color: row.color,
            display_order: row.display_order,
            is_initial: row.is_initial,
            is_terminal: row.is_terminal,
            created_at: row.created_at,
        }
    }
}

/// Request to create a workflow status
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowStatusRequest {
    pub entity_type: String,
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_initial: bool,
    #[serde(default)]
    pub is_terminal: bool,
}

fn default_color() -> String {
    "#6B7280".to_string()
}

/// Request to update a workflow status
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowStatusRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub display_order: Option<i32>,
    pub is_initial: Option<bool>,
    pub is_terminal: Option<bool>,
}
