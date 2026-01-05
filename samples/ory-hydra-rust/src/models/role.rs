use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// User roles in the DONADONA multi-tenant system
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Platform administrator - manages all tenants
    PlatformAdmin,
    /// Manager - manages engineers, assignments, workflows, and can see proficiency levels
    Manager,
    /// Engineer - can work on assigned incidents/projects
    #[default]
    Engineer,
    /// Reporter - can only report incidents
    Reporter,
}

impl UserRole {
    /// Check if user can manage engineers (hiring, firing, assignments)
    pub fn can_manage_engineers(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can view proficiency levels
    pub fn can_view_proficiency(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can create/manage incidents
    pub fn can_manage_incidents(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can create incidents
    pub fn can_create_incidents(&self) -> bool {
        // All roles can create incidents
        true
    }

    /// Check if user can create projects
    pub fn can_create_projects(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can manage projects
    pub fn can_manage_projects(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can assign engineers to incidents/projects
    pub fn can_assign(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can update workflow status
    pub fn can_update_status(&self) -> bool {
        matches!(
            self,
            UserRole::PlatformAdmin | UserRole::Manager | UserRole::Engineer
        )
    }

    /// Check if user can manage settings (specialties, workflows)
    pub fn can_manage_settings(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can manage finance (salary, transactions)
    pub fn can_manage_finance(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can view finance information
    pub fn can_view_finance(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can manage training
    pub fn can_manage_training(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can create tenants
    pub fn can_create_tenants(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin)
    }

    /// Check if user has platform-level access
    pub fn is_platform_admin(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin)
    }

    /// Check if user is a manager
    pub fn is_manager(&self) -> bool {
        matches!(self, UserRole::Manager)
    }

    /// Check if user is an engineer
    pub fn is_engineer(&self) -> bool {
        matches!(self, UserRole::Engineer)
    }

    /// Check if user is a reporter
    pub fn is_reporter(&self) -> bool {
        matches!(self, UserRole::Reporter)
    }

    /// Check if user can manage team (alias for can_manage_engineers)
    pub fn can_manage_team(&self) -> bool {
        matches!(self, UserRole::PlatformAdmin | UserRole::Manager)
    }

    /// Check if user can work on tasks (engineers and above)
    pub fn can_work_on_tasks(&self) -> bool {
        matches!(
            self,
            UserRole::PlatformAdmin | UserRole::Manager | UserRole::Engineer
        )
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::PlatformAdmin => write!(f, "platform_admin"),
            UserRole::Manager => write!(f, "manager"),
            UserRole::Engineer => write!(f, "engineer"),
            UserRole::Reporter => write!(f, "reporter"),
        }
    }
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "platform_admin" => Ok(UserRole::PlatformAdmin),
            "manager" => Ok(UserRole::Manager),
            "engineer" => Ok(UserRole::Engineer),
            "reporter" => Ok(UserRole::Reporter),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_admin_permissions() {
        let role = UserRole::PlatformAdmin;
        assert!(role.can_create_tenants());
        assert!(role.is_platform_admin());
        assert!(role.can_manage_engineers());
        assert!(role.can_view_proficiency());
        assert!(role.can_manage_incidents());
        assert!(role.can_assign());
        assert!(role.can_manage_finance());
    }

    #[test]
    fn test_manager_permissions() {
        let role = UserRole::Manager;
        assert!(role.is_manager());
        assert!(role.can_manage_engineers());
        assert!(role.can_view_proficiency());
        assert!(role.can_manage_incidents());
        assert!(role.can_create_projects());
        assert!(role.can_assign());
        assert!(role.can_manage_settings());
        assert!(role.can_manage_finance());
        assert!(!role.can_create_tenants());
    }

    #[test]
    fn test_engineer_permissions() {
        let role = UserRole::Engineer;
        assert!(role.is_engineer());
        assert!(role.can_create_incidents());
        assert!(role.can_update_status());
        assert!(!role.can_manage_engineers());
        assert!(!role.can_view_proficiency());
        assert!(!role.can_assign());
        assert!(!role.can_create_projects());
        assert!(!role.can_manage_finance());
    }

    #[test]
    fn test_reporter_permissions() {
        let role = UserRole::Reporter;
        assert!(role.is_reporter());
        assert!(role.can_create_incidents());
        assert!(!role.can_update_status());
        assert!(!role.can_manage_engineers());
        assert!(!role.can_view_proficiency());
        assert!(!role.can_assign());
        assert!(!role.can_manage_finance());
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            UserRole::from_str("platform_admin").unwrap(),
            UserRole::PlatformAdmin
        );
        assert_eq!(UserRole::from_str("manager").unwrap(), UserRole::Manager);
        assert_eq!(UserRole::from_str("ENGINEER").unwrap(), UserRole::Engineer);
        assert_eq!(UserRole::from_str("reporter").unwrap(), UserRole::Reporter);
        assert!(UserRole::from_str("invalid").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(UserRole::PlatformAdmin.to_string(), "platform_admin");
        assert_eq!(UserRole::Manager.to_string(), "manager");
        assert_eq!(UserRole::Engineer.to_string(), "engineer");
        assert_eq!(UserRole::Reporter.to_string(), "reporter");
    }
}
