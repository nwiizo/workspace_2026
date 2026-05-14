//! JSONL output for results/ directory.

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// `results/<eval-name>-<timestamp>.jsonl`. When `iter` is set the filename
/// includes `-iter-<N>`.
pub fn results_path(root: &Path, results_dir: &str, eval_name: &str, iter: Option<u32>) -> PathBuf {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let name = match iter {
        Some(n) => format!("{eval_name}-iter-{n}-{stamp}.jsonl"),
        None => format!("{eval_name}-{stamp}.jsonl"),
    };
    root.join(results_dir).join(name)
}

pub fn append<T: Serialize>(path: &Path, record: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating results dir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let json = serde_json::to_string(record).context("serializing JSONL record")?;
    writeln!(file, "{json}").with_context(|| format!("writing to {}", path.display()))?;
    Ok(())
}
