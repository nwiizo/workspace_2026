pub mod rules;

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Result, RustGuardError};
pub use rules::RulesConfig;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Sarif,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            _ => Err(format!("unknown output format: {s}")),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct GeneralConfig {
    pub exclude_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub output_path: Option<PathBuf>,
    pub include_source_snippets: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct RustGuardConfig {
    pub general: GeneralConfig,
    pub rules: RulesConfig,
    pub output: OutputConfig,
}

impl RustGuardConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| RustGuardError::ConfigRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let config: Self = toml::from_str(&content).map_err(|e| RustGuardError::ConfigParse {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(config)
    }

    pub fn from_env() -> Result<Self> {
        if let Ok(path) = std::env::var("RUSTGUARD_CONFIG") {
            return Self::load(Path::new(&path));
        }

        // Try default paths (no TOCTOU: attempt load, ignore NotFound)
        for candidate in &["rustguard.toml", ".rustguard.toml"] {
            let path = Path::new(candidate);
            match Self::load(path) {
                Ok(config) => return Ok(config),
                Err(RustGuardError::ConfigRead { source, .. })
                    if source.kind() == std::io::ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Self::default())
    }
}
