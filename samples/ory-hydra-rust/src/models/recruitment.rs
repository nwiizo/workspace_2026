use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Candidate status in the recruitment pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    /// Available for hiring
    Available,
    /// Currently in interview process
    Interviewing,
    /// Offer extended, waiting for response
    OfferPending,
    /// Hired by this tenant
    Hired,
    /// Rejected or withdrew
    Unavailable,
}

impl fmt::Display for CandidateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CandidateStatus::Available => write!(f, "available"),
            CandidateStatus::Interviewing => write!(f, "interviewing"),
            CandidateStatus::OfferPending => write!(f, "offer_pending"),
            CandidateStatus::Hired => write!(f, "hired"),
            CandidateStatus::Unavailable => write!(f, "unavailable"),
        }
    }
}

impl FromStr for CandidateStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "available" => Ok(CandidateStatus::Available),
            "interviewing" => Ok(CandidateStatus::Interviewing),
            "offer_pending" => Ok(CandidateStatus::OfferPending),
            "hired" => Ok(CandidateStatus::Hired),
            "unavailable" => Ok(CandidateStatus::Unavailable),
            _ => Err(format!("Invalid candidate status: {}", s)),
        }
    }
}

/// Candidate rarity/quality tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CandidateRarity {
    /// Common candidate - basic skills
    Common,
    /// Uncommon candidate - some experience
    Uncommon,
    /// Rare candidate - skilled professional
    Rare,
    /// Epic candidate - highly skilled expert
    Epic,
    /// Legendary candidate - exceptional talent
    Legendary,
}

impl CandidateRarity {
    /// Get hire cost multiplier based on rarity
    pub fn cost_multiplier(&self) -> f64 {
        match self {
            CandidateRarity::Common => 1.0,
            CandidateRarity::Uncommon => 1.5,
            CandidateRarity::Rare => 2.5,
            CandidateRarity::Epic => 4.0,
            CandidateRarity::Legendary => 7.0,
        }
    }

    /// Get base salary multiplier
    pub fn salary_multiplier(&self) -> f64 {
        match self {
            CandidateRarity::Common => 1.0,
            CandidateRarity::Uncommon => 1.2,
            CandidateRarity::Rare => 1.5,
            CandidateRarity::Epic => 2.0,
            CandidateRarity::Legendary => 3.0,
        }
    }

    /// Get starting level range
    pub fn level_range(&self) -> (i32, i32) {
        match self {
            CandidateRarity::Common => (1, 5),
            CandidateRarity::Uncommon => (3, 10),
            CandidateRarity::Rare => (8, 20),
            CandidateRarity::Epic => (15, 35),
            CandidateRarity::Legendary => (30, 50),
        }
    }
}

impl fmt::Display for CandidateRarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CandidateRarity::Common => write!(f, "common"),
            CandidateRarity::Uncommon => write!(f, "uncommon"),
            CandidateRarity::Rare => write!(f, "rare"),
            CandidateRarity::Epic => write!(f, "epic"),
            CandidateRarity::Legendary => write!(f, "legendary"),
        }
    }
}

impl FromStr for CandidateRarity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "common" => Ok(CandidateRarity::Common),
            "uncommon" => Ok(CandidateRarity::Uncommon),
            "rare" => Ok(CandidateRarity::Rare),
            "epic" => Ok(CandidateRarity::Epic),
            "legendary" => Ok(CandidateRarity::Legendary),
            _ => Err(format!("Invalid candidate rarity: {}", s)),
        }
    }
}

/// Candidate in the recruitment pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: Uuid,
    /// Display name (generated or custom)
    pub name: String,
    /// Avatar/portrait identifier
    pub avatar: String,
    /// Rarity tier
    pub rarity: CandidateRarity,
    /// Starting level if hired
    pub level: i32,
    /// Primary specialty
    pub primary_specialty_id: Uuid,
    /// Proficiency in primary specialty
    pub primary_proficiency: String,
    /// Secondary specialty (optional)
    pub secondary_specialty_id: Option<Uuid>,
    /// Proficiency in secondary specialty
    pub secondary_proficiency: Option<String>,
    /// Expected salary
    pub expected_salary: i64,
    /// Hiring cost (one-time)
    pub hiring_cost: i64,
    /// Base satisfaction when hired
    pub base_satisfaction: i32,
    /// Special trait or perk
    pub trait_name: Option<String>,
    /// Trait description
    pub trait_description: Option<String>,
    /// Current status
    pub status: CandidateStatus,
    /// When this candidate expires from the pool
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Candidate row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CandidateRow {
    pub id: Uuid,
    pub name: String,
    pub avatar: String,
    pub rarity: String,
    pub level: i32,
    pub primary_specialty_id: Uuid,
    pub primary_proficiency: String,
    pub secondary_specialty_id: Option<Uuid>,
    pub secondary_proficiency: Option<String>,
    pub expected_salary: i64,
    pub hiring_cost: i64,
    pub base_satisfaction: i32,
    pub trait_name: Option<String>,
    pub trait_description: Option<String>,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<CandidateRow> for Candidate {
    fn from(row: CandidateRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            avatar: row.avatar,
            rarity: row.rarity.parse().unwrap_or(CandidateRarity::Common),
            level: row.level,
            primary_specialty_id: row.primary_specialty_id,
            primary_proficiency: row.primary_proficiency,
            secondary_specialty_id: row.secondary_specialty_id,
            secondary_proficiency: row.secondary_proficiency,
            expected_salary: row.expected_salary,
            hiring_cost: row.hiring_cost,
            base_satisfaction: row.base_satisfaction,
            trait_name: row.trait_name,
            trait_description: row.trait_description,
            status: row.status.parse().unwrap_or(CandidateStatus::Available),
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

/// Candidate with specialty details for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateWithDetails {
    #[serde(flatten)]
    pub candidate: Candidate,
    pub primary_specialty_name: String,
    pub primary_specialty_color: String,
    pub secondary_specialty_name: Option<String>,
    pub secondary_specialty_color: Option<String>,
    /// Whether current tenant can afford to hire
    pub can_afford: bool,
}

/// Recruitment action/event log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecruitmentEvent {
    pub id: Uuid,
    pub candidate_id: Uuid,
    pub recruiter_id: Uuid,
    pub event_type: RecruitmentEventType,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Types of recruitment events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecruitmentEventType {
    /// Candidate was viewed
    Viewed,
    /// Interview started
    InterviewStarted,
    /// Interview completed
    InterviewCompleted,
    /// Offer extended
    OfferExtended,
    /// Offer accepted (hired)
    OfferAccepted,
    /// Offer rejected
    OfferRejected,
    /// Candidate passed/skipped
    Passed,
}

impl fmt::Display for RecruitmentEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecruitmentEventType::Viewed => write!(f, "viewed"),
            RecruitmentEventType::InterviewStarted => write!(f, "interview_started"),
            RecruitmentEventType::InterviewCompleted => write!(f, "interview_completed"),
            RecruitmentEventType::OfferExtended => write!(f, "offer_extended"),
            RecruitmentEventType::OfferAccepted => write!(f, "offer_accepted"),
            RecruitmentEventType::OfferRejected => write!(f, "offer_rejected"),
            RecruitmentEventType::Passed => write!(f, "passed"),
        }
    }
}

/// Recruitment event row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecruitmentEventRow {
    pub id: Uuid,
    pub candidate_id: Uuid,
    pub recruiter_id: Uuid,
    pub event_type: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<RecruitmentEventRow> for RecruitmentEvent {
    fn from(row: RecruitmentEventRow) -> Self {
        Self {
            id: row.id,
            candidate_id: row.candidate_id,
            recruiter_id: row.recruiter_id,
            event_type: match row.event_type.as_str() {
                "viewed" => RecruitmentEventType::Viewed,
                "interview_started" => RecruitmentEventType::InterviewStarted,
                "interview_completed" => RecruitmentEventType::InterviewCompleted,
                "offer_extended" => RecruitmentEventType::OfferExtended,
                "offer_accepted" => RecruitmentEventType::OfferAccepted,
                "offer_rejected" => RecruitmentEventType::OfferRejected,
                "passed" => RecruitmentEventType::Passed,
                _ => RecruitmentEventType::Viewed,
            },
            notes: row.notes,
            created_at: row.created_at,
        }
    }
}

/// Request to hire a candidate
#[derive(Debug, Deserialize)]
pub struct HireCandidateRequest {
    pub candidate_id: Uuid,
    /// Custom email for the new engineer (optional, will generate if not provided)
    pub email: Option<String>,
    /// Negotiated salary (optional, uses expected_salary if not provided)
    pub negotiated_salary: Option<i64>,
}

/// Result of hiring a candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HireResult {
    pub engineer_id: Uuid,
    pub candidate_id: Uuid,
    pub hiring_cost: i64,
    pub monthly_salary: i64,
    pub new_balance: i64,
}

/// Request to refresh the candidate pool
#[derive(Debug, Deserialize)]
pub struct RefreshPoolRequest {
    /// Number of candidates to generate
    #[serde(default = "default_pool_size")]
    pub count: i32,
    /// Cost to refresh (0 for free refresh)
    #[serde(default)]
    pub refresh_cost: i64,
}

fn default_pool_size() -> i32 {
    5
}

/// Request to start interview with a candidate
#[derive(Debug, Deserialize)]
pub struct StartInterviewRequest {
    pub candidate_id: Uuid,
    pub notes: Option<String>,
}

/// Request to extend an offer to a candidate
#[derive(Debug, Deserialize)]
pub struct ExtendOfferRequest {
    pub candidate_id: Uuid,
    /// Offered salary (can be different from expected)
    pub offered_salary: i64,
    pub notes: Option<String>,
}

/// Candidate pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub total_available: i32,
    pub by_rarity: Vec<RarityCount>,
    pub by_specialty: Vec<SpecialtyCount>,
    pub next_free_refresh_at: Option<DateTime<Utc>>,
    pub refresh_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarityCount {
    pub rarity: CandidateRarity,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialtyCount {
    pub specialty_id: Uuid,
    pub specialty_name: String,
    pub count: i32,
}

/// Candidate name generator data
pub const FIRST_NAMES: &[&str] = &[
    "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Kai", "Sage",
    "Reese", "Charlie", "Finley", "Rowan", "Eden", "Phoenix", "River", "Skyler", "Dakota", "Blake",
    "Cameron", "Drew", "Jamie", "Kelly", "Hayden", "Parker", "Sydney", "Peyton", "Alexis", "Emery",
    "Rory", "Shawn",
];

pub const LAST_NAMES: &[&str] = &[
    "Smith", "Chen", "Garcia", "Kim", "Patel", "Johnson", "Williams", "Brown", "Jones", "Miller",
    "Davis", "Wilson", "Anderson", "Taylor", "Thomas", "Moore", "Jackson", "Martin", "Lee",
    "Thompson", "White", "Harris", "Clark", "Lewis", "Robinson", "Walker", "Young", "Allen",
    "King", "Wright", "Scott", "Hill",
];

/// Candidate traits for special abilities
pub const CANDIDATE_TRAITS: &[(&str, &str)] = &[
    ("Fast Learner", "+20% XP gain from all sources"),
    ("Night Owl", "+15% productivity for late-night incidents"),
    ("Team Player", "+10% satisfaction for all team members"),
    ("Perfectionist", "+25% quality but -10% speed"),
    ("Speed Demon", "+20% faster task completion"),
    ("Mentor", "Nearby engineers gain +10% XP"),
    ("Specialist", "+30% effectiveness in primary specialty"),
    ("Generalist", "Can work on any specialty at 80% efficiency"),
    ("Crisis Handler", "+50% effectiveness on Critical incidents"),
    ("Budget Master", "-15% cost for all training"),
    ("Networker", "+1 candidate quality on pool refresh"),
    ("Resilient", "Satisfaction decreases 50% slower"),
];
