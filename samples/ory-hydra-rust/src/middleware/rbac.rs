use axum::{extract::Request, middleware::Next, response::Response};
use tracing::instrument;

use crate::error::AppError;
use crate::models::{Claims, Tenant, UserRole};

/// Middleware factory to require specific roles
///
/// Usage:
/// ```ignore
/// .layer(middleware::from_fn(require_role(vec![UserRole::PlatformAdmin])))
/// ```
#[allow(unused)]
pub fn require_role(
    allowed_roles: Vec<UserRole>,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
+ Clone
+ Send
+ 'static {
    move |request: Request, next: Next| {
        let roles = allowed_roles.clone();
        Box::pin(async move { check_role(request, next, roles).await })
    }
}

/// Check if user has one of the allowed roles
#[allow(unused)]
#[instrument(skip(request, next))]
async fn check_role(
    request: Request,
    next: Next,
    allowed_roles: Vec<UserRole>,
) -> Result<Response, AppError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(AppError::AuthenticationFailed(
            "Authentication required".to_string(),
        ))?;

    let user_role = claims.get_role();

    if !allowed_roles.contains(&user_role) {
        return Err(AppError::Forbidden(format!(
            "Insufficient permissions. Required roles: {:?}, your role: {}",
            allowed_roles, user_role
        )));
    }

    Ok(next.run(request).await)
}

/// Middleware to ensure user belongs to the current tenant
///
/// Platform admins have access to all tenants.
/// Other users must belong to the tenant in the request.
#[allow(unused)]
#[instrument(skip(request, next))]
pub async fn require_tenant_membership(request: Request, next: Next) -> Result<Response, AppError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(AppError::AuthenticationFailed(
            "Authentication required".to_string(),
        ))?
        .clone();

    let tenant = request
        .extensions()
        .get::<Tenant>()
        .ok_or(AppError::TenantNotFound)?;

    let user_role = claims.get_role();

    // Platform admins can access all tenants
    if user_role == UserRole::PlatformAdmin {
        return Ok(next.run(request).await);
    }

    // Other users must belong to this tenant
    match claims.tenant_id {
        Some(user_tenant_id) if user_tenant_id == tenant.id => Ok(next.run(request).await),
        _ => Err(AppError::Forbidden(
            "You are not a member of this tenant".to_string(),
        )),
    }
}

/// Check if user can perform action on a specific resource
#[allow(unused)]
pub fn check_permission(
    claims: &Claims,
    required_permission: Permission,
    resource_owner_id: Option<uuid::Uuid>,
) -> Result<(), AppError> {
    let role = claims.get_role();

    match required_permission {
        Permission::ViewIncidents => {
            // Anyone in the tenant can view incidents
            Ok(())
        }
        Permission::ManageIncidents => {
            if role.can_manage_incidents() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage incidents".to_string(),
                ))
            }
        }
        Permission::CreateIncidents => {
            if role.can_create_incidents() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to create incidents".to_string(),
                ))
            }
        }
        Permission::ViewProjects => {
            // Anyone in the tenant can view projects
            Ok(())
        }
        Permission::ManageProjects => {
            if role.can_manage_projects() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage projects".to_string(),
                ))
            }
        }
        Permission::Assign => {
            if role.can_assign() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to assign engineers".to_string(),
                ))
            }
        }
        Permission::ManageEngineers => {
            if role.can_manage_engineers() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage engineers".to_string(),
                ))
            }
        }
        Permission::ViewProficiency => {
            if role.can_view_proficiency() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to view proficiency levels".to_string(),
                ))
            }
        }
        Permission::ManageSettings => {
            if role.can_manage_settings() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage settings".to_string(),
                ))
            }
        }
        Permission::ManageFinance => {
            if role.can_manage_finance() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage finance".to_string(),
                ))
            }
        }
        Permission::ManageTenants => {
            if role.can_create_tenants() {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You don't have permission to manage tenants".to_string(),
                ))
            }
        }
    }
}

/// Permissions for DONADONA RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum Permission {
    // Incidents
    ViewIncidents,
    ManageIncidents,
    CreateIncidents,
    // Projects
    ViewProjects,
    ManageProjects,
    // Engineers
    ManageEngineers,
    ViewProficiency,
    Assign,
    // Settings
    ManageSettings,
    // Finance
    ManageFinance,
    // Tenants
    ManageTenants,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_claims(role: &str, tenant_id: Option<uuid::Uuid>) -> Claims {
        Claims {
            sub: uuid::Uuid::new_v4().to_string(),
            exp: 0,
            iat: 0,
            iss: "test".to_string(),
            aud: vec![],
            email: None,
            role: Some(role.to_string()),
            tenant_id,
        }
    }

    #[test]
    fn test_platform_admin_permissions() {
        let claims = make_claims("platform_admin", None);

        assert!(check_permission(&claims, Permission::ManageTenants, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageEngineers, None).is_ok());
        assert!(check_permission(&claims, Permission::ViewProficiency, None).is_ok());
    }

    #[test]
    fn test_manager_permissions() {
        let tenant_id = uuid::Uuid::new_v4();
        let claims = make_claims("manager", Some(tenant_id));

        assert!(check_permission(&claims, Permission::ManageEngineers, None).is_ok());
        assert!(check_permission(&claims, Permission::ViewProficiency, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageProjects, None).is_ok());
        assert!(check_permission(&claims, Permission::Assign, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageFinance, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageTenants, None).is_err());
    }

    #[test]
    fn test_engineer_permissions() {
        let tenant_id = uuid::Uuid::new_v4();
        let claims = make_claims("engineer", Some(tenant_id));

        assert!(check_permission(&claims, Permission::ViewIncidents, None).is_ok());
        assert!(check_permission(&claims, Permission::CreateIncidents, None).is_ok());
        assert!(check_permission(&claims, Permission::ViewProjects, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageEngineers, None).is_err());
        assert!(check_permission(&claims, Permission::ViewProficiency, None).is_err());
        assert!(check_permission(&claims, Permission::Assign, None).is_err());
    }

    #[test]
    fn test_reporter_permissions() {
        let tenant_id = uuid::Uuid::new_v4();
        let claims = make_claims("reporter", Some(tenant_id));

        assert!(check_permission(&claims, Permission::ViewIncidents, None).is_ok());
        assert!(check_permission(&claims, Permission::CreateIncidents, None).is_ok());
        assert!(check_permission(&claims, Permission::ManageIncidents, None).is_err());
        assert!(check_permission(&claims, Permission::ManageProjects, None).is_err());
        assert!(check_permission(&claims, Permission::ManageFinance, None).is_err());
    }
}
