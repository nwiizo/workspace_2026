use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeOptions {
    pub client_profile: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub resource: ResourceParam,
    pub expected_upstream_client_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceParam {
    Auto,
    Omit,
    Value(String),
}

impl Default for SmokeOptions {
    fn default() -> Self {
        Self::for_profile("generic")
    }
}

impl SmokeOptions {
    pub fn for_profile(profile: &str) -> Self {
        match profile {
            "claude" => Self {
                client_profile: "claude".to_string(),
                client_id: "https://claude.ai/oauth/claude-code-client-metadata".to_string(),
                redirect_uri: "http://localhost/callback".to_string(),
                scopes: vec!["mcp:read".to_string()],
                resource: ResourceParam::Auto,
                expected_upstream_client_id: None,
            },
            "chatgpt" => Self {
                client_profile: "chatgpt".to_string(),
                client_id: "https://chatgpt.com/oauth/client.json".to_string(),
                redirect_uri: "https://chatgpt.com/connector_platform_oauth_redirect".to_string(),
                scopes: vec!["mcp:read".to_string()],
                resource: ResourceParam::Auto,
                expected_upstream_client_id: None,
            },
            _ => Self {
                client_profile: "generic".to_string(),
                client_id: "smoke-client".to_string(),
                redirect_uri: "http://127.0.0.1:9/cb".to_string(),
                scopes: vec!["mcp:read".to_string()],
                resource: ResourceParam::Auto,
                expected_upstream_client_id: None,
            },
        }
    }

    fn resolved_resource(&self, mcp_url: &str) -> Option<String> {
        match &self.resource {
            ResourceParam::Auto => Some(mcp_url.to_string()),
            ResourceParam::Omit => None,
            ResourceParam::Value(resource) => Some(resource.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeReport {
    pub base_url: String,
    pub mcp_path: String,
    pub ran_at: chrono::DateTime<chrono::Utc>,
    pub checks: Vec<CheckResult>,
}

impl SmokeReport {
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub messages: Vec<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub elapsed_ms: u64,
    pub request: RequestLog,
    pub response: ResponseLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseLog {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body_preview: String,
    pub body_json: Option<Value>,
}

pub async fn run(
    base_url: &str,
    mcp_path: &str,
    artifact_dir: &Path,
) -> anyhow::Result<SmokeReport> {
    run_with_options(base_url, mcp_path, artifact_dir, SmokeOptions::default()).await
}

pub async fn run_with_options(
    base_url: &str,
    mcp_path: &str,
    artifact_dir: &Path,
    options: SmokeOptions,
) -> anyhow::Result<SmokeReport> {
    std::fs::create_dir_all(artifact_dir)?;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let base = base_url.trim_end_matches('/').to_string();
    let mcp_url = format!("{base}{mcp_path}");

    let mut checks = Vec::new();

    checks.push(check_prm(&client, &base, &mcp_url).await);
    checks.push(check_unauth_mcp(&client, &base, &mcp_url).await);
    checks.push(check_as_metadata(&client, &base).await);
    checks.push(check_authorize_redirect(&client, &base, &mcp_url, &options).await);

    let report = SmokeReport {
        base_url: base.clone(),
        mcp_path: mcp_path.to_string(),
        ran_at: Utc::now(),
        checks,
    };

    write_artifacts(&report, artifact_dir, &base, &mcp_url)?;

    Ok(report)
}

async fn check_prm(client: &reqwest::Client, base: &str, mcp_url: &str) -> CheckResult {
    let url = format!("{base}/.well-known/oauth-protected-resource");
    fold(
        "GET /.well-known/oauth-protected-resource",
        "GET",
        &url,
        Vec::new(),
        || client.get(&url).send(),
        |status, _headers, body_json| {
            let mut msgs = Vec::new();
            let mut ok = true;
            if status != 200 {
                msgs.push(format!("expected 200, got {status}"));
                ok = false;
            }
            if let Some(json) = body_json {
                if json.get("resource").and_then(|v| v.as_str()) != Some(mcp_url) {
                    msgs.push(format!(
                        "resource != {mcp_url} (got {:?})",
                        json.get("resource")
                    ));
                    ok = false;
                }
                let auth_servers = json
                    .get("authorization_servers")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if !auth_servers.iter().any(|v| v.as_str() == Some(base)) {
                    msgs.push(format!("authorization_servers does not contain {base}"));
                    ok = false;
                }
            } else {
                msgs.push("body is not JSON".into());
                ok = false;
            }
            (ok, msgs)
        },
    )
    .await
}

async fn check_unauth_mcp(client: &reqwest::Client, base: &str, mcp_url: &str) -> CheckResult {
    fold(
        "POST <mcp> without Authorization",
        "POST",
        mcp_url,
        Vec::new(),
        || client.post(mcp_url).body("{}".to_string()).send(),
        |status, headers, _body| {
            let mut msgs = Vec::new();
            let mut ok = true;
            if status != 401 {
                msgs.push(format!("expected 401, got {status}"));
                ok = false;
            }
            let www = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("www-authenticate"))
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            if !www.to_ascii_lowercase().starts_with("bearer") {
                msgs.push(format!("WWW-Authenticate not Bearer: {www:?}"));
                ok = false;
            }
            if !www.contains(r#"error="invalid_token""#) {
                msgs.push("WWW-Authenticate missing error=\"invalid_token\"".into());
                ok = false;
            }
            let expected_prm = format!("{base}/.well-known/oauth-protected-resource");
            if !www.contains(&format!(r#"resource_metadata="{expected_prm}""#)) {
                msgs.push(format!(
                    "WWW-Authenticate missing resource_metadata=\"{expected_prm}\""
                ));
                ok = false;
            }
            (ok, msgs)
        },
    )
    .await
}

async fn check_as_metadata(client: &reqwest::Client, base: &str) -> CheckResult {
    let url = format!("{base}/.well-known/oauth-authorization-server");
    fold(
        "GET /.well-known/oauth-authorization-server",
        "GET",
        &url,
        Vec::new(),
        || client.get(&url).send(),
        |status, _headers, body_json| {
            let mut msgs = Vec::new();
            let mut ok = true;
            if status != 200 {
                msgs.push(format!("expected 200, got {status}"));
                ok = false;
            }
            let Some(json) = body_json else {
                return (false, vec!["body is not JSON".into()]);
            };
            for key in ["issuer", "authorization_endpoint", "token_endpoint"] {
                if json.get(key).is_none() {
                    msgs.push(format!("missing field: {key}"));
                    ok = false;
                }
            }
            if json
                .get("client_id_metadata_document_supported")
                .and_then(|v| v.as_bool())
                != Some(true)
            {
                msgs.push("client_id_metadata_document_supported should be true".into());
                ok = false;
            }
            let methods = json
                .get("code_challenge_methods_supported")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if !methods.iter().any(|v| v.as_str() == Some("S256")) {
                msgs.push("code_challenge_methods_supported must include S256".into());
                ok = false;
            }
            (ok, msgs)
        },
    )
    .await
}

async fn check_authorize_redirect(
    client: &reqwest::Client,
    base: &str,
    mcp_url: &str,
    options: &SmokeOptions,
) -> CheckResult {
    let verifier = crate::pkce::random_verifier();
    let challenge = crate::pkce::challenge_s256(&verifier);
    let mut url = url::Url::parse(&format!("{base}/oauth/authorize")).expect("valid base url");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", &options.client_id);
        query.append_pair("redirect_uri", &options.redirect_uri);
        query.append_pair("code_challenge", &challenge);
        query.append_pair("code_challenge_method", "S256");
        query.append_pair("state", "smoke");
        if !options.scopes.is_empty() {
            query.append_pair("scope", &options.scopes.join(" "));
        }
        if let Some(resource) = options.resolved_resource(mcp_url) {
            query.append_pair("resource", &resource);
        }
    }
    let url = url.to_string();
    fold(
        "GET /oauth/authorize redirect",
        "GET",
        &url,
        Vec::new(),
        || client.get(&url).send(),
        |status, headers, _body| {
            let mut msgs = Vec::new();
            let mut ok = true;
            if !matches!(status, 302 | 303 | 307) {
                msgs.push(format!("expected 302/303/307, got {status}"));
                ok = false;
            }
            let loc = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                .map(|(_, v)| v.clone());
            match loc {
                Some(loc) if loc.starts_with(&options.redirect_uri) => {
                    if !loc.contains("code=") {
                        msgs.push("redirect missing code= param".into());
                        ok = false;
                    }
                    if !loc.contains("state=smoke") {
                        msgs.push("redirect did not preserve state".into());
                        ok = false;
                    }
                }
                Some(loc) => {
                    let (redirect_ok, redirect_msgs) =
                        assert_upstream_authorize_redirect(&loc, options, mcp_url, &challenge);
                    if !redirect_ok {
                        ok = false;
                    }
                    msgs.extend(redirect_msgs);
                }
                None => {
                    msgs.push("missing Location header".into());
                    ok = false;
                }
            }
            (ok, msgs)
        },
    )
    .await
}

fn assert_upstream_authorize_redirect(
    location: &str,
    options: &SmokeOptions,
    mcp_url: &str,
    challenge: &str,
) -> (bool, Vec<String>) {
    let mut msgs = Vec::new();
    let mut ok = true;
    let parsed = match url::Url::parse(location) {
        Ok(url) => url,
        Err(err) => {
            return (
                false,
                vec![format!(
                    "Location is neither callback nor parseable URL: {err}"
                )],
            );
        }
    };
    let query: std::collections::BTreeMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    if query.get("state").map(String::as_str) != Some("smoke") {
        msgs.push("redirect did not preserve state=smoke".into());
        ok = false;
    }
    if query.get("redirect_uri").map(String::as_str) != Some(options.redirect_uri.as_str()) {
        msgs.push(format!(
            "redirect_uri mismatch in upstream Location: got {:?}",
            query.get("redirect_uri")
        ));
        ok = false;
    }
    if query.get("code_challenge").map(String::as_str) != Some(challenge) {
        msgs.push("code_challenge mismatch in upstream Location".into());
        ok = false;
    }
    if query.get("code_challenge_method").map(String::as_str) != Some("S256") {
        msgs.push("code_challenge_method is not S256 in upstream Location".into());
        ok = false;
    }
    if let Some(resource) = options.resolved_resource(mcp_url)
        && query.get("resource").map(String::as_str) != Some(resource.as_str())
    {
        msgs.push(format!(
            "resource mismatch in upstream Location: got {:?}, expected {resource}",
            query.get("resource")
        ));
        ok = false;
    }
    if let Some(expected) = options.expected_upstream_client_id.as_deref()
        && query.get("client_id").map(String::as_str) != Some(expected)
    {
        msgs.push(format!(
            "client_id mismatch in upstream Location: got {:?}, expected {expected}",
            query.get("client_id")
        ));
        ok = false;
    }
    (ok, msgs)
}

async fn fold<F, Fut>(
    name: &str,
    method: &str,
    url: &str,
    req_headers: Vec<(String, String)>,
    send: F,
    assertion: impl FnOnce(u16, &[(String, String)], Option<&Value>) -> (bool, Vec<String>),
) -> CheckResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let started_at = chrono::Utc::now();
    let start_instant = std::time::Instant::now();
    let result = send().await;
    let elapsed_ms = start_instant.elapsed().as_millis() as u64;
    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let bytes = resp.bytes().await.unwrap_or_default();
            let body_str = String::from_utf8_lossy(&bytes).to_string();
            let body_json: Option<Value> = serde_json::from_str(&body_str).ok();
            let (passed, messages) = assertion(status, &headers, body_json.as_ref());
            let preview = if body_str.len() > 4096 {
                format!("{}…", &body_str[..4096])
            } else {
                body_str
            };
            CheckResult {
                name: name.to_string(),
                passed,
                messages,
                started_at,
                elapsed_ms,
                request: RequestLog {
                    method: method.to_string(),
                    url: url.to_string(),
                    headers: req_headers,
                },
                response: ResponseLog {
                    status,
                    headers,
                    body_preview: preview,
                    body_json,
                },
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            messages: vec![format!("request failed: {e}")],
            started_at,
            elapsed_ms,
            request: RequestLog {
                method: method.to_string(),
                url: url.to_string(),
                headers: req_headers,
            },
            response: ResponseLog {
                status: 0,
                headers: Vec::new(),
                body_preview: String::new(),
                body_json: None,
            },
        },
    }
}

fn write_artifacts(
    report: &SmokeReport,
    artifact_dir: &Path,
    base: &str,
    mcp_url: &str,
) -> anyhow::Result<()> {
    let network_path: PathBuf = artifact_dir.join("network.json");
    std::fs::write(&network_path, serde_json::to_string_pretty(report)?)?;

    let report_md_path = artifact_dir.join("report.md");
    std::fs::write(&report_md_path, render_markdown(report))?;

    let curl_path = artifact_dir.join("curl-equivalent.sh");
    std::fs::write(&curl_path, render_curl(report, base, mcp_url))?;

    let har_path = artifact_dir.join("network.har");
    std::fs::write(
        &har_path,
        serde_json::to_string_pretty(&render_har(report))?,
    )?;

    Ok(())
}

/// Build an HAR 1.2 document from the smoke report. No redaction: this is a
/// local-only artifact and full fidelity is the whole point.
fn render_har(report: &SmokeReport) -> Value {
    let entries: Vec<Value> = report.checks.iter().map(har_entry).collect();
    serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {
                "name": "remote-mcp-devkit",
                "version": env!("CARGO_PKG_VERSION")
            },
            "browser": {
                "name": "remote-mcp-devkit-smoke",
                "version": env!("CARGO_PKG_VERSION")
            },
            "pages": [],
            "entries": entries,
        }
    })
}

fn har_entry(c: &CheckResult) -> Value {
    let url = &c.request.url;
    let parsed = url::Url::parse(url).ok();
    let query_string: Vec<Value> = parsed
        .as_ref()
        .map(|u| {
            u.query_pairs()
                .map(|(k, v)| {
                    serde_json::json!({
                        "name": k.to_string(),
                        "value": v.to_string()
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let request_headers: Vec<Value> = c
        .request
        .headers
        .iter()
        .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
        .collect();
    let response_headers: Vec<Value> = c
        .response
        .headers
        .iter()
        .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
        .collect();
    let content_type = c
        .response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let body_text = &c.response.body_preview;
    let redirect_url = c
        .response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("location"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    serde_json::json!({
        "_devkit_check_name": c.name,
        "_devkit_check_passed": c.passed,
        "_devkit_check_messages": c.messages,
        "startedDateTime": c.started_at.to_rfc3339(),
        "time": c.elapsed_ms,
        "request": {
            "method": c.request.method,
            "url": c.request.url,
            "httpVersion": "HTTP/1.1",
            "cookies": [],
            "headers": request_headers,
            "queryString": query_string,
            "headersSize": -1,
            "bodySize": -1
        },
        "response": {
            "status": c.response.status,
            "statusText": "",
            "httpVersion": "HTTP/1.1",
            "cookies": [],
            "headers": response_headers,
            "content": {
                "size": body_text.len(),
                "mimeType": content_type,
                "text": body_text
            },
            "redirectURL": redirect_url,
            "headersSize": -1,
            "bodySize": body_text.len() as i64
        },
        "cache": {},
        "timings": {
            "send": 0,
            "wait": c.elapsed_ms,
            "receive": 0
        }
    })
}

fn render_markdown(report: &SmokeReport) -> String {
    let mut out = String::new();
    out.push_str("# remote-mcp-devkit smoke report\n\n");
    out.push_str(&format!("- base_url: `{}`\n", report.base_url));
    out.push_str(&format!("- mcp_path: `{}`\n", report.mcp_path));
    out.push_str(&format!("- ran_at: `{}`\n", report.ran_at));
    out.push_str(&format!(
        "- result: **{}**\n\n",
        if report.passed() { "PASS" } else { "FAIL" }
    ));

    for c in &report.checks {
        out.push_str(&format!(
            "## {} — {}\n\n",
            c.name,
            if c.passed { "PASS" } else { "FAIL" }
        ));
        out.push_str(&format!(
            "- request: `{} {}`\n",
            c.request.method, c.request.url
        ));
        out.push_str(&format!("- response status: `{}`\n", c.response.status));
        if !c.messages.is_empty() {
            out.push_str("\n### findings\n\n");
            for m in &c.messages {
                out.push_str(&format!("- {m}\n"));
            }
        }
        out.push('\n');
    }
    out
}

fn render_curl(report: &SmokeReport, base: &str, mcp_url: &str) -> String {
    let authorize_url = report
        .checks
        .iter()
        .find(|check| check.name == "GET /oauth/authorize redirect")
        .map(|check| check.request.url.as_str())
        .unwrap_or("$BASE/oauth/authorize");
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
# Manual reproduction of remote-mcp-devkit smoke checks.
# Uses --insecure for self-signed TLS; remove if your trust store has the cert.

BASE="{base}"
MCP="{mcp_url}"

echo '== PRM =='
curl --insecure -sS -o /dev/stdout -w '\nHTTP %{{http_code}}\n' "$BASE/.well-known/oauth-protected-resource"

echo '== unauth MCP =='
curl --insecure -sS -o /dev/null -D - -X POST "$MCP" -d '{{}}' | head -n 20

echo '== AS metadata =='
curl --insecure -sS "$BASE/.well-known/oauth-authorization-server"

echo '== authorize redirect =='
curl --insecure -sS -o /dev/null -D - \
  "{authorize_url}"
"#,
        base = base,
        mcp_url = mcp_url,
        authorize_url = authorize_url
    )
}
