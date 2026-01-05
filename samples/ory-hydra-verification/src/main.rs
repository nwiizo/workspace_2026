//! Ory Hydra Login/Consent Provider Server
//!
//! OAuth2認証フローを検証するためのサーバー

use axum::{routing::get, routing::post, Router};
use std::env;
use std::net::SocketAddr;

use ory_hydra_verification::{AuthService, AppState, HydraService};
use ory_hydra_verification::handlers::{
    consent_page, consent_submit, health, login_page, login_submit, logout_handler,
};

#[tokio::main]
async fn main() {
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a number");

    let hydra_admin_url =
        env::var("HYDRA_ADMIN_URL").unwrap_or_else(|_| "http://localhost:4445".to_string());

    let auth = AuthService::new();
    let hydra = HydraService::new(&hydra_admin_url);

    // デモ用ユーザーを登録
    auth.register("demo@example.com", "password123")
        .await
        .expect("Failed to create demo user");
    println!("Demo user created: demo@example.com / password123");

    let state = AppState { auth, hydra };

    let app = Router::new()
        .route("/health", get(health))
        .route("/login", get(login_page))
        .route("/login", post(login_submit))
        .route("/consent", get(consent_page))
        .route("/consent", post(consent_submit))
        .route("/logout", get(logout_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Login Provider listening on http://{}", addr);
    println!("Hydra Admin URL: {}", hydra_admin_url);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
