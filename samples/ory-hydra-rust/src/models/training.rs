use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Training status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TrainingStatus {
    InProgress,
    Completed,
    Cancelled,
}

impl fmt::Display for TrainingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingStatus::InProgress => write!(f, "in_progress"),
            TrainingStatus::Completed => write!(f, "completed"),
            TrainingStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl FromStr for TrainingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "in_progress" => Ok(TrainingStatus::InProgress),
            "completed" => Ok(TrainingStatus::Completed),
            "cancelled" => Ok(TrainingStatus::Cancelled),
            _ => Err(format!("Invalid training status: {}", s)),
        }
    }
}

/// Training definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Training {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub specialty_id: Uuid,
    pub duration_hours: i32,
    pub cost: i64,
    pub xp_gain: i64,
    pub proficiency_boost: i32,
    pub required_level: i32,
    pub created_at: DateTime<Utc>,
}

/// Training row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrainingRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub specialty_id: Uuid,
    pub duration_hours: i32,
    pub cost: i64,
    pub xp_gain: i64,
    pub proficiency_boost: i32,
    pub required_level: i32,
    pub created_at: DateTime<Utc>,
}

impl From<TrainingRow> for Training {
    fn from(row: TrainingRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            specialty_id: row.specialty_id,
            duration_hours: row.duration_hours,
            cost: row.cost,
            xp_gain: row.xp_gain,
            proficiency_boost: row.proficiency_boost,
            required_level: row.required_level,
            created_at: row.created_at,
        }
    }
}

/// Training with specialty info for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingWithSpecialty {
    #[serde(flatten)]
    pub training: Training,
    pub specialty_name: String,
    pub specialty_color: String,
}

/// Engineer's training session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerTraining {
    pub id: Uuid,
    pub engineer_id: Uuid,
    pub training_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub expected_completion_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TrainingStatus,
}

/// Engineer training row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EngineerTrainingRow {
    pub id: Uuid,
    pub engineer_id: Uuid,
    pub training_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub expected_completion_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
}

impl From<EngineerTrainingRow> for EngineerTraining {
    fn from(row: EngineerTrainingRow) -> Self {
        Self {
            id: row.id,
            engineer_id: row.engineer_id,
            training_id: row.training_id,
            started_at: row.started_at,
            expected_completion_at: row.expected_completion_at,
            completed_at: row.completed_at,
            status: row.status.parse().unwrap_or(TrainingStatus::InProgress),
        }
    }
}

impl EngineerTraining {
    /// Check if training can be completed
    pub fn can_complete(&self, now: DateTime<Utc>) -> bool {
        self.status == TrainingStatus::InProgress && now >= self.expected_completion_at
    }

    /// Calculate progress (0.0 - 1.0)
    pub fn progress(&self, now: DateTime<Utc>) -> f32 {
        if self.status == TrainingStatus::Completed {
            return 1.0;
        }
        if self.status == TrainingStatus::Cancelled {
            return 0.0;
        }

        let total_duration = (self.expected_completion_at - self.started_at).num_seconds() as f32;
        let elapsed = (now - self.started_at).num_seconds() as f32;

        (elapsed / total_duration).clamp(0.0, 1.0)
    }
}

/// Engineer training with full details for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerTrainingWithDetails {
    #[serde(flatten)]
    pub session: EngineerTraining,
    pub training: TrainingWithSpecialty,
    pub engineer_email: String,
    pub progress: f32,
    pub can_complete: bool,
}

/// Request to create a training definition
#[derive(Debug, Deserialize)]
pub struct CreateTrainingRequest {
    pub name: String,
    pub description: String,
    pub specialty_id: Uuid,
    #[serde(default = "default_duration")]
    pub duration_hours: i32,
    #[serde(default)]
    pub cost: i64,
    #[serde(default = "default_xp")]
    pub xp_gain: i64,
    #[serde(default = "default_proficiency")]
    pub proficiency_boost: i32,
    #[serde(default)]
    pub required_level: i32,
}

fn default_duration() -> i32 {
    8 // 8 hours
}

fn default_xp() -> i64 {
    100
}

fn default_proficiency() -> i32 {
    1
}

/// Request to update a training definition
#[derive(Debug, Deserialize)]
pub struct UpdateTrainingRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub specialty_id: Option<Uuid>,
    pub duration_hours: Option<i32>,
    pub cost: Option<i64>,
    pub xp_gain: Option<i64>,
    pub proficiency_boost: Option<i32>,
    pub required_level: Option<i32>,
}

/// Request to start training for an engineer
#[derive(Debug, Deserialize)]
pub struct StartTrainingRequest {
    pub training_id: Uuid,
}

/// Result of completing a training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCompletionResult {
    pub session: EngineerTraining,
    pub xp_gained: i64,
    pub proficiency_gained: i32,
    pub new_level: Option<i32>,
    pub achievements_unlocked: Vec<Uuid>,
}

/// Default trainings for each specialty
pub fn default_trainings(specialty_id: Uuid, specialty_name: &str) -> Vec<CreateTrainingRequest> {
    vec![
        CreateTrainingRequest {
            name: format!("{} Fundamentals", specialty_name),
            description: format!("Learn the basics of {}", specialty_name),
            specialty_id,
            duration_hours: 4,
            cost: 1000,
            xp_gain: 50,
            proficiency_boost: 1,
            required_level: 0,
        },
        CreateTrainingRequest {
            name: format!("{} Intermediate", specialty_name),
            description: format!("Deepen your {} skills", specialty_name),
            specialty_id,
            duration_hours: 8,
            cost: 2500,
            xp_gain: 150,
            proficiency_boost: 1,
            required_level: 5,
        },
        CreateTrainingRequest {
            name: format!("{} Advanced", specialty_name),
            description: format!("Master advanced {} techniques", specialty_name),
            specialty_id,
            duration_hours: 16,
            cost: 5000,
            xp_gain: 400,
            proficiency_boost: 1,
            required_level: 15,
        },
    ]
}
