use crate::config::Config;
use crate::state::SessionState;
use serde::Serialize;
use std::net::TcpListener;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub advisories: Vec<DoctorAdvisory>,
}

impl DoctorReport {
    /// `passed()` reflects only blocking checks. Advisories surface in the
    /// `advisories` field for humans / agents to read; they never fail the run.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Informational findings. A running `ngrok` or `kubectl port-forward` may be
/// intentional, so the tool surfaces them without failing — the operator
/// decides whether they are stale.
#[derive(Debug, Serialize)]
pub struct DoctorAdvisory {
    pub name: String,
    pub hits: Vec<ProcessHit>,
    pub detail: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProcessHit {
    pub pid: u32,
    pub argv: String,
}

pub async fn run(cfg: &Config, state_dir: &Path) -> DoctorReport {
    let mut checks = Vec::new();
    checks.push(check_port(&cfg.server.host, cfg.server.port));
    checks.push(check_cert_dir(state_dir));
    checks.push(check_upstream(cfg).await);
    checks.push(check_stale_sessions(state_dir));

    let advisories = stale_process_advisories(default_process_lister);

    DoctorReport { checks, advisories }
}

fn check_port(host: &str, port: u16) -> DoctorCheck {
    let addr = format!("{host}:{port}");
    match TcpListener::bind(&addr) {
        Ok(_) => DoctorCheck {
            name: format!("listen port {addr}"),
            ok: true,
            detail: "available".into(),
        },
        Err(e) => DoctorCheck {
            name: format!("listen port {addr}"),
            ok: false,
            detail: format!("not bindable: {e}"),
        },
    }
}

fn check_cert_dir(state_dir: &Path) -> DoctorCheck {
    let ok = state_dir
        .parent()
        .map(|p| p.exists() || p == Path::new(""))
        .unwrap_or(true);
    DoctorCheck {
        name: format!("state dir {}", state_dir.display()),
        ok,
        detail: if ok {
            "writable / creatable".into()
        } else {
            "parent dir missing".into()
        },
    }
}

async fn check_upstream(cfg: &Config) -> DoctorCheck {
    let Some(upstream) = cfg.upstreams.mcp.as_ref() else {
        return DoctorCheck {
            name: "upstream MCP".into(),
            ok: true,
            detail: "no upstream configured (using authorized-echo)".into(),
        };
    };
    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck {
                name: "upstream MCP".into(),
                ok: false,
                detail: format!("client build: {e}"),
            };
        }
    };
    match client.head(&upstream.url).send().await {
        Ok(r) => DoctorCheck {
            name: "upstream MCP".into(),
            ok: r.status().as_u16() < 500,
            detail: format!("{} -> {}", upstream.url, r.status()),
        },
        Err(e) => DoctorCheck {
            name: "upstream MCP".into(),
            ok: false,
            detail: format!("{}: {e}", upstream.url),
        },
    }
}

fn check_stale_sessions(state_dir: &Path) -> DoctorCheck {
    let sessions = SessionState::list(state_dir).unwrap_or_default();
    let running: Vec<_> = sessions
        .iter()
        .filter(|s| s.cleanup_status == crate::state::CleanupStatus::Running)
        .collect();
    DoctorCheck {
        name: "stale sessions".into(),
        ok: running.is_empty(),
        detail: if running.is_empty() {
            "none".into()
        } else {
            format!(
                "{} session(s) still marked running: {}",
                running.len(),
                running
                    .iter()
                    .map(|s| s.session_id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    }
}

/// Build the stale-process advisories. `lister` is injected so tests can supply
/// deterministic mock data without depending on the host's actual process list.
pub(crate) fn stale_process_advisories<F>(lister: F) -> Vec<DoctorAdvisory>
where
    F: Fn(&str) -> Vec<ProcessHit>,
{
    [
        ("ngrok", "ngrok"),
        ("kubectl port-forward", "kubectl.*port-forward"),
    ]
    .into_iter()
    .map(|(label, pattern)| {
        let hits = lister(pattern);
        let detail = if hits.is_empty() {
            "none detected".into()
        } else {
            format!(
                "{} running process(es) match (may be intentional): {}",
                hits.len(),
                hits.iter()
                    .map(|h| format!("pid={}", h.pid))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        DoctorAdvisory {
            name: format!("stale {label} process"),
            hits,
            detail,
        }
    })
    .collect()
}

/// Default lister: shells out to `pgrep -af PATTERN` on Unix. On other
/// platforms returns an empty list (we don't want to pull in a process-listing
/// dep). Failures (pgrep missing, non-zero exit) also return empty.
#[cfg(unix)]
fn default_process_lister(pattern: &str) -> Vec<ProcessHit> {
    parse_pgrep_output(
        std::process::Command::new("pgrep")
            .args(["-af", pattern])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(o.stdout)
                } else {
                    None
                }
            })
            .as_deref(),
    )
}

#[cfg(not(unix))]
fn default_process_lister(_pattern: &str) -> Vec<ProcessHit> {
    Vec::new()
}

fn parse_pgrep_output(stdout: Option<&[u8]>) -> Vec<ProcessHit> {
    let Some(bytes) = stdout else {
        return Vec::new();
    };
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let pid = parts.next()?.parse::<u32>().ok()?;
            let argv = parts.next().unwrap_or("").to_string();
            Some(ProcessHit { pid, argv })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pgrep_handles_typical_output() {
        let raw = b"12345 ngrok http 18443\n67890 ngrok --config /etc/ngrok.yml\n";
        let hits = parse_pgrep_output(Some(raw));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].pid, 12345);
        assert_eq!(hits[0].argv, "ngrok http 18443");
        assert_eq!(hits[1].pid, 67890);
    }

    #[test]
    fn parse_pgrep_handles_empty_output() {
        assert!(parse_pgrep_output(None).is_empty());
        assert!(parse_pgrep_output(Some(b"")).is_empty());
    }

    #[test]
    fn parse_pgrep_skips_malformed_lines() {
        let raw = b"not-a-pid blah\n42 valid\n";
        let hits = parse_pgrep_output(Some(raw));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].pid, 42);
    }

    #[test]
    fn advisories_report_zero_hits_as_none_detected() {
        let lister = |_: &str| Vec::new();
        let advisories = stale_process_advisories(lister);
        assert_eq!(advisories.len(), 2);
        assert!(advisories.iter().all(|a| a.hits.is_empty()));
        assert!(advisories.iter().all(|a| a.detail == "none detected"));
    }

    #[test]
    fn advisories_report_hits_with_pid_summary() {
        let lister = |_: &str| {
            vec![
                ProcessHit {
                    pid: 1234,
                    argv: "ngrok http 18443".into(),
                },
                ProcessHit {
                    pid: 5678,
                    argv: "ngrok stop".into(),
                },
            ]
        };
        let advisories = stale_process_advisories(lister);
        assert_eq!(advisories.len(), 2);
        for a in &advisories {
            assert_eq!(a.hits.len(), 2);
            assert!(a.detail.contains("pid=1234"));
            assert!(a.detail.contains("pid=5678"));
            assert!(a.detail.contains("may be intentional"));
        }
    }

    #[test]
    fn report_passed_ignores_advisories() {
        // A report with one advisory (e.g. ngrok running) and all checks ok should still pass.
        let report = DoctorReport {
            checks: vec![DoctorCheck {
                name: "x".into(),
                ok: true,
                detail: "y".into(),
            }],
            advisories: vec![DoctorAdvisory {
                name: "stale ngrok process".into(),
                hits: vec![ProcessHit {
                    pid: 1,
                    argv: "ngrok ...".into(),
                }],
                detail: "1 running process(es) match".into(),
            }],
        };
        assert!(report.passed());
    }
}
