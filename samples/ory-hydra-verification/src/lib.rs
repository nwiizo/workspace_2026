//! Ory Hydra Authentication Verification
//!
//! blog-02-implementation.md の技術検証用プロジェクト
//! 認証サービスのパターンとテスト手法を検証する

pub mod auth;
pub mod error;
pub mod handlers;
pub mod hydra;
pub mod models;

pub use auth::AuthService;
pub use error::AppError;
pub use handlers::AppState;
pub use hydra::HydraService;
