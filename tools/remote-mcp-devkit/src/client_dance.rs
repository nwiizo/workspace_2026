use crate::pkce;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DanceReport {
    pub base_url: String,
    pub steps: Vec<DanceStep>,
    pub access_token_preview: Option<String>,
    pub authorized_call_status: Option<u16>,
}

impl DanceReport {
    pub fn passed(&self) -> bool {
        self.access_token_preview.is_some() && self.authorized_call_status == Some(200)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DanceStep {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

pub async fn run(
    base_url: &str,
    mcp_path: &str,
    client_id_hint: Option<&str>,
    client_id_metadata_document: Option<&str>,
    artifact_dir: &Path,
) -> anyhow::Result<DanceReport> {
    std::fs::create_dir_all(artifact_dir)?;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let base = base_url.trim_end_matches('/').to_string();
    let mcp_url = format!("{base}{mcp_path}");
    let mut steps = Vec::new();

    // 1. PRM discovery
    let prm = client
        .get(format!("{base}/.well-known/oauth-protected-resource"))
        .send()
        .await?;
    let prm_status = prm.status().as_u16();
    let prm_json: Value = prm.json().await.unwrap_or(Value::Null);
    steps.push(DanceStep {
        name: "PRM".into(),
        ok: prm_status == 200,
        detail: format!(
            "status={prm_status}; resource={:?}",
            prm_json.get("resource")
        ),
    });

    // 2. AS metadata
    let as_meta_url = prm_json
        .get("authorization_servers")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| {
            format!(
                "{}/.well-known/oauth-authorization-server",
                s.trim_end_matches('/')
            )
        })
        .unwrap_or_else(|| format!("{base}/.well-known/oauth-authorization-server"));
    let as_meta: Value = client.get(&as_meta_url).send().await?.json().await?;
    let auth_ep = as_meta
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("AS metadata missing authorization_endpoint"))?
        .to_string();
    let token_ep = as_meta
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("AS metadata missing token_endpoint"))?
        .to_string();
    let reg_ep = as_meta
        .get("registration_endpoint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    steps.push(DanceStep {
        name: "AS metadata".into(),
        ok: true,
        detail: format!("authorize={auth_ep}; token={token_ep}"),
    });

    // 3. unauth MCP call → expect 401
    let unauth = client.post(&mcp_url).body("{}".to_string()).send().await?;
    let unauth_status = unauth.status().as_u16();
    let www = unauth
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    steps.push(DanceStep {
        name: "unauth MCP".into(),
        ok: unauth_status == 401 && www.to_ascii_lowercase().starts_with("bearer"),
        detail: format!("status={unauth_status}; www-authenticate={www}"),
    });

    // 4. (optional) DCR
    let mut client_id = client_id_hint
        .map(|s| s.to_string())
        .unwrap_or_else(|| "fake-client".into());
    let redirect_uri = "http://127.0.0.1:9/callback";
    if let Some(reg_url) = reg_ep {
        let mut payload = serde_json::Map::new();
        payload.insert(
            "redirect_uris".into(),
            Value::Array(vec![Value::String(redirect_uri.to_string())]),
        );
        if let Some(doc) = client_id_metadata_document {
            payload.insert(
                "client_id_metadata_document".into(),
                Value::String(doc.to_string()),
            );
        }
        if let Some(name) = client_id_hint {
            payload.insert("client_name".into(), Value::String(name.to_string()));
        }
        let resp = client.post(&reg_url).json(&payload).send().await?;
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if let Some(cid) = body.get("client_id").and_then(|v| v.as_str()) {
            client_id = cid.to_string();
        }
        steps.push(DanceStep {
            name: "DCR".into(),
            ok: status == 200 || status == 201,
            detail: format!("status={status}; client_id={client_id}"),
        });
    }

    // 5. authorize → expect 302 with code
    let verifier = pkce::random_verifier();
    let challenge = pkce::challenge_s256(&verifier);
    let state_token = uuid::Uuid::new_v4().simple().to_string();
    let auth_url = format!(
        "{auth_ep}?response_type=code&client_id={cid}&redirect_uri={ru}&code_challenge={cc}&code_challenge_method=S256&state={st}&scope=mcp:read",
        cid = urlencoding::encode(&client_id),
        ru = urlencoding::encode(redirect_uri),
        cc = urlencoding::encode(&challenge),
        st = state_token
    );
    let auth_resp = client.get(&auth_url).send().await?;
    let auth_status = auth_resp.status().as_u16();
    let loc = auth_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let code = extract_query_param(&loc, "code").unwrap_or_default();
    let ok_authorize = matches!(auth_status, 302 | 303 | 307) && !code.is_empty();
    steps.push(DanceStep {
        name: "authorize".into(),
        ok: ok_authorize,
        detail: format!("status={auth_status}; redirect={loc}"),
    });

    // 6. token exchange
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.clone()),
        ("redirect_uri", redirect_uri.to_string()),
        ("client_id", client_id.clone()),
        ("code_verifier", verifier.clone()),
    ];
    let token_resp = client
        .post(&token_ep)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(&mut form)?)
        .send()
        .await?;
    let token_status = token_resp.status().as_u16();
    let token_body: Value = token_resp.json().await.unwrap_or(Value::Null);
    let access_token = token_body
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    steps.push(DanceStep {
        name: "token exchange".into(),
        ok: token_status == 200 && access_token.is_some(),
        detail: format!(
            "status={token_status}; token_type={:?}",
            token_body.get("token_type")
        ),
    });

    // 7. authorized MCP call
    let mut authorized_status = None;
    if let Some(tok) = &access_token {
        let resp = client
            .post(&mcp_url)
            .bearer_auth(tok)
            .body("{}".to_string())
            .send()
            .await?;
        let st = resp.status().as_u16();
        authorized_status = Some(st);
        steps.push(DanceStep {
            name: "authorized MCP call".into(),
            ok: st == 200,
            detail: format!("status={st}"),
        });
    }

    let report = DanceReport {
        base_url: base,
        steps,
        access_token_preview: access_token.as_deref().map(|t| {
            if t.len() > 12 {
                format!("{}…", &t[..12])
            } else {
                t.to_string()
            }
        }),
        authorized_call_status: authorized_status,
    };

    let path = artifact_dir.join("client-dance.json");
    std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;

    Ok(report)
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}
