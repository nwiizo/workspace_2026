use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Skill bonus type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BonusType {
    /// Multiply XP earned
    XpMultiplier,
    /// Increase salary
    SalaryBonus,
    /// Reduce task completion time
    SpeedBonus,
    /// Increase satisfaction gain
    SatisfactionBonus,
    /// Increase revenue from tasks
    RevenueBonus,
    /// Reduce training time
    TrainingSpeedBonus,
    /// Increase success rate on difficult tasks
    DifficultyBonus,
}

impl fmt::Display for BonusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BonusType::XpMultiplier => write!(f, "xp_multiplier"),
            BonusType::SalaryBonus => write!(f, "salary_bonus"),
            BonusType::SpeedBonus => write!(f, "speed_bonus"),
            BonusType::SatisfactionBonus => write!(f, "satisfaction_bonus"),
            BonusType::RevenueBonus => write!(f, "revenue_bonus"),
            BonusType::TrainingSpeedBonus => write!(f, "training_speed_bonus"),
            BonusType::DifficultyBonus => write!(f, "difficulty_bonus"),
        }
    }
}

impl FromStr for BonusType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xp_multiplier" => Ok(BonusType::XpMultiplier),
            "salary_bonus" => Ok(BonusType::SalaryBonus),
            "speed_bonus" => Ok(BonusType::SpeedBonus),
            "satisfaction_bonus" => Ok(BonusType::SatisfactionBonus),
            "revenue_bonus" => Ok(BonusType::RevenueBonus),
            "training_speed_bonus" => Ok(BonusType::TrainingSpeedBonus),
            "difficulty_bonus" => Ok(BonusType::DifficultyBonus),
            _ => Err(format!("Invalid bonus type: {}", s)),
        }
    }
}

/// Skill tree node definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillNode {
    pub id: Uuid,
    pub specialty_id: Uuid,
    pub name: String,
    pub description: String,
    pub tier: i32,
    pub required_level: i32,
    pub required_xp: i64,
    pub parent_node_id: Option<Uuid>,
    pub bonus_type: String,
    pub bonus_value: i32,
    pub icon: String,
    pub created_at: DateTime<Utc>,
}

/// Skill node row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SkillNodeRow {
    pub id: Uuid,
    pub specialty_id: Uuid,
    pub name: String,
    pub description: String,
    pub tier: i32,
    pub required_level: i32,
    pub required_xp: i64,
    pub parent_node_id: Option<Uuid>,
    pub bonus_type: String,
    pub bonus_value: i32,
    pub icon: String,
    pub created_at: DateTime<Utc>,
}

impl From<SkillNodeRow> for SkillNode {
    fn from(row: SkillNodeRow) -> Self {
        Self {
            id: row.id,
            specialty_id: row.specialty_id,
            name: row.name,
            description: row.description,
            tier: row.tier,
            required_level: row.required_level,
            required_xp: row.required_xp,
            parent_node_id: row.parent_node_id,
            bonus_type: row.bonus_type,
            bonus_value: row.bonus_value,
            icon: row.icon,
            created_at: row.created_at,
        }
    }
}

/// Engineer's unlocked skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerSkillNode {
    pub engineer_id: Uuid,
    pub skill_node_id: Uuid,
    pub unlocked_at: DateTime<Utc>,
}

/// Engineer skill node row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EngineerSkillNodeRow {
    pub engineer_id: Uuid,
    pub skill_node_id: Uuid,
    pub unlocked_at: DateTime<Utc>,
}

impl From<EngineerSkillNodeRow> for EngineerSkillNode {
    fn from(row: EngineerSkillNodeRow) -> Self {
        Self {
            engineer_id: row.engineer_id,
            skill_node_id: row.skill_node_id,
            unlocked_at: row.unlocked_at,
        }
    }
}

/// Skill node with unlock status for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillNodeWithStatus {
    #[serde(flatten)]
    pub node: SkillNode,
    pub is_unlocked: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub can_unlock: bool,
    pub specialty_name: String,
    pub specialty_color: String,
}

/// Skill tree for a specialty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTree {
    pub specialty_id: Uuid,
    pub specialty_name: String,
    pub specialty_color: String,
    pub nodes: Vec<SkillNodeWithStatus>,
}

/// Request to create a skill node
#[derive(Debug, Deserialize)]
pub struct CreateSkillNodeRequest {
    pub specialty_id: Uuid,
    pub name: String,
    pub description: String,
    #[serde(default = "default_tier")]
    pub tier: i32,
    #[serde(default = "default_level")]
    pub required_level: i32,
    #[serde(default)]
    pub required_xp: i64,
    pub parent_node_id: Option<Uuid>,
    pub bonus_type: String,
    #[serde(default = "default_bonus")]
    pub bonus_value: i32,
    #[serde(default = "default_icon")]
    pub icon: String,
}

fn default_tier() -> i32 {
    1
}

fn default_level() -> i32 {
    1
}

fn default_bonus() -> i32 {
    10
}

fn default_icon() -> String {
    "star".to_string()
}

/// Request to update a skill node
#[derive(Debug, Deserialize)]
pub struct UpdateSkillNodeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tier: Option<i32>,
    pub required_level: Option<i32>,
    pub required_xp: Option<i64>,
    pub parent_node_id: Option<Uuid>,
    pub bonus_type: Option<String>,
    pub bonus_value: Option<i32>,
    pub icon: Option<String>,
}

/// Request to unlock a skill
#[derive(Debug, Deserialize)]
pub struct UnlockSkillRequest {
    pub skill_node_id: Uuid,
}
