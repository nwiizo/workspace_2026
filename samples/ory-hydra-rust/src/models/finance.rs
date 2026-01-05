use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    /// Revenue from resolving an incident
    IncidentReward,
    /// Revenue from completing a project
    ProjectReward,
    /// Monthly salary payment
    SalaryPayment,
    /// Bonus for hiring an engineer
    HiringBonus,
    /// Severance pay when firing an engineer
    FiringSeverance,
    /// Training cost
    Training,
    /// Skill unlock cost
    SkillUnlock,
    /// Achievement bonus
    AchievementBonus,
    /// Manual adjustment
    Adjustment,
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionType::IncidentReward => write!(f, "incident_reward"),
            TransactionType::ProjectReward => write!(f, "project_reward"),
            TransactionType::SalaryPayment => write!(f, "salary_payment"),
            TransactionType::HiringBonus => write!(f, "hiring_bonus"),
            TransactionType::FiringSeverance => write!(f, "firing_severance"),
            TransactionType::Training => write!(f, "training"),
            TransactionType::SkillUnlock => write!(f, "skill_unlock"),
            TransactionType::AchievementBonus => write!(f, "achievement_bonus"),
            TransactionType::Adjustment => write!(f, "adjustment"),
        }
    }
}

impl FromStr for TransactionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "incident_reward" => Ok(TransactionType::IncidentReward),
            "project_reward" => Ok(TransactionType::ProjectReward),
            "salary_payment" => Ok(TransactionType::SalaryPayment),
            "hiring_bonus" => Ok(TransactionType::HiringBonus),
            "firing_severance" => Ok(TransactionType::FiringSeverance),
            "training" => Ok(TransactionType::Training),
            "skill_unlock" => Ok(TransactionType::SkillUnlock),
            "achievement_bonus" => Ok(TransactionType::AchievementBonus),
            "adjustment" => Ok(TransactionType::Adjustment),
            _ => Err(format!("Invalid transaction type: {}", s)),
        }
    }
}

/// Tenant finance state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantFinance {
    pub tenant_id: Uuid,
    pub balance: i64,
    pub monthly_revenue: i64,
    pub monthly_expenses: i64,
    pub revenue_target: i64,
    pub updated_at: DateTime<Utc>,
}

/// Tenant finance row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TenantFinanceRow {
    pub tenant_id: Uuid,
    pub balance: i64,
    pub monthly_revenue: i64,
    pub monthly_expenses: i64,
    pub revenue_target: i64,
    pub updated_at: DateTime<Utc>,
}

impl From<TenantFinanceRow> for TenantFinance {
    fn from(row: TenantFinanceRow) -> Self {
        Self {
            tenant_id: row.tenant_id,
            balance: row.balance,
            monthly_revenue: row.monthly_revenue,
            monthly_expenses: row.monthly_expenses,
            revenue_target: row.revenue_target,
            updated_at: row.updated_at,
        }
    }
}

impl TenantFinance {
    /// Check if tenant is at risk of not meeting revenue target
    pub fn is_at_risk(&self) -> bool {
        self.monthly_revenue < self.revenue_target / 2
    }

    /// Check if tenant is in deficit
    pub fn is_in_deficit(&self) -> bool {
        self.balance < 0
    }

    /// Calculate net profit/loss
    pub fn net_monthly(&self) -> i64 {
        self.monthly_revenue - self.monthly_expenses
    }
}

/// Transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub description: String,
    pub engineer_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Transaction row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TransactionRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub transaction_type: String,
    pub amount: i64,
    pub description: String,
    pub engineer_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<TransactionRow> for Transaction {
    fn from(row: TransactionRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            transaction_type: row
                .transaction_type
                .parse()
                .unwrap_or(TransactionType::Adjustment),
            amount: row.amount,
            description: row.description,
            engineer_id: row.engineer_id,
            incident_id: row.incident_id,
            project_id: row.project_id,
            created_at: row.created_at,
        }
    }
}

/// Transaction with related entity names for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWithDetails {
    #[serde(flatten)]
    pub transaction: Transaction,
    pub engineer_email: Option<String>,
    pub incident_title: Option<String>,
    pub project_title: Option<String>,
}

/// Finance summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceSummary {
    pub balance: i64,
    pub monthly_revenue: i64,
    pub monthly_expenses: i64,
    pub net_monthly: i64,
    pub revenue_target: i64,
    pub is_at_risk: bool,
    pub is_in_deficit: bool,
    /// Total salary obligations
    pub total_salaries: i64,
    /// Number of active engineers
    pub active_engineers: i32,
    /// Revenue breakdown by type
    pub revenue_by_type: Vec<RevenueBreakdown>,
    /// Recent transactions
    pub recent_transactions: Vec<TransactionWithDetails>,
}

/// Revenue breakdown by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueBreakdown {
    pub transaction_type: TransactionType,
    pub total: i64,
    pub count: i32,
}

/// Request to create a transaction
#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub transaction_type: String,
    pub amount: i64,
    pub description: String,
    pub engineer_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

/// Request to pay salaries
#[derive(Debug, Deserialize)]
pub struct PaySalariesRequest {
    /// If None, pay all active engineers
    pub engineer_ids: Option<Vec<Uuid>>,
}

/// Salary payment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalaryPaymentResult {
    pub engineers_paid: i32,
    pub total_amount: i64,
    pub transactions: Vec<Transaction>,
    pub new_balance: i64,
}

/// Request to adjust balance
#[derive(Debug, Deserialize)]
pub struct AdjustBalanceRequest {
    pub amount: i64,
    pub description: String,
}
