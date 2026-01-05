use sqlx::PgPool;

use crate::db::TenantSchemaManager;
use crate::services::{AuthService, HydraClient, JwtService, TenantService, UserService};

/// Application state shared across handlers
#[allow(unused)]
pub struct AppState {
    pub hydra: HydraClient,
    pub auth: AuthService,
    pub jwt: JwtService,
    pub pool: PgPool,
    pub tenant_schema: TenantSchemaManager,
    pub tenant: TenantService,
    pub user: UserService,
}

impl AppState {
    /// Create a new application state
    pub fn new(hydra_admin_url: String, jwt_secret: &[u8], issuer: String, pool: PgPool) -> Self {
        let tenant_schema = TenantSchemaManager::new(pool.clone());
        let tenant = TenantService::new(pool.clone());
        let user = UserService::new(pool.clone());

        Self {
            hydra: HydraClient::new(hydra_admin_url),
            auth: AuthService::new(pool.clone()),
            jwt: JwtService::new(jwt_secret, issuer, vec!["api.example.com".to_string()]),
            pool,
            tenant_schema,
            tenant,
            user,
        }
    }
}
