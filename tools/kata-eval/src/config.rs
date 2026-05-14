//! Repo-root discovery and `.kata.yaml` / `.waxa.yaml` / `.waza.yaml` loading.

use crate::types::ProjectConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const CONFIG_FILENAMES: &[&str] = &[".kata.yaml", ".waxa.yaml", ".waza.yaml"];

/// Walk up from `start` looking for one of the supported config files.
/// Returns the directory containing the first match, or `start` if none found.
pub fn find_repo_root(start: &Path) -> PathBuf {
    let mut cur = start.to_path_buf();
    loop {
        for name in CONFIG_FILENAMES {
            if cur.join(name).is_file() {
                return cur;
            }
        }
        if !cur.pop() {
            return start.to_path_buf();
        }
    }
}

pub fn load_project_config(root: &Path) -> Result<ProjectConfig> {
    for name in CONFIG_FILENAMES {
        let path = root.join(name);
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading project config {}", path.display()))?;
            let cfg: ProjectConfig = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?;
            return Ok(cfg);
        }
    }
    Ok(ProjectConfig::default())
}
