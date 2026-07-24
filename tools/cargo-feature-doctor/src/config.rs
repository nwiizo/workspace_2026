use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::issue::IssueType;

#[derive(Debug, Clone, Default)]
pub struct Config {
    allow: HashSet<IssueType>,
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    allow: Option<Vec<String>>,
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
        let allow = raw
            .allow
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| parse_issue_type(&item))
            .collect();
        Ok(Self { allow })
    }

    pub(crate) fn is_allowed(&self, issue_type: IssueType) -> bool {
        self.allow.contains(&issue_type)
    }
}

fn find_config(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    for dir in start.ancestors() {
        let candidate = dir.join("feature-doctor.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn parse_issue_type(value: &str) -> Option<IssueType> {
    match value {
        "default-leak" => Some(IssueType::DefaultLeak),
        "exclusive-undeclared" => Some(IssueType::ExclusiveUndeclared),
        "untested-cfg-path" => Some(IssueType::UntestedCfgPath),
        "optional-dep-exposure" => Some(IssueType::OptionalDepExposure),
        "non-additive-feature" => Some(IssueType::NonAdditiveFeature),
        _ => None,
    }
}
