use axum::{
    extract::{Host, Request, State},
    http::header::HeaderName,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::models::Tenant;
use crate::services::TenantService;
use crate::state::AppState;

/// Header name for tenant slug (used in local development or when subdomains aren't available)
const TENANT_HEADER: &str = "x-tenant-slug";

/// Middleware to extract tenant from subdomain or X-Tenant-Slug header
///
/// Parses the host header to extract the tenant slug from subdomain.
/// For example: shop-a.techmart.io -> "shop-a"
///
/// Alternatively, accepts X-Tenant-Slug header for local development
/// or when subdomains are not available.
///
/// Special subdomains (admin, api, auth, www) are skipped.
#[instrument(skip(state, request, next))]
pub async fn extract_tenant(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Try to get tenant slug from X-Tenant-Slug header first
    let header_name = HeaderName::from_static(TENANT_HEADER);
    let slug_from_header = request
        .headers()
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Then try subdomain
    let subdomain = host.split('.').next().unwrap_or("");
    let slug_from_subdomain = if is_special_subdomain(subdomain)
        || subdomain.is_empty()
        || subdomain == "localhost"
        || subdomain.contains(':')
    {
        None
    } else {
        Some(subdomain.to_string())
    };

    // Use header if available, otherwise use subdomain
    let slug = slug_from_header.or(slug_from_subdomain);

    // If no slug found, continue without tenant
    let Some(slug) = slug else {
        return Ok(next.run(request).await);
    };

    // Skip if slug is empty
    if slug.is_empty() {
        return Ok(next.run(request).await);
    }

    // Get tenant from database
    let tenant_service = TenantService::new(state.pool.clone());
    let tenant = tenant_service.get_by_slug(&slug).await?;

    // Check if tenant is active
    if !tenant.is_active() {
        return Err(AppError::Forbidden("Tenant is not active".to_string()));
    }

    // Add tenant to request extensions
    request.extensions_mut().insert(tenant);

    Ok(next.run(request).await)
}

/// Check if subdomain is a special/reserved one
fn is_special_subdomain(subdomain: &str) -> bool {
    matches!(
        subdomain.to_lowercase().as_str(),
        "admin" | "api" | "auth" | "www" | "mail" | "ftp" | "cdn" | "static"
    )
}

/// Extension trait to get tenant from request
#[allow(unused)]
pub trait TenantExt {
    fn tenant(&self) -> Option<&Tenant>;
    fn require_tenant(&self) -> Result<&Tenant, AppError>;
}

impl<B> TenantExt for axum::http::Request<B> {
    fn tenant(&self) -> Option<&Tenant> {
        self.extensions().get::<Tenant>()
    }

    fn require_tenant(&self) -> Result<&Tenant, AppError> {
        self.tenant().ok_or(AppError::TenantNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_special_subdomain() {
        assert!(is_special_subdomain("admin"));
        assert!(is_special_subdomain("ADMIN"));
        assert!(is_special_subdomain("api"));
        assert!(is_special_subdomain("www"));

        assert!(!is_special_subdomain("shop-a"));
        assert!(!is_special_subdomain("my-store"));
        assert!(!is_special_subdomain(""));
    }
}
