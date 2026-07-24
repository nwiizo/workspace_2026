use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) method_threshold: usize,
    pub(crate) associated_type_threshold: usize,
    intentional_traits: HashSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            method_threshold: 10,
            associated_type_threshold: 4,
            intentional_traits: HashSet::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    thresholds: Option<Thresholds>,
    intent: Option<Intent>,
}

#[derive(Debug, Deserialize, Default)]
struct Thresholds {
    methods: Option<usize>,
    associated_types: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct Intent {
    intentional_abstractions: Option<Vec<String>>,
}

impl Config {
    pub fn load_near(path: &Path) -> Result<Self> {
        let Some(config_path) = find_config(path) else {
            return Ok(Self::default());
        };
        Self::load_file(&config_path)
    }

    pub fn load_file(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
        let raw: RawConfig = toml::from_str(&source).map_err(|source| Error::ConfigToml {
            path: path.to_path_buf(),
            source,
        })?;
        let mut config = Self::default();
        if let Some(thresholds) = raw.thresholds {
            if let Some(methods) = thresholds.methods {
                config.method_threshold = methods.max(1);
            }
            if let Some(types) = thresholds.associated_types {
                config.associated_type_threshold = types.max(1);
            }
        }
        if let Some(intent) = raw.intent {
            if let Some(traits) = intent.intentional_abstractions {
                config.intentional_traits = traits.into_iter().collect();
            }
        }
        Ok(config)
    }

    pub(crate) fn is_intentional_trait(&self, name: &str) -> bool {
        self.intentional_traits.contains(name)
    }
}

fn find_config(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    for dir in start.ancestors() {
        let candidate = dir.join("trait-surface.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
