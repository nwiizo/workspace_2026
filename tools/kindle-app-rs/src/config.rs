use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::macos::NextKey;
use crate::web::{Layout, WaitStrategy};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub browser: BrowserConfig,
    pub capture: CaptureConfig,
    pub app_capture: AppCaptureConfig,
    pub pdf: PdfConfig,
    pub trim: TrimConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    pub chrome_profile: String,
    pub fallback_profile: Option<String>,
    pub headless: bool,
    pub timeout: u64,
    pub wait_for_login: bool,
    pub login_timeout: u64,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub default_layout: Layout,
    pub max_pages: Option<usize>,
    pub wait_strategy: WaitStrategy,
    pub wait_timeout: f64,
    pub screenshot_format: String,
    pub output_dir: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AppCaptureConfig {
    pub app_name: String,
    pub process_name: String,
    pub window_title: Option<String>,
    pub capture_region: Option<String>,
    pub scale: f64,
    pub output_dir: String,
    pub wait_after_turn: f64,
    pub initial_wait: f64,
    pub max_pages: Option<usize>,
    pub duplicate_threshold: u32,
    pub duplicate_diff_mean: f64,
    pub duplicate_size_kb: Option<f64>,
    pub duplicate_size_ratio: Option<f64>,
    pub duplicate_limit: usize,
    pub min_pages: usize,
    pub next_key: NextKey,
    pub log_file: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PdfConfig {
    pub default_quality: u8,
    pub default_resize: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TrimConfig {
    pub default_output_subdir: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let yaml = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        Self::from_yaml(&yaml)
            .with_context(|| format!("failed to parse config: {}", path.display()))
    }

    pub fn from_yaml(yaml: &str) -> Result<Self> {
        if yaml.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_yaml_ng::from_str(yaml).context("invalid YAML configuration")
    }
}

pub fn expand_tilde(value: &str) -> PathBuf {
    if value == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(relative) = value.strip_prefix("~/") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(relative))
            .unwrap_or_else(|| PathBuf::from(value));
    }
    PathBuf::from(value)
}

impl Default for BrowserConfig {
    fn default() -> Self {
        let profile = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/kindle-app-rs/chrome-profile");
        Self {
            chrome_profile: profile.to_string_lossy().into_owned(),
            fallback_profile: None,
            headless: false,
            timeout: 30_000,
            wait_for_login: true,
            login_timeout: 600_000,
            viewport_width: 3840,
            viewport_height: 2160,
        }
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            default_layout: Layout::Double,
            max_pages: None,
            wait_strategy: WaitStrategy::Hybrid,
            wait_timeout: 3.0,
            screenshot_format: "png".into(),
            output_dir: "./kindle-captures".into(),
        }
    }
}

impl Default for AppCaptureConfig {
    fn default() -> Self {
        Self {
            app_name: "Amazon Kindle".into(),
            process_name: "Kindle".into(),
            window_title: None,
            capture_region: None,
            scale: 1.0,
            output_dir: "./kindle-captures".into(),
            wait_after_turn: 0.6,
            initial_wait: 0.4,
            max_pages: Some(2_000),
            duplicate_threshold: 3,
            duplicate_diff_mean: 3.0,
            duplicate_size_kb: None,
            duplicate_size_ratio: Some(0.02),
            duplicate_limit: 5,
            min_pages: 2,
            next_key: NextKey::Right,
            log_file: None,
        }
    }
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            default_quality: 85,
            default_resize: 1.0,
        }
    }
}

impl Default for TrimConfig {
    fn default() -> Self {
        Self {
            default_output_subdir: "trimmed".into(),
        }
    }
}
