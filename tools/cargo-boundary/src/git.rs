use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{BoundaryError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Volatility {
    Unknown,
    Low,
    Medium,
    High,
}

impl Volatility {
    pub fn multiplier(self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 1.5,
            Self::High => 2.0,
            Self::Unknown => 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub root: PathBuf,
    changes: HashMap<PathBuf, usize>,
}

impl GitInfo {
    pub fn discover(path: &Path) -> Result<Option<Self>> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .stderr(Stdio::null())
            .output();
        let output = match output {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };
        if !output.status.success() {
            return Ok(None);
        }
        let root_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if root_text.is_empty() {
            return Ok(None);
        }
        let root = PathBuf::from(root_text);
        let root = root.canonicalize().unwrap_or(root);
        let changes = git_change_counts(&root)?;
        Ok(Some(Self { root, changes }))
    }

    pub fn volatility_for(&self, path: &Path) -> Volatility {
        let relative = path.strip_prefix(&self.root).map_or(path, |p| p);
        let count = self.changes.get(relative).copied().unwrap_or(0);
        match count {
            0..=2 => Volatility::Low,
            3..=10 => Volatility::Medium,
            _ => Volatility::High,
        }
    }
}

fn git_change_counts(root: &Path) -> Result<HashMap<PathBuf, usize>> {
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:",
            "--name-only",
            "--diff-filter=AMRC",
            "--since=12 months ago",
            "--",
            "*.rs",
        ])
        .current_dir(root)
        .stderr(Stdio::null())
        .output()
        .map_err(|err| BoundaryError::Git(err.to_string()))?;
    if !output.status.success() {
        return Ok(HashMap::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut counts = HashMap::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        *counts.entry(PathBuf::from(line)).or_insert(0usize) += 1;
    }
    Ok(counts)
}
