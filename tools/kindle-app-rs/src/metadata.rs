use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebPageMetadata {
    pub page: usize,
    pub location: LocationMetadata,
    pub timestamp: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct LocationMetadata {
    pub current: i64,
    pub total: i64,
    pub percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebMetadata {
    pub asin: String,
    pub layout: String,
    pub total_pages: usize,
    pub position_range: [i64; 2],
    pub capture_range: [i64; 2],
    pub captured_at: String,
    pub pages: Vec<WebPageMetadata>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CaptureRegionMetadata {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppPageMetadata {
    pub page: usize,
    pub file: String,
    pub timestamp: String,
    pub hash: String,
    pub hash_distance: Option<u32>,
    pub mean_diff: Option<f64>,
    pub size_kb: f64,
    pub size_delta_kb: Option<f64>,
    pub size_delta_ratio: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppMetadata {
    pub source: String,
    pub book: String,
    pub app_name: String,
    pub window_title: Option<String>,
    pub capture_region: CaptureRegionMetadata,
    pub total_pages: usize,
    pub wait_after_turn: f64,
    pub duplicate_threshold: u32,
    pub duplicate_diff_mean: f64,
    pub duplicate_size_kb: Option<f64>,
    pub duplicate_size_ratio: Option<f64>,
    pub duplicate_limit: usize,
    pub min_pages: usize,
    pub next_key: String,
    pub captured_at: String,
    pub pages: Vec<AppPageMetadata>,
}

pub fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("metadata.json");
    let temporary = path.with_file_name(format!(".{file_name}.partial"));
    let bytes = serde_json::to_vec_pretty(value).context("failed to serialize metadata")?;
    fs::write(&temporary, bytes).with_context(|| {
        format!(
            "failed to write temporary metadata: {}",
            temporary.display()
        )
    })?;
    fs::rename(&temporary, path)
        .with_context(|| format!("failed to finalize metadata: {}", path.display()))?;
    Ok(())
}
