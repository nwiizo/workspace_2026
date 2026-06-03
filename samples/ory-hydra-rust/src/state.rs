use sqlx::PgPool;

use crate::db::TenantSchemaManager;
use crate::services::{
    AuthService, BffConfig, BffService, HydraClient, JwtService, TenantService, UserService,
};

/// Application state shared across handlers
#[allow(unused)]
pub struct AppState {
    pub hydra: HydraClient,
    pub bff: BffService,
    pub auth: AuthService,
    pub jwt: JwtService,
    pub pool: PgPool,
    pub tenant_schema: TenantSchemaManager,
    pub tenant: TenantService,
    pub user: UserService,
}

impl AppState {
    /// Create a new application state
    pub fn new(
        hydra_admin_url: String,
        bff_config: BffConfig,
        jwt_secret: &[u8],
        issuer: String,
        pool: PgPool,
    ) -> Self {
        let tenant_schema = TenantSchemaManager::new(pool.clone());
        let tenant = TenantService::new(pool.clone());
        let user = UserService::new(pool.clone());

        Self {
            hydra: HydraClient::new(hydra_admin_url),
            bff: BffService::new(bff_config),
            auth: AuthService::new(pool.clone()),
            jwt: JwtService::new(jwt_secret, issuer, vec!["api.example.com".to_string()]),
            pool,
            tenant_schema,
            tenant,
            user,
        }
    }
}
