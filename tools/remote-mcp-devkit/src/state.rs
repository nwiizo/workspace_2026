use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub public_base_url: String,
    pub listen_addr: String,
    pub upstream_mcp_url: Option<String>,
    pub pid: u32,
    pub artifact_dir: PathBuf,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    #[serde(default)]
    pub cleanup_status: CleanupStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupStatus {
    #[default]
    Running,
    Stopped,
    Failed,
}

impl SessionState {
    pub fn new_id(now: DateTime<Utc>) -> String {
        now.format("%Y%m%d-%H%M%S").to_string()
    }

    pub fn save(&self, state_dir: &Path) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(state_dir)?;
        let path = state_dir.join(format!("{}.json", self.session_id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    pub fn load(state_dir: &Path, session_id: &str) -> anyhow::Result<Self> {
        let path = state_dir.join(format!("{}.json", session_id));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("read state {}: {}", path.display(), e))?;
        let state: SessionState = serde_json::from_str(&raw)?;
        Ok(state)
    }

    pub fn delete(state_dir: &Path, session_id: &str) -> anyhow::Result<()> {
        let path = state_dir.join(format!("{}.json", session_id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn list(state_dir: &Path) -> anyhow::Result<Vec<SessionState>> {
        let mut out = Vec::new();
        if !state_dir.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(state_dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(raw) = std::fs::read_to_string(&p)
                && let Ok(s) = serde_json::from_str::<SessionState>(&raw)
            {
                out.push(s);
            }
        }
        out.sort_by_key(|s| std::cmp::Reverse(s.started_at));
        Ok(out)
    }
}
