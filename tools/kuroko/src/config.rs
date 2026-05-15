//! Server configuration loaded from environment variables and CLI overrides.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    /// Optional directory for JSON snapshot persistence. `None` means in-memory only.
    pub data_dir: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let host = std::env::var("KUROKO_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("KUROKO_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4566);
        let data_dir = std::env::var("KUROKO_DATA_DIR")
            .ok()
            .filter(|s| !s.is_empty());
        Self {
            host,
            port,
            data_dir,
        }
    }

    pub fn data_dir_path(&self) -> Option<PathBuf> {
        self.data_dir.as_ref().map(PathBuf::from)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 4566,
            data_dir: None,
        }
    }
}
