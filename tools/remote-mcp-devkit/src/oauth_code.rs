//! `oauth-code` — the niche-furniture slot between `smoke` passing and a user
//! pasting the MCP URL into Claude Desktop.
//!
//! It does exactly one thing: from a base_url, walk PRM → AS metadata, build a
//! PKCE-protected authorize URL, capture the redirect on a local listener,
//! exchange the code for a token, and write a redacted report + HAR.
//!
//! Intentionally **no browser automation, no selectors, no screenshots, no
//! retries**. The user clicks; the tool catches.

use crate::pkce;
use axum::{Router, extract::Query, response::Html, routing::get};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
use url::Url;

#[derive(Debug, Clone)]
pub struct OAuthCodeOptions {
    pub base_url: String,
    pub mcp_path: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub callback_mode: CallbackCaptureMode,
    pub scopes: Vec<String>,
    pub resource: Option<String>,
    pub timeout: Duration,
    /// When true, attempt to open the user's default browser.
    pub open_browser: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCodeReport {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub callback_listen_uri: Option<String>,
    pub callback_capture_mode: String,
    pub callback: Option<CallbackPayload>,
    pub token_summary: Option<TokenSummary>,
    pub jwt: Option<JwtSummary>,
    pub failure: Option<String>,
    pub elapsed_ms: u64,
}

impl OAuthCodeReport {
    pub fn passed(&self) -> bool {
        self.failure.is_none() && self.token_summary.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackCaptureMode {
    Listener,
    Manual,
}

impl FromStr for CallbackCaptureMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "listener" => Ok(Self::Listener),
            "manual" => Ok(Self::Manual),
            other => anyhow::bail!("callback_mode must be `listener` or `manual` (got {other})"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackPayload {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub raw_query: String,
}

/// Token-endpoint response with secret values stripped. Original lengths are
/// preserved so debugging can tell "no token" apart from "redacted token".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
    pub has_access_token: bool,
    pub access_token_len: usize,
    pub has_refresh_token: bool,
    pub has_id_token: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtSummary {
    pub which: String,
    pub header: Value,
    pub payload: Value,
}

#[derive(Clone, Default)]
struct CallbackState {
    payload: Arc<Mutex<Option<CallbackPayload>>>,
    notifier: Arc<tokio::sync::Notify>,
}

/// Test / agent hooks for `run`. Default values give CLI behavior (stdin reader,
/// no extra side channels). Tests inject channels to drive the flow without
/// touching real stdin or opening a browser.
#[derive(Default)]
pub struct OAuthCodeHooks {
    /// If set, run sends the constructed authorize URL on this channel right
    /// after building it (and before waiting on the callback). Useful for tests
    /// that need to drive the AS themselves once they know the URL.
    pub authorize_url_tx: Option<tokio::sync::oneshot::Sender<String>>,
    /// If set in Manual mode, run awaits the callback URL on this receiver
    /// instead of reading from stdin.
    pub manual_callback_rx: Option<tokio::sync::oneshot::Receiver<String>>,
}

/// Run the OAuth code flow against `opts.base_url` and persist a report.
///
/// Returns `Ok(report)` for both success and OAuth-level failure (e.g. `error=access_denied`);
/// `Err` is reserved for transport / configuration issues.
pub async fn run(opts: OAuthCodeOptions, artifact_dir: &Path) -> anyhow::Result<OAuthCodeReport> {
    run_with_hooks(opts, OAuthCodeHooks::default(), artifact_dir).await
}

pub async fn run_with_hooks(
    opts: OAuthCodeOptions,
    hooks: OAuthCodeHooks,
    artifact_dir: &Path,
) -> anyhow::Result<OAuthCodeReport> {
    std::fs::create_dir_all(artifact_dir)?;
    let start_instant = std::time::Instant::now();

    // 1. Discover PRM → AS metadata.
    let http = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let base = opts.base_url.trim_end_matches('/');
    let prm_url = format!("{base}/.well-known/oauth-protected-resource");
    let prm: Value = http
        .get(&prm_url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("PRM fetch failed: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("PRM parse failed: {e}"))?;
    let as_base = prm
        .get("authorization_servers")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or(base)
        .trim_end_matches('/')
        .to_string();
    let as_meta_url = format!("{as_base}/.well-known/oauth-authorization-server");
    let as_meta: Value = http
        .get(&as_meta_url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("AS metadata fetch failed: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("AS metadata parse failed: {e}"))?;
    let authorize_endpoint = as_meta
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("AS metadata missing authorization_endpoint"))?
        .to_string();
    let token_endpoint = as_meta
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("AS metadata missing token_endpoint"))?
        .to_string();

    // 2. Build authorize URL with PKCE.
    let verifier = pkce::random_verifier();
    let challenge = pkce::challenge_s256(&verifier);
    let state_token = uuid::Uuid::new_v4().simple().to_string();
    let resolved_resource = match &opts.resource {
        Some(r) if r == "auto" => Some(format!("{base}{}", opts.mcp_path)),
        Some(r) => Some(r.clone()),
        None => None,
    };
    let mut authorize_url = url::Url::parse(&authorize_endpoint)?;
    {
        let mut q = authorize_url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", &opts.client_id);
        q.append_pair("redirect_uri", &opts.redirect_uri);
        q.append_pair("code_challenge", &challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("state", &state_token);
        if !opts.scopes.is_empty() {
            q.append_pair("scope", &opts.scopes.join(" "));
        }
        if let Some(r) = resolved_resource.as_deref() {
            q.append_pair("resource", r);
        }
    }
    let authorize_url = authorize_url.to_string();

    // Tests can pull the URL right away to drive the AS themselves; the channel
    // is fire-and-forget so a closed receiver does not abort the flow.
    if let Some(tx) = hooks.authorize_url_tx {
        let _ = tx.send(authorize_url.clone());
    }

    // Human progress to stderr; stdout stays reserved for the final JSON.
    eprintln!();
    eprintln!("Open this URL in a browser (logged-in as the test user):");
    eprintln!();
    eprintln!("    {authorize_url}");
    eprintln!();

    // 3. Capture callback.
    let capture = match opts.callback_mode {
        CallbackCaptureMode::Listener => {
            let binding = CallbackBinding::from_redirect_uri(&opts.redirect_uri)?;
            let listen_addr = binding.listen_addr;
            let callback_path = binding.path;
            let callback_listen_uri = binding.listen_uri;
            let callback_capture_mode = binding.capture_mode;
            let state = CallbackState::default();
            let listener_state = state.clone();
            let listener_path = callback_path.clone();
            let shutdown_notify = Arc::new(tokio::sync::Notify::new());
            let shutdown_signal = shutdown_notify.clone();
            let listener = tokio::net::TcpListener::bind(listen_addr)
                .await
                .map_err(|err| {
                    anyhow::anyhow!(
                        "failed to bind callback listener on {callback_listen_uri}: {err}. \
                         Use `--callback-mode manual` when the real redirect_uri uses a privileged or occupied port."
                    )
                })?;
            let server_handle = tokio::spawn(async move {
                let app = Router::new()
                    .route(&listener_path, get(callback_handler))
                    .with_state(listener_state);
                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        shutdown_signal.notified().await;
                    })
                    .await;
            });
            eprintln!("Listening for callback on {callback_listen_uri} …");
            eprintln!();

            if opts.open_browser {
                let _ = open_in_browser(&authorize_url);
            }

            let payload = tokio::time::timeout(opts.timeout, async {
                loop {
                    state.notifier.notified().await;
                    if let Some(p) = state.payload.lock().await.clone() {
                        return p;
                    }
                }
            })
            .await;
            shutdown_notify.notify_one();
            let _ = server_handle.await;
            CallbackCapture {
                payload: payload
                    .map_err(|_| format!("timeout waiting for callback after {:?}", opts.timeout)),
                listen_uri: Some(callback_listen_uri),
                mode: callback_capture_mode,
            }
        }
        CallbackCaptureMode::Manual => {
            eprintln!("Paste the full callback URL here, then press Enter.");
            eprintln!(
                "Use this mode when the real redirect_uri cannot be listened on locally, e.g. Claude's http://localhost/callback on macOS."
            );
            eprintln!();
            if opts.open_browser {
                let _ = open_in_browser(&authorize_url);
            }
            let line_future = async {
                match hooks.manual_callback_rx {
                    Some(rx) => rx
                        .await
                        .map_err(|_| anyhow::anyhow!("manual callback channel closed")),
                    None => read_manual_callback().await,
                }
            };
            let payload = tokio::time::timeout(opts.timeout, line_future).await;
            CallbackCapture {
                payload: match payload {
                    Ok(Ok(line)) => parse_callback_url(&line).map_err(|err| err.to_string()),
                    Ok(Err(err)) => Err(err.to_string()),
                    Err(_) => Err(format!(
                        "timeout waiting for callback after {:?}",
                        opts.timeout
                    )),
                },
                listen_uri: None,
                mode: "manual-callback-url".to_string(),
            }
        }
    };
    let callback_listen_uri = capture.listen_uri;
    let callback_capture_mode = capture.mode;

    let payload = match capture.payload {
        Ok(p) => p,
        Err(failure) => {
            let report = OAuthCodeReport {
                authorize_url,
                redirect_uri: opts.redirect_uri,
                callback_listen_uri,
                callback_capture_mode,
                callback: None,
                token_summary: None,
                jwt: None,
                failure: Some(failure),
                elapsed_ms: start_instant.elapsed().as_millis() as u64,
            };
            write_artifacts(&report, artifact_dir)?;
            return Ok(report);
        }
    };

    // 4. Verify state and detect explicit AS errors.
    if payload.state.as_deref() != Some(state_token.as_str()) {
        let report = OAuthCodeReport {
            authorize_url,
            redirect_uri: opts.redirect_uri,
            callback_listen_uri,
            callback_capture_mode,
            callback: Some(payload),
            token_summary: None,
            jwt: None,
            failure: Some("state mismatch (possible CSRF or session swap)".into()),
            elapsed_ms: start_instant.elapsed().as_millis() as u64,
        };
        write_artifacts(&report, artifact_dir)?;
        return Ok(report);
    }
    if let Some(err) = payload.error.clone() {
        let desc = payload.error_description.clone().unwrap_or_default();
        let report = OAuthCodeReport {
            authorize_url,
            redirect_uri: opts.redirect_uri,
            callback_listen_uri,
            callback_capture_mode,
            callback: Some(payload),
            token_summary: None,
            jwt: None,
            failure: Some(format!("AS returned error: {err} ({desc})")),
            elapsed_ms: start_instant.elapsed().as_millis() as u64,
        };
        write_artifacts(&report, artifact_dir)?;
        return Ok(report);
    }
    let Some(code) = payload.code.clone() else {
        let report = OAuthCodeReport {
            authorize_url,
            redirect_uri: opts.redirect_uri,
            callback_listen_uri,
            callback_capture_mode,
            callback: Some(payload),
            token_summary: None,
            jwt: None,
            failure: Some(
                "callback had no `code` and no `error` — AS returned an empty result".into(),
            ),
            elapsed_ms: start_instant.elapsed().as_millis() as u64,
        };
        write_artifacts(&report, artifact_dir)?;
        return Ok(report);
    };

    // 5. Exchange code → token.
    let mut form: Vec<(&str, String)> = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code),
        ("redirect_uri", opts.redirect_uri.clone()),
        ("client_id", opts.client_id.clone()),
        ("code_verifier", verifier),
    ];
    if let Some(resource) = resolved_resource {
        form.push(("resource", resource));
    }
    let body = serde_urlencoded::to_string(&mut form)?;
    let token_resp = http
        .post(&token_endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("token exchange failed: {e}"))?;
    let status = token_resp.status().as_u16();
    let token_json: Value = token_resp.json().await.unwrap_or(Value::Null);
    if status != 200 {
        let report = OAuthCodeReport {
            authorize_url,
            redirect_uri: opts.redirect_uri,
            callback_listen_uri,
            callback_capture_mode,
            callback: Some(payload),
            token_summary: None,
            jwt: None,
            failure: Some(format!(
                "token endpoint returned status {status}: {}",
                token_json
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no error field)")
            )),
            elapsed_ms: start_instant.elapsed().as_millis() as u64,
        };
        write_artifacts(&report, artifact_dir)?;
        return Ok(report);
    }

    let access_token = token_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let id_token = token_json
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let summary = TokenSummary {
        token_type: token_json
            .get("token_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        expires_in: token_json.get("expires_in").and_then(|v| v.as_i64()),
        scope: token_json
            .get("scope")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        has_access_token: access_token.is_some(),
        access_token_len: access_token.as_ref().map(|s| s.len()).unwrap_or(0),
        has_refresh_token: token_json.get("refresh_token").is_some(),
        has_id_token: id_token.is_some(),
    };
    let jwt = id_token
        .as_deref()
        .or(access_token.as_deref())
        .and_then(decode_jwt_summary);

    let report = OAuthCodeReport {
        authorize_url,
        redirect_uri: opts.redirect_uri,
        callback_listen_uri,
        callback_capture_mode,
        callback: Some(payload),
        token_summary: Some(summary),
        jwt,
        failure: None,
        elapsed_ms: start_instant.elapsed().as_millis() as u64,
    };
    write_artifacts(&report, artifact_dir)?;
    Ok(report)
}

struct CallbackCapture {
    payload: Result<CallbackPayload, String>,
    listen_uri: Option<String>,
    mode: String,
}

async fn read_manual_callback() -> anyhow::Result<String> {
    let mut line = String::new();
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let bytes = stdin.read_line(&mut line).await?;
    if bytes == 0 {
        anyhow::bail!("stdin closed before a callback URL was provided");
    }
    Ok(line.trim().to_string())
}

fn parse_callback_url(input: &str) -> anyhow::Result<CallbackPayload> {
    let url = Url::parse(input).map_err(|err| anyhow::anyhow!("invalid callback URL: {err}"))?;
    Ok(CallbackPayload::from_query_pairs(url.query_pairs()))
}

#[derive(Debug, Clone)]
struct CallbackBinding {
    listen_addr: SocketAddr,
    path: String,
    listen_uri: String,
    capture_mode: String,
}

impl CallbackBinding {
    fn from_redirect_uri(redirect_uri: &str) -> anyhow::Result<Self> {
        let redirect =
            Url::parse(redirect_uri).map_err(|e| anyhow::anyhow!("invalid redirect_uri: {e}"))?;
        let host = redirect.host_str().ok_or_else(|| {
            anyhow::anyhow!("redirect_uri must have a host (e.g. http://127.0.0.1:PORT/callback)")
        })?;
        let port = redirect.port_or_known_default().ok_or_else(|| {
            anyhow::anyhow!(
                "redirect_uri must include a port or use a scheme with a known default port"
            )
        })?;
        if host != "127.0.0.1" && host != "localhost" {
            anyhow::bail!(
                "redirect_uri host must be 127.0.0.1 or localhost for local capture (got {host})"
            );
        }
        let listen_addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
        let path = if redirect.path().is_empty() {
            "/".to_string()
        } else {
            redirect.path().to_string()
        };
        let listen_uri = format!("http://{listen_addr}{path}");
        let capture_mode = if redirect.port().is_some() {
            "local-listener-explicit-port".to_string()
        } else {
            "local-listener-default-port".to_string()
        };
        Ok(Self {
            listen_addr,
            path,
            listen_uri,
            capture_mode,
        })
    }
}

async fn callback_handler(
    axum::extract::State(state): axum::extract::State<CallbackState>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<&'static str> {
    let payload = CallbackPayload::from_map(params);
    *state.payload.lock().await = Some(payload);
    state.notifier.notify_waiters();
    Html(
        "<!doctype html><meta charset=utf-8><title>remote-mcp-devkit oauth-code</title>\
         <body style=\"font-family:system-ui;max-width:480px;margin:60px auto;\">\
         <h1>Callback captured</h1>\
         <p>You can close this tab and return to the terminal.</p>\
         </body>",
    )
}

impl CallbackPayload {
    fn from_map(params: HashMap<String, String>) -> Self {
        Self {
            code: params.get("code").cloned(),
            state: params.get("state").cloned(),
            error: params.get("error").cloned(),
            error_description: params.get("error_description").cloned(),
            raw_query: serde_urlencoded::to_string(&params).unwrap_or_default(),
        }
    }

    fn from_query_pairs<'a, I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (std::borrow::Cow<'a, str>, std::borrow::Cow<'a, str>)>,
    {
        let params: HashMap<String, String> = pairs
            .into_iter()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        Self::from_map(params)
    }
}

fn open_in_browser(url: &str) -> std::io::Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "start"
    } else {
        "xdg-open"
    };
    std::process::Command::new(cmd)
        .arg(url)
        .status()
        .map(|_| ())
}

fn decode_jwt_summary(token: &str) -> Option<JwtSummary> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let header = decode_jwt_segment(parts[0])?;
    let payload = decode_jwt_segment(parts[1])?;
    Some(JwtSummary {
        which: "token".to_string(),
        header,
        payload,
    })
}

fn decode_jwt_segment(segment: &str) -> Option<Value> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_artifacts(report: &OAuthCodeReport, artifact_dir: &Path) -> anyhow::Result<()> {
    let report_path: PathBuf = artifact_dir.join("oauth-code-report.md");
    std::fs::write(&report_path, render_markdown(report))?;

    let json_path = artifact_dir.join("oauth-code-report.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(report)?)?;

    if report.failure.is_some() {
        let failures_path = artifact_dir.join("failures.json");
        std::fs::write(
            &failures_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "failure": report.failure,
                "callback": report.callback,
            }))?,
        )?;
    }
    Ok(())
}

fn render_markdown(r: &OAuthCodeReport) -> String {
    let mut out = String::new();
    out.push_str("# remote-mcp-devkit oauth-code report\n\n");
    out.push_str(&format!("- authorize_url: `{}`\n", r.authorize_url));
    out.push_str(&format!("- redirect_uri: `{}`\n", r.redirect_uri));
    out.push_str(&format!(
        "- callback_listen_uri: `{}`\n",
        r.callback_listen_uri.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!(
        "- callback_capture_mode: `{}`\n",
        r.callback_capture_mode
    ));
    out.push_str(&format!(
        "- result: **{}**\n",
        if r.passed() { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!("- elapsed_ms: `{}`\n\n", r.elapsed_ms));

    if let Some(cb) = &r.callback {
        out.push_str("## Callback\n\n");
        out.push_str(&format!(
            "- code: {}\n",
            cb.code
                .as_deref()
                .map(|s| format!(
                    "`{}…` ({} chars)",
                    &s.chars().take(8).collect::<String>(),
                    s.len()
                ))
                .unwrap_or_else(|| "(none)".into())
        ));
        out.push_str(&format!(
            "- state: {}\n",
            cb.state
                .as_deref()
                .map(|s| format!("`{}`", s))
                .unwrap_or_else(|| "(none)".into())
        ));
        if let Some(err) = &cb.error {
            out.push_str(&format!("- error: `{err}`\n"));
        }
        if let Some(desc) = &cb.error_description {
            out.push_str(&format!("- error_description: `{desc}`\n"));
        }
        out.push('\n');
    }

    if let Some(t) = &r.token_summary {
        out.push_str("## Token (redacted)\n\n");
        out.push_str(&format!("- token_type: {:?}\n", t.token_type));
        out.push_str(&format!("- expires_in: {:?}\n", t.expires_in));
        out.push_str(&format!("- scope: {:?}\n", t.scope));
        out.push_str(&format!(
            "- access_token: present={} length={}\n",
            t.has_access_token, t.access_token_len
        ));
        out.push_str(&format!(
            "- refresh_token: present={}\n",
            t.has_refresh_token
        ));
        out.push_str(&format!("- id_token: present={}\n\n", t.has_id_token));
    }

    if let Some(j) = &r.jwt {
        out.push_str("## JWT payload summary\n\n");
        out.push_str("Header:\n\n```json\n");
        out.push_str(&serde_json::to_string_pretty(&j.header).unwrap_or_default());
        out.push_str("\n```\n\nPayload:\n\n```json\n");
        out.push_str(&serde_json::to_string_pretty(&j.payload).unwrap_or_default());
        out.push_str("\n```\n\n");
    }

    if let Some(f) = &r.failure {
        out.push_str("## Failure\n\n");
        out.push_str(&format!("{f}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::CallbackBinding;

    #[test]
    fn callback_binding_uses_default_http_port_when_redirect_uri_omits_port() {
        let binding = CallbackBinding::from_redirect_uri("http://localhost/callback").unwrap();
        assert_eq!(binding.listen_addr.to_string(), "127.0.0.1:80");
        assert_eq!(binding.path, "/callback");
        assert_eq!(binding.listen_uri, "http://127.0.0.1:80/callback");
        assert_eq!(binding.capture_mode, "local-listener-default-port");
    }

    #[test]
    fn callback_binding_preserves_explicit_loopback_port() {
        let binding =
            CallbackBinding::from_redirect_uri("http://127.0.0.1:18454/callback").unwrap();
        assert_eq!(binding.listen_addr.to_string(), "127.0.0.1:18454");
        assert_eq!(binding.path, "/callback");
        assert_eq!(binding.listen_uri, "http://127.0.0.1:18454/callback");
        assert_eq!(binding.capture_mode, "local-listener-explicit-port");
    }

    #[test]
    fn callback_binding_rejects_non_loopback_hosts() {
        let err = CallbackBinding::from_redirect_uri("https://example.com/callback")
            .unwrap_err()
            .to_string();
        assert!(err.contains("host must be 127.0.0.1 or localhost"));
    }
}
