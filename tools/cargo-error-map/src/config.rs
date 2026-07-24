use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::issue::IssueType;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) enum_variant_threshold: usize,
    pub(crate) context_depth_threshold: usize,
    pub(crate) boundary_layers: Vec<String>,
    pub(crate) allow: HashSet<IssueType>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enum_variant_threshold: 12,
            context_depth_threshold: 3,
            boundary_layers: Vec::new(),
            allow: HashSet::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    thresholds: Option<Thresholds>,
    boundary_layers: Option<Vec<String>>,
    allow: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct Thresholds {
    enum_variants: Option<usize>,
    context_depth: Option<usize>,
}

impl Config {
    pub fn load_near(path: &Path) -> Result<Self> {
        let config_path = find_config(path);
        let Some(config_path) = config_path else {
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
            if let Some(value) = thresholds.enum_variants {
                config.enum_variant_threshold = value;
            }
            if let Some(value) = thresholds.context_depth {
                config.context_depth_threshold = value.max(1);
            }
        }
        if let Some(layers) = raw.boundary_layers {
            config.boundary_layers = layers;
        }
        if let Some(allow) = raw.allow {
            config.allow = allow
                .into_iter()
                .filter_map(|item| parse_issue_type(&item))
                .collect();
        }
        Ok(config)
    }

    pub(crate) fn is_allowed(&self, issue_type: IssueType) -> bool {
        self.allow.contains(&issue_type)
    }

    pub(crate) fn is_boundary_path(&self, path: &Path) -> bool {
        let normalized = path.to_string_lossy().replace('\\', "/");
        normalized.ends_with("/main.rs")
            || normalized.contains("/src/bin/")
            || normalized.contains("/tests/")
            || normalized.contains("/benches/")
            || normalized.contains("/examples/")
            || normalized.contains("/src/handlers/")
            || normalized.contains("/src/routes/")
            || normalized.contains("/src/api/")
            || self
                .boundary_layers
                .iter()
                .any(|layer| normalized.contains(layer))
    }
}

fn find_config(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    for dir in start.ancestors() {
        let candidate = dir.join("error-map.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn parse_issue_type(value: &str) -> Option<IssueType> {
    match value {
        "anyhow-leak" => Some(IssueType::AnyhowLeak),
        "error-enum-bloat" => Some(IssueType::ErrorEnumBloat),
        "missing-context" => Some(IssueType::MissingContext),
        "boundary-panic" => Some(IssueType::BoundaryPanic),
        "dyn-error-exposure" => Some(IssueType::DynErrorExposure),
        _ => None,
    }
}
