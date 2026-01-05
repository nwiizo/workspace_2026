use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Achievement/Badge category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AchievementCategory {
    Incidents,
    Projects,
    Skills,
    Special,
}

impl fmt::Display for AchievementCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AchievementCategory::Incidents => write!(f, "incidents"),
            AchievementCategory::Projects => write!(f, "projects"),
            AchievementCategory::Skills => write!(f, "skills"),
            AchievementCategory::Special => write!(f, "special"),
        }
    }
}

impl FromStr for AchievementCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "incidents" => Ok(AchievementCategory::Incidents),
            "projects" => Ok(AchievementCategory::Projects),
            "skills" => Ok(AchievementCategory::Skills),
            "special" => Ok(AchievementCategory::Special),
            _ => Err(format!("Invalid achievement category: {}", s)),
        }
    }
}

/// Achievement condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    /// Total incidents resolved
    IncidentCount,
    /// Total projects completed
    ProjectCount,
    /// Total XP earned
    TotalXp,
    /// Reach a specific level
    ReachLevel,
    /// Resolve incident within X minutes
    FastIncidentResolve,
    /// Complete project under budget
    UnderBudget,
    /// Maintain satisfaction above X for Y days
    HighSatisfaction,
    /// Complete X extreme difficulty tasks
    ExtremeDifficulty,
    /// Total revenue generated
    TotalRevenue,
    /// Unlock X skills
    SkillsUnlocked,
    /// Custom condition (checked by game engine)
    Custom,
}

impl fmt::Display for ConditionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConditionType::IncidentCount => write!(f, "incident_count"),
            ConditionType::ProjectCount => write!(f, "project_count"),
            ConditionType::TotalXp => write!(f, "total_xp"),
            ConditionType::ReachLevel => write!(f, "reach_level"),
            ConditionType::FastIncidentResolve => write!(f, "fast_incident_resolve"),
            ConditionType::UnderBudget => write!(f, "under_budget"),
            ConditionType::HighSatisfaction => write!(f, "high_satisfaction"),
            ConditionType::ExtremeDifficulty => write!(f, "extreme_difficulty"),
            ConditionType::TotalRevenue => write!(f, "total_revenue"),
            ConditionType::SkillsUnlocked => write!(f, "skills_unlocked"),
            ConditionType::Custom => write!(f, "custom"),
        }
    }
}

impl FromStr for ConditionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "incident_count" => Ok(ConditionType::IncidentCount),
            "project_count" => Ok(ConditionType::ProjectCount),
            "total_xp" => Ok(ConditionType::TotalXp),
            "reach_level" => Ok(ConditionType::ReachLevel),
            "fast_incident_resolve" => Ok(ConditionType::FastIncidentResolve),
            "under_budget" => Ok(ConditionType::UnderBudget),
            "high_satisfaction" => Ok(ConditionType::HighSatisfaction),
            "extreme_difficulty" => Ok(ConditionType::ExtremeDifficulty),
            "total_revenue" => Ok(ConditionType::TotalRevenue),
            "skills_unlocked" => Ok(ConditionType::SkillsUnlocked),
            "custom" => Ok(ConditionType::Custom),
            _ => Err(format!("Invalid condition type: {}", s)),
        }
    }
}

/// Achievement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: AchievementCategory,
    pub condition_type: String,
    pub condition_value: i32,
    pub xp_reward: i64,
    pub is_hidden: bool,
    pub created_at: DateTime<Utc>,
}

/// Achievement row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AchievementRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub condition_type: String,
    pub condition_value: i32,
    pub xp_reward: i64,
    pub is_hidden: bool,
    pub created_at: DateTime<Utc>,
}

impl From<AchievementRow> for Achievement {
    fn from(row: AchievementRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            icon: row.icon,
            category: row.category.parse().unwrap_or(AchievementCategory::Special),
            condition_type: row.condition_type,
            condition_value: row.condition_value,
            xp_reward: row.xp_reward,
            is_hidden: row.is_hidden,
            created_at: row.created_at,
        }
    }
}

/// Engineer's unlocked achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerAchievement {
    pub engineer_id: Uuid,
    pub achievement_id: Uuid,
    pub unlocked_at: DateTime<Utc>,
}

/// Engineer achievement row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EngineerAchievementRow {
    pub engineer_id: Uuid,
    pub achievement_id: Uuid,
    pub unlocked_at: DateTime<Utc>,
}

impl From<EngineerAchievementRow> for EngineerAchievement {
    fn from(row: EngineerAchievementRow) -> Self {
        Self {
            engineer_id: row.engineer_id,
            achievement_id: row.achievement_id,
            unlocked_at: row.unlocked_at,
        }
    }
}

/// Achievement with unlock status for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementWithStatus {
    #[serde(flatten)]
    pub achievement: Achievement,
    pub is_unlocked: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
    /// Progress towards the achievement (0.0 - 1.0)
    pub progress: f32,
}

/// Request to create an achievement
#[derive(Debug, Deserialize)]
pub struct CreateAchievementRequest {
    pub name: String,
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    pub category: String,
    pub condition_type: String,
    pub condition_value: i32,
    #[serde(default)]
    pub xp_reward: i64,
    #[serde(default)]
    pub is_hidden: bool,
}

fn default_icon() -> String {
    "trophy".to_string()
}

/// Request to update an achievement
#[derive(Debug, Deserialize)]
pub struct UpdateAchievementRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub condition_type: Option<String>,
    pub condition_value: Option<i32>,
    pub xp_reward: Option<i64>,
    pub is_hidden: Option<bool>,
}

/// Default achievements to be created when tenant is initialized
pub fn default_achievements() -> Vec<CreateAchievementRequest> {
    vec![
        CreateAchievementRequest {
            name: "First Blood".to_string(),
            description: "Resolve your first incident".to_string(),
            icon: "fire".to_string(),
            category: "incidents".to_string(),
            condition_type: "incident_count".to_string(),
            condition_value: 1,
            xp_reward: 50,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Incident Hunter".to_string(),
            description: "Resolve 10 incidents".to_string(),
            icon: "target".to_string(),
            category: "incidents".to_string(),
            condition_type: "incident_count".to_string(),
            condition_value: 10,
            xp_reward: 200,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Incident Master".to_string(),
            description: "Resolve 100 incidents".to_string(),
            icon: "crown".to_string(),
            category: "incidents".to_string(),
            condition_type: "incident_count".to_string(),
            condition_value: 100,
            xp_reward: 1000,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Project Starter".to_string(),
            description: "Complete your first project".to_string(),
            icon: "rocket".to_string(),
            category: "projects".to_string(),
            condition_type: "project_count".to_string(),
            condition_value: 1,
            xp_reward: 100,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Project Pro".to_string(),
            description: "Complete 10 projects".to_string(),
            icon: "star".to_string(),
            category: "projects".to_string(),
            condition_type: "project_count".to_string(),
            condition_value: 10,
            xp_reward: 500,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Level 10".to_string(),
            description: "Reach level 10".to_string(),
            icon: "badge".to_string(),
            category: "skills".to_string(),
            condition_type: "reach_level".to_string(),
            condition_value: 10,
            xp_reward: 300,
            is_hidden: false,
        },
        CreateAchievementRequest {
            name: "Extreme Challenger".to_string(),
            description: "Complete an extreme difficulty task".to_string(),
            icon: "skull".to_string(),
            category: "special".to_string(),
            condition_type: "extreme_difficulty".to_string(),
            condition_value: 1,
            xp_reward: 500,
            is_hidden: true,
        },
    ]
}
