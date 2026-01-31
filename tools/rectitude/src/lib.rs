//! # Rectitude
//!
//! General-purpose E2E scenario testing library for security testing and more.
//!
//! ## Overview
//!
//! Rectitude provides a fluent API for:
//! - **Scenario-based testing** - Chain HTTP requests with shared state
//! - **Security payloads** - SQL injection, XSS, XXE, JWT manipulation, and more
//! - **Response analysis** - Extract data with JSON path, regex, and assertions
//! - **Session management** - Automatic cookie and JWT token handling
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rectitude::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     Scenario::new("User Authentication Flow")
//!         .base_url("http://localhost:3000")
//!         .step("Login", |ctx| Box::pin(async move {
//!             let resp = ctx.post("/api/login")
//!                 .json(&serde_json::json!({
//!                     "username": "admin",
//!                     "password": "password123"
//!                 }))
//!                 .send()
//!                 .await?;
//!
//!             ctx.assert_status(&resp, 200)?;
//!             ctx.set_var("token", resp.json_path("$.token")?.to_string());
//!             Ok(StepResult::success())
//!         }))
//!         .step("Access Protected Resource", |ctx| Box::pin(async move {
//!             let token = ctx.get_var("token")?;
//!             let resp = ctx.get("/api/protected")
//!                 .bearer_auth(&token)
//!                 .send()
//!                 .await?;
//!
//!             ctx.assert_status(&resp, 200)?;
//!             Ok(StepResult::success())
//!         }))
//!         .run()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Security Testing Example
//!
//! ```rust,ignore
//! use rectitude::prelude::*;
//! use rectitude::payloads::sqli;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     Scenario::new("SQL Injection Test")
//!         .base_url("http://localhost:3000")
//!         .step("Test SQLi Login Bypass", |ctx| Box::pin(async move {
//!             for payload in sqli::auth_bypass_payloads() {
//!                 let resp = ctx.post("/api/login")
//!                     .json(&serde_json::json!({
//!                         "username": payload.payload,
//!                         "password": "anything"
//!                     }))
//!                     .send()
//!                     .await?;
//!
//!                 if resp.is_success() {
//!                     println!("SQLi bypass found: {}", payload.name);
//!                     return Ok(StepResult::success());
//!                 }
//!             }
//!             Ok(StepResult::failed("No SQLi bypass found"))
//!         }))
//!         .run()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

pub mod assertions;
pub mod client;
pub mod clients;
pub mod config;
pub mod ctf;
pub mod error;
pub mod extractors;
pub mod fuzzing;
pub mod helpers;
pub mod payloads;
pub mod reporter;
pub mod resource;
pub mod scenario;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::assertions::{Expect, expect};
    pub use crate::client::{SecurityClient, SecurityClientBuilder, SecurityResponse};
    pub use crate::error::{Error, Result};
    pub use crate::extractors::{ExtractBuilder, Extractor, JsonExtractor, RegexExtractor};
    pub use crate::payloads::encoding::{
        base64_decode, base64_encode, hex_decode, hex_encode, url_encode,
    };
    pub use crate::resource::{Resource, ResourceManager};
    pub use crate::scenario::{Scenario, ScenarioContext, ScenarioResult, Step, StepResult};

    /// Quick success result
    pub fn ok() -> crate::error::Result<StepResult> {
        Ok(StepResult::success())
    }

    /// Quick success with message
    pub fn ok_with(msg: impl Into<String>) -> crate::error::Result<StepResult> {
        Ok(StepResult::success_with_message(msg))
    }

    /// Quick failure
    pub fn fail(msg: impl Into<String>) -> crate::error::Result<StepResult> {
        Ok(StepResult::failed(msg))
    }

    /// Quick skip
    pub fn skip(reason: impl Into<String>) -> crate::error::Result<StepResult> {
        Ok(StepResult::skipped(reason))
    }
}

pub use config::{RectitudeConfig, TagFilter};
pub use ctf::{ChallengeProgress, ChallengeVerifier};
pub use error::{Error, Result};
pub use helpers::{
    auth_helpers, captcha_helpers, coupon_helpers, file_disclosure, forgery_helpers,
    header_helpers, idor_helpers, omission_helpers, osint_helpers, sqli_helpers, upload_helpers,
    validation_helpers,
};
pub use reporter::{ReportBuilder, ReportFormat, TestReport};

// Re-export the clients module types
pub use clients::{
    Client, ClientConfig, ClientError, ClientResult, HttpClient, HttpRequest, HttpResponse,
};
