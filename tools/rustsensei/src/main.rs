use axum::Router;
use axum::extract::DefaultBodyLimit;
use rustsensei::api::{self, AppState};
use rustsensei::challenge;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load challenges
    let challenges_dir = PathBuf::from(
        std::env::var("RUSTSENSEI_CHALLENGES_DIR").unwrap_or_else(|_| "challenges".to_string()),
    );
    let challenges = challenge::load_challenges(&challenges_dir)?;
    tracing::info!("Loaded {} challenges", challenges.len());

    let state = Arc::new(AppState { challenges });

    // Static file serving for the frontend
    let static_dir =
        std::env::var("RUSTSENSEI_STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    let app = Router::new()
        .route("/api/analyze", axum::routing::post(api::analyze_handler))
        .route("/api/compile", axum::routing::post(api::compile_handler))
        .route("/api/suggest", axum::routing::post(api::suggest_handler))
        .route("/api/diff", axum::routing::post(api::diff_handler))
        .route(
            "/api/quiz/questions",
            axum::routing::get(api::quiz_questions_handler),
        )
        .route(
            "/api/quiz/check",
            axum::routing::post(api::quiz_check_handler),
        )
        .route(
            "/api/challenges",
            axum::routing::get(api::list_challenges_handler),
        )
        .route(
            "/api/challenges/{id}",
            axum::routing::get(api::get_challenge_handler),
        )
        .with_state(state)
        .fallback_service(ServeDir::new(&static_dir).append_index_html_on_directories(true))
        .layer(DefaultBodyLimit::max(64 * 1024)) // 64KB max request body
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = "127.0.0.1:3000";
    tracing::info!("RustSensei listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
