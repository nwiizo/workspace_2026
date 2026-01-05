use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Specialty/Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specialty {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

/// Specialty row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SpecialtyRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

impl From<SpecialtyRow> for Specialty {
    fn from(row: SpecialtyRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            color: row.color,
            is_default: row.is_default,
            created_at: row.created_at,
        }
    }
}

/// Proficiency level - hidden from non-managers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Proficiency {
    Expert,
    #[default]
    Intermediate,
    Beginner,
}

impl Proficiency {
    /// Get numeric skill level (1-3)
    pub fn skill_level(&self) -> i32 {
        match self {
            Proficiency::Beginner => 1,
            Proficiency::Intermediate => 2,
            Proficiency::Expert => 3,
        }
    }
}

impl fmt::Display for Proficiency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Proficiency::Expert => write!(f, "expert"),
            Proficiency::Intermediate => write!(f, "intermediate"),
            Proficiency::Beginner => write!(f, "beginner"),
        }
    }
}

impl FromStr for Proficiency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "expert" => Ok(Proficiency::Expert),
            "intermediate" => Ok(Proficiency::Intermediate),
            "beginner" => Ok(Proficiency::Beginner),
            _ => Err(format!("Invalid proficiency: {}", s)),
        }
    }
}

/// Engineer's specialty with proficiency level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerSpecialty {
    pub engineer_id: Uuid,
    pub specialty_id: Uuid,
    pub proficiency: Proficiency,
}

/// Engineer specialty row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EngineerSpecialtyRow {
    pub engineer_id: Uuid,
    pub specialty_id: Uuid,
    pub proficiency: String,
}

impl From<EngineerSpecialtyRow> for EngineerSpecialty {
    fn from(row: EngineerSpecialtyRow) -> Self {
        Self {
            engineer_id: row.engineer_id,
            specialty_id: row.specialty_id,
            proficiency: row.proficiency.parse().unwrap_or_default(),
        }
    }
}

/// Specialty with proficiency for API response
/// Note: proficiency is Option - only shown to Managers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialtyWithProficiency {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    /// Only included for Manager+ roles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proficiency: Option<Proficiency>,
}

/// Request to create a specialty
#[derive(Debug, Deserialize)]
pub struct CreateSpecialtyRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#6B7280".to_string()
}

/// Request to update a specialty
#[derive(Debug, Deserialize)]
pub struct UpdateSpecialtyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
}

/// Request to update engineer's specialties
#[derive(Debug, Deserialize)]
pub struct UpdateEngineerSpecialtiesRequest {
    pub specialties: Vec<EngineerSpecialtyInput>,
}

/// Input for a single specialty assignment
#[derive(Debug, Deserialize)]
pub struct EngineerSpecialtyInput {
    pub specialty_id: Uuid,
    pub proficiency: String,
}
