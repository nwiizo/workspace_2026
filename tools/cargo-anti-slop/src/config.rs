use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::issue::IssueType;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) erased_types: HashSet<String>,
    pub(crate) map_types: HashSet<String>,
    pub(crate) vague_words: HashSet<String>,
    pub(crate) fallible_markers: Vec<String>,
    pub(crate) allow: HashSet<IssueType>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            erased_types: [
                "serde_json::Value",
                "serde_yaml::Value",
                "toml::Value",
                "std::any::Any",
                "core::any::Any",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            map_types: ["HashMap", "BTreeMap", "IndexMap", "DashMap"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            vague_words: [
                "Data", "Info", "Helper", "Util", "Utils", "Manager", "Wrapper", "Misc", "Temp",
                "Stuff", "Thing",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            fallible_markers: [
                ".parse", "parse::<", "from_str", "try_into", "try_from", ".ok()",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            allow: HashSet::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    erased_types: Option<Vec<String>>,
    map_types: Option<Vec<String>>,
    vague_words: Option<Vec<String>>,
    fallible_markers: Option<Vec<String>>,
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
        if let Some(erased_types) = raw.erased_types {
            config.erased_types.extend(erased_types);
        }
        if let Some(map_types) = raw.map_types {
            config.map_types.extend(map_types);
        }
        if let Some(vague_words) = raw.vague_words {
            config.vague_words.extend(vague_words);
        }
        if let Some(fallible_markers) = raw.fallible_markers {
            config.fallible_markers.extend(fallible_markers);
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
        let candidate = dir.join("anti-slop.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn parse_issue_type(value: &str) -> Option<IssueType> {
    match value {
        "chained-cast" => Some(IssueType::ChainedCast),
        "erased-signature" => Some(IssueType::ErasedSignature),
        "erased-alias" => Some(IssueType::ErasedAlias),
        "erased-dictionary" => Some(IssueType::ErasedDictionary),
        "runtime-downcast" => Some(IssueType::RuntimeDowncast),
        "widen-then-assert" => Some(IssueType::WidenThenAssert),
        "default-swallow" => Some(IssueType::DefaultSwallow),
        "vague-symbol-name" => Some(IssueType::VagueSymbolName),
        "bool-validation" => Some(IssueType::BoolValidation),
        "raw-id-params" => Some(IssueType::RawIdParams),
        "constructor-bypass" => Some(IssueType::ConstructorBypass),
        "orphan-file" => Some(IssueType::OrphanFile),
        _ => None,
    }
}
