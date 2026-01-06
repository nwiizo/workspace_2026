mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;
mod state;

use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, patch, post, put},
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::pool::{create_pool, run_migrations};
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ory_hydra_rust=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env();
    info!("Starting DONADONA server with config: {:?}", config);

    // Create database connection pool
    let pool = create_pool(&config.database_url).await?;

    // Run database migrations
    run_migrations(&pool).await?;

    // Create application state
    let state = Arc::new(AppState::new(
        config.hydra_admin_url.clone(),
        config.jwt_secret.as_bytes(),
        config.jwt_issuer.clone(),
        pool,
    ));

    // Initialize auth service (seed demo user)
    state.auth.init().await?;

    // Platform admin API routes (requires authentication)
    let platform_api = Router::new()
        .route("/tenants", post(handlers::platform::create_tenant))
        .route("/tenants", get(handlers::platform::list_tenants))
        .route("/tenants/{tenant_id}", get(handlers::platform::get_tenant))
        .route(
            "/tenants/{tenant_id}",
            put(handlers::platform::update_tenant),
        )
        .route(
            "/tenants/{tenant_id}",
            delete(handlers::platform::delete_tenant),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // DONADONA tenant API routes (requires authentication + tenant context)
    let tenant_api = Router::new()
        // Incidents
        .route("/incidents", post(handlers::tenant::create_incident))
        .route("/incidents", get(handlers::tenant::list_incidents))
        .route(
            "/incidents/stats",
            get(handlers::tenant::get_incident_stats),
        )
        .route("/incidents/{id}", get(handlers::tenant::get_incident))
        .route("/incidents/{id}", put(handlers::tenant::update_incident))
        .route("/incidents/{id}", delete(handlers::tenant::delete_incident))
        .route(
            "/incidents/{id}/assign",
            post(handlers::tenant::assign_incident),
        )
        .route(
            "/incidents/{id}/status",
            patch(handlers::tenant::change_incident_status),
        )
        // Projects
        .route("/projects", post(handlers::tenant::create_project))
        .route("/projects", get(handlers::tenant::list_projects))
        .route("/projects/stats", get(handlers::tenant::get_project_stats))
        .route("/projects/{id}", get(handlers::tenant::get_project))
        .route("/projects/{id}", put(handlers::tenant::update_project))
        .route("/projects/{id}", delete(handlers::tenant::delete_project))
        .route(
            "/projects/{id}/assign",
            post(handlers::tenant::assign_project),
        )
        .route(
            "/projects/{id}/status",
            patch(handlers::tenant::change_project_status),
        )
        .route(
            "/projects/{id}/hours",
            patch(handlers::tenant::update_project_hours),
        )
        // Engineers
        .route("/engineers", get(handlers::tenant::list_engineers))
        .route(
            "/engineers/salary-total",
            get(handlers::tenant::get_total_salary),
        )
        .route("/engineers/{id}", get(handlers::tenant::get_engineer))
        .route(
            "/engineers/{id}/specialties",
            post(handlers::tenant::add_specialty),
        )
        .route(
            "/engineers/{id}/fire",
            post(handlers::tenant::fire_engineer),
        )
        // Recruitment
        .route(
            "/recruitment/candidates",
            get(handlers::tenant::list_candidates),
        )
        .route(
            "/recruitment/candidates/{id}",
            get(handlers::tenant::get_candidate),
        )
        .route("/recruitment/refresh", post(handlers::tenant::refresh_pool))
        .route("/recruitment/hire", post(handlers::tenant::hire_candidate))
        .route(
            "/recruitment/status",
            get(handlers::tenant::get_refresh_status),
        )
        // Game/Leaderboard
        .route("/leaderboard", get(handlers::tenant::get_leaderboard))
        .route(
            "/leaderboard/level",
            get(handlers::tenant::get_level_leaderboard),
        )
        .route(
            "/leaderboard/revenue",
            get(handlers::tenant::get_revenue_leaderboard),
        )
        .route(
            "/leaderboard/incidents",
            get(handlers::tenant::get_incidents_leaderboard),
        )
        .route(
            "/leaderboard/projects",
            get(handlers::tenant::get_projects_leaderboard),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::extract_tenant,
        ));

    // Build the router
    let app = Router::new()
        // Home and health
        .route("/", get(handlers::home))
        .route("/health", get(handlers::health))
        // Hydra Login Provider
        .route("/login", get(handlers::show_login))
        .route("/login", post(handlers::handle_login))
        // Hydra Consent Provider
        .route("/consent", get(handlers::show_consent))
        .route("/consent", post(handlers::handle_consent))
        // Hydra Logout Provider
        .route("/logout", get(handlers::handle_logout))
        .route("/logout/confirm", get(handlers::confirm_logout))
        .route("/logout/cancel", get(handlers::cancel_logout))
        // OAuth2 Callback
        .route("/callback", get(handlers::handle_callback))
        // Dashboard
        .route("/dashboard", get(handlers::show_dashboard))
        // Error page
        .route("/error", get(handlers::show_error))
        // HTML Pages for viewing data
        .route("/pages/tenants", get(handlers::tenants_page))
        .route("/pages/tenants/new", get(handlers::new_tenant_page))
        .route("/pages/tenants/create", post(handlers::create_tenant_page))
        // API endpoints - authentication
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/refresh", post(handlers::refresh))
        .route("/api/auth/logout", post(handlers::logout))
        // API v1 - Platform management
        .nest("/api/v1", platform_api)
        // API v1 - Tenant operations (DONADONA)
        .nest("/api/v1/tenant", tenant_api)
        // Add CORS layer (allow frontend origin)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // Add tracing layer
        .layer(TraceLayer::new_for_http())
        // Add shared state
        .with_state(state);

    // Start the server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("DONADONA server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
