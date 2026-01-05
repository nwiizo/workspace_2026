use axum::{extract::FromRequestParts, http::request::Parts};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use crate::error::AppError;

/// Tenant entity representing an EC shop
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: Uuid,
    /// URL-friendly slug (e.g., "shop-a")
    pub slug: String,
    /// Display name (e.g., "Shop A")
    pub name: String,
    /// Database schema name (e.g., "tenant_abc123")
    pub schema_name: String,
    /// Subscription plan
    pub plan: String,
    /// Tenant status
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tenant subscription plans
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantPlan {
    #[default]
    Free,
    Starter,
    Professional,
    Enterprise,
}

impl fmt::Display for TenantPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TenantPlan::Free => write!(f, "free"),
            TenantPlan::Starter => write!(f, "starter"),
            TenantPlan::Professional => write!(f, "professional"),
            TenantPlan::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl FromStr for TenantPlan {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(TenantPlan::Free),
            "starter" => Ok(TenantPlan::Starter),
            "professional" => Ok(TenantPlan::Professional),
            "enterprise" => Ok(TenantPlan::Enterprise),
            _ => Err(format!("Invalid plan: {}", s)),
        }
    }
}

/// Tenant status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    #[default]
    Active,
    Suspended,
    Deleted,
}

impl fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Suspended => write!(f, "suspended"),
            TenantStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl FromStr for TenantStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(TenantStatus::Active),
            "suspended" => Ok(TenantStatus::Suspended),
            "deleted" => Ok(TenantStatus::Deleted),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

impl Tenant {
    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Get the plan as enum
    pub fn get_plan(&self) -> TenantPlan {
        TenantPlan::from_str(&self.plan).unwrap_or_default()
    }

    /// Get the status as enum
    pub fn get_status(&self) -> TenantStatus {
        TenantStatus::from_str(&self.status).unwrap_or_default()
    }
}

/// Extractor for Tenant from request extensions
///
/// This requires the extract_tenant middleware to have run first.
#[axum::async_trait]
impl<S> FromRequestParts<S> for Tenant
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Tenant>()
            .cloned()
            .ok_or(AppError::TenantNotFound)
    }
}

/// Request to create a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub plan: Option<String>,
}

/// Request to update a tenant
#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub plan: Option<String>,
    pub status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_plan_from_str() {
        assert_eq!(TenantPlan::from_str("free").unwrap(), TenantPlan::Free);
        assert_eq!(
            TenantPlan::from_str("PROFESSIONAL").unwrap(),
            TenantPlan::Professional
        );
        assert!(TenantPlan::from_str("invalid").is_err());
    }

    #[test]
    fn test_tenant_status_from_str() {
        assert_eq!(
            TenantStatus::from_str("active").unwrap(),
            TenantStatus::Active
        );
        assert_eq!(
            TenantStatus::from_str("SUSPENDED").unwrap(),
            TenantStatus::Suspended
        );
        assert!(TenantStatus::from_str("invalid").is_err());
    }
}
