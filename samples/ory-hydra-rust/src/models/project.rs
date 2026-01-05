use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use super::engineer::Difficulty;

/// Project priority levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    #[default]
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(Priority::High),
            "medium" => Ok(Priority::Medium),
            "low" => Ok(Priority::Low),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

/// Project/Case entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status_id: Uuid,
    pub priority: Priority,
    pub difficulty: Difficulty,
    pub reward: i64,
    pub deadline: Option<DateTime<Utc>>,
    pub estimated_hours: Option<i32>,
    pub actual_hours: i32,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Project row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status_id: Uuid,
    pub priority: String,
    pub difficulty: String,
    pub reward: i64,
    pub deadline: Option<DateTime<Utc>>,
    pub estimated_hours: Option<i32>,
    pub actual_hours: i32,
    pub required_specialty_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<ProjectRow> for Project {
    fn from(row: ProjectRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            description: row.description,
            status_id: row.status_id,
            priority: row.priority.parse().unwrap_or_default(),
            difficulty: row.difficulty.parse().unwrap_or_default(),
            reward: row.reward,
            deadline: row.deadline,
            estimated_hours: row.estimated_hours,
            actual_hours: row.actual_hours,
            required_specialty_id: row.required_specialty_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
        }
    }
}

/// Project with status name for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithStatus {
    #[serde(flatten)]
    pub project: Project,
    pub status_name: String,
    pub status_color: String,
    /// Whether the project is in a terminal/completed state
    pub is_completed: bool,
    pub assigned_engineers: Vec<AssignedEngineer>,
}

/// Assigned engineer info for project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedEngineer {
    pub id: Uuid,
    pub email: String,
    pub role_in_assignment: String,
}

/// Request to create a project
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub reward: Option<i64>,
    pub deadline: Option<DateTime<Utc>>,
    pub estimated_hours: Option<i32>,
    #[serde(default)]
    pub required_specialty_id: Option<Uuid>,
}

/// Request to update a project
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub difficulty: Option<String>,
    pub reward: Option<i64>,
    pub deadline: Option<DateTime<Utc>>,
    pub estimated_hours: Option<i32>,
    pub required_specialty_id: Option<Uuid>,
}

/// Request to assign engineers to a project
#[derive(Debug, Deserialize)]
pub struct AssignProjectRequest {
    pub engineer_id: Uuid,
    #[serde(default)]
    pub role: Option<String>,
}

/// Request to change project status
#[derive(Debug, Deserialize)]
pub struct ChangeProjectStatusRequest {
    pub status_id: Uuid,
}

/// Request to update project hours
#[derive(Debug, Deserialize)]
pub struct UpdateProjectHoursRequest {
    pub hours_to_add: i32,
}
