use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::issue::IssueType;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) blocking_calls: HashSet<String>,
    pub(crate) timeout_methods: HashSet<String>,
    pub(crate) allow: HashSet<IssueType>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blocking_calls: [
                "std::fs",
                "std::net",
                "std::thread::sleep",
                "reqwest::blocking",
                "ureq",
                "rusqlite",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            timeout_methods: ["connect", "send", "recv", "request"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            allow: HashSet::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    blocking_calls: Option<Vec<String>>,
    timeout_methods: Option<Vec<String>>,
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
        let mut config = Self::default();
        if let Some(blocking_calls) = raw.blocking_calls {
            config.blocking_calls.extend(blocking_calls);
        }
        if let Some(timeout_methods) = raw.timeout_methods {
            config.timeout_methods.extend(timeout_methods);
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
}

fn find_config(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    for dir in start.ancestors() {
        let candidate = dir.join("async-smell.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn parse_issue_type(value: &str) -> Option<IssueType> {
    match value {
        "guard-across-await" => Some(IssueType::GuardAcrossAwait),
        "blocking-in-async" => Some(IssueType::BlockingInAsync),
        "unbounded-spawn" => Some(IssueType::UnboundedSpawn),
        "detached-task" => Some(IssueType::DetachedTask),
        "missing-timeout" => Some(IssueType::MissingTimeout),
        _ => None,
    }
}
