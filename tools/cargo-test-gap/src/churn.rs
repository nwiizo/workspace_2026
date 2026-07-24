use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use design_gate_core::relative_path_string;

use crate::parser::FunctionInfo;

#[derive(Debug, Clone, Copy)]
pub struct ChurnResult {
    pub score: f64,
    pub failed: bool,
}

#[derive(Debug, Clone)]
pub struct ChurnMap {
    by_path: HashMap<String, ChurnResult>,
}

impl ChurnMap {
    pub fn collect(root: &Path, functions: &[FunctionInfo]) -> Self {
        Self::collect_with_now(root, functions, current_unix_timestamp())
    }

    pub fn collect_with_now(root: &Path, functions: &[FunctionInfo], now_unix: i64) -> Self {
        let paths = functions
            .iter()
            .map(|function| function.rel_path.clone())
            .collect::<HashSet<_>>();
        let history = git_history(root);
        let by_path = match history {
            Ok(history) => paths
                .into_iter()
                .map(|rel_path| {
                    let result = history
                        .get(&rel_path)
                        .map(|timestamps| churn_from_timestamps(timestamps, now_unix))
                        .unwrap_or(ChurnResult {
                            score: 1.0,
                            failed: true,
                        });
                    (rel_path, result)
                })
                .collect(),
            Err(_) => paths
                .into_iter()
                .map(|rel_path| {
                    (
                        rel_path,
                        ChurnResult {
                            score: 1.0,
                            failed: true,
                        },
                    )
                })
                .collect(),
        };
        Self { by_path }
    }

    pub fn churn_for(&self, function: &FunctionInfo) -> ChurnResult {
        self.by_path
            .get(&function.rel_path)
            .copied()
            .unwrap_or(ChurnResult {
                score: 1.0,
                failed: true,
            })
    }
}

fn git_history(root: &Path) -> std::result::Result<HashMap<String, Vec<i64>>, ()> {
    let git_root = git_root(root).ok_or(())?;
    let output = Command::new("git")
        .args([
            "log",
            "--pretty=format:%ct",
            "--name-only",
            "--diff-filter=AMRC",
            "--",
            "*.rs",
        ])
        .current_dir(&git_root)
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut current_timestamp = None;
    let mut by_path: HashMap<String, Vec<i64>> = HashMap::new();
    for line in stdout.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Ok(timestamp) = line.parse::<i64>() {
            current_timestamp = Some(timestamp);
            continue;
        }
        let Some(timestamp) = current_timestamp else {
            continue;
        };
        let absolute = git_root.join(line);
        if !is_under(&absolute, &root) {
            continue;
        }
        let rel_path = relative_path_string(&root, &absolute);
        by_path.entry(rel_path).or_default().push(timestamp);
    }
    Ok(by_path)
}

fn git_root(root: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return None;
    }
    let root = PathBuf::from(root);
    Some(root.canonicalize().unwrap_or(root))
}

fn is_under(path: &Path, root: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.starts_with(root)
}

fn churn_from_timestamps(timestamps: &[i64], now_unix: i64) -> ChurnResult {
    if timestamps.is_empty() {
        return ChurnResult {
            score: 1.0,
            failed: true,
        };
    }
    let change_count = timestamps.len().saturating_sub(1) as f64;
    let recency = if change_count > 0.0 {
        timestamps
            .first()
            .map(|timestamp| recency_bonus(*timestamp, now_unix))
            .unwrap_or(0.0)
    } else {
        0.0
    };
    ChurnResult {
        score: change_count + recency,
        failed: false,
    }
}

fn recency_bonus(timestamp: i64, now_unix: i64) -> f64 {
    let age_days = ((now_unix - timestamp).max(0) as f64) / 86_400.0;
    if age_days <= 30.0 {
        2.0
    } else if age_days <= 180.0 {
        1.0
    } else if age_days <= 365.0 {
        0.5
    } else {
        0.0
    }
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recency_uses_injected_now_instead_of_fixed_date() {
        let commit = 1_783_440_000;
        assert_eq!(recency_bonus(commit, commit + 10 * 86_400), 2.0);
        assert_eq!(recency_bonus(commit, commit + 45 * 86_400), 1.0);
        assert_eq!(recency_bonus(commit, commit + 220 * 86_400), 0.5);
        assert_eq!(recency_bonus(commit, commit + 500 * 86_400), 0.0);
    }
}
