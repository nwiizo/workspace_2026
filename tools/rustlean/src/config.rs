use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{Result, RustLeanError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RustLeanConfig {
    pub output: OutputFormat,
    pub cost_weights: CostWeights,
    pub thresholds: Thresholds,
    #[serde(default)]
    pub ignore_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CostWeights {
    pub clone_heap: f64,
    pub clone_stack: f64,
    pub heap_alloc: f64,
    pub vec_push: f64,
    pub string_alloc: f64,
    pub format_macro: f64,
    pub loop_multiplier: f64,
    pub large_struct_move: f64,
    pub padding_waste_per_byte: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    pub large_struct_bytes: usize,
    pub padding_waste_percent: f64,
    pub fn_score_warning: f64,
    pub fn_score_critical: f64,
}

impl Default for RustLeanConfig {
    fn default() -> Self {
        Self {
            output: OutputFormat::Text,
            cost_weights: CostWeights::default(),
            thresholds: Thresholds::default(),
            ignore_paths: Vec::new(),
        }
    }
}

impl Default for CostWeights {
    fn default() -> Self {
        Self {
            clone_heap: 10.0,
            clone_stack: 2.0,
            heap_alloc: 8.0,
            vec_push: 3.0,
            string_alloc: 7.0,
            format_macro: 8.0,
            loop_multiplier: 10.0,
            large_struct_move: 5.0,
            padding_waste_per_byte: 1.0,
        }
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            large_struct_bytes: 128,
            padding_waste_percent: 25.0,
            fn_score_warning: 50.0,
            fn_score_critical: 100.0,
        }
    }
}

impl RustLeanConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_from_cwd() -> Result<Self> {
        let candidates = ["rustlean.toml", ".rustlean.toml"];
        for name in &candidates {
            let path = Path::new(name);
            if path.exists() {
                return Self::load(path);
            }
        }
        Err(RustLeanError::Config(
            "no config file found, using defaults".into(),
        ))
    }

    pub fn load_or_default() -> Self {
        match Self::load_from_cwd() {
            Ok(config) => config,
            Err(RustLeanError::Config(_)) => {
                // No config file found — this is expected, use defaults silently
                Self::default()
            }
            Err(e) => {
                // IO error or TOML parse error — warn the user
                eprintln!("rustlean: warning: {e}, using default configuration");
                Self::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = RustLeanConfig::default();
        assert_eq!(config.output, OutputFormat::Text);
        assert!(config.cost_weights.clone_heap > 0.0);
        assert!(config.thresholds.large_struct_bytes > 0);
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
output = "json"
ignore_paths = ["tests/**"]

[cost_weights]
clone_heap = 15.0

[thresholds]
large_struct_bytes = 256
"#;
        let config: RustLeanConfig = toml::from_str(toml_str).expect("parse config");
        assert_eq!(config.output, OutputFormat::Json);
        assert_eq!(config.cost_weights.clone_heap, 15.0);
        assert_eq!(config.cost_weights.clone_stack, 2.0); // default preserved
        assert_eq!(config.thresholds.large_struct_bytes, 256);
    }
}
