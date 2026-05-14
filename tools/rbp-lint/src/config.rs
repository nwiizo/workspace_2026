//! `.rbp-lint.toml` configuration.
//!
//! ```toml
//! [rules]
//! no-unwrap = "error"          # default
//! bool-option-pair = "off"     # disable entirely
//! tracing-format = "warning"   # raise / lower severity
//!
//! [paths]
//! exclude = ["vendor/**", "**/*_generated.rs", "target/**"]
//! ```
//!
//! The CLI walks up from the lint target until it finds `.rbp-lint.toml` or
//! hits the filesystem root.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::diagnostic::Severity;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub rules: HashMap<String, RuleSetting>,
    pub exclude_globs: Vec<glob::Pattern>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSetting {
    Off,
    Severity(Severity),
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    rules: HashMap<String, String>,
    #[serde(default)]
    paths: PathsSection,
}

#[derive(Debug, Default, Deserialize)]
struct PathsSection {
    #[serde(default)]
    exclude: Vec<String>,
}

impl Config {
    /// Walk parents of `start` looking for `.rbp-lint.toml`. Returns
    /// `Ok(default)` if none is found.
    pub fn discover(start: &Path) -> Result<Self> {
        let mut cur = if start.is_file() {
            start.parent().map(Path::to_path_buf)
        } else {
            Some(start.to_path_buf())
        };
        while let Some(dir) = cur.clone() {
            let candidate = dir.join(".rbp-lint.toml");
            if candidate.is_file() {
                return Self::from_file(&candidate);
            }
            cur = dir.parent().map(Path::to_path_buf);
        }
        Ok(Self::default())
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut cfg = Self::parse(&text).with_context(|| format!("parsing {}", path.display()))?;
        cfg.source_path = Some(path.to_path_buf());
        Ok(cfg)
    }

    pub fn parse(toml_text: &str) -> Result<Self> {
        let raw: RawConfig = toml::from_str(toml_text).context("invalid .rbp-lint.toml")?;
        let mut rules = HashMap::new();
        for (k, v) in raw.rules {
            rules.insert(k, parse_setting(&v)?);
        }
        let mut globs = Vec::new();
        for p in raw.paths.exclude {
            globs.push(glob::Pattern::new(&p).with_context(|| format!("invalid glob: {p}"))?);
        }
        Ok(Self {
            rules,
            exclude_globs: globs,
            source_path: None,
        })
    }

    pub fn is_path_excluded(&self, path: &Path) -> bool {
        let p = path.to_string_lossy();
        self.exclude_globs.iter().any(|g| g.matches(&p))
    }

    /// Apply the config to a single diagnostic. Returns `Some(diag)` (with
    /// possibly-overridden severity) if it should be kept, or `None` if the
    /// rule is set to `off`.
    pub fn apply(
        &self,
        mut d: crate::diagnostic::Diagnostic,
    ) -> Option<crate::diagnostic::Diagnostic> {
        match self.rules.get(d.rule) {
            None => Some(d),
            Some(RuleSetting::Off) => None,
            Some(RuleSetting::Severity(s)) => {
                d.severity = *s;
                Some(d)
            }
        }
    }
}

fn parse_setting(raw: &str) -> Result<RuleSetting> {
    let v = raw.trim().to_ascii_lowercase();
    match v.as_str() {
        "off" | "allow" | "disabled" | "false" => Ok(RuleSetting::Off),
        "error" | "deny" => Ok(RuleSetting::Severity(Severity::Error)),
        "warning" | "warn" => Ok(RuleSetting::Severity(Severity::Warning)),
        "note" | "info" => Ok(RuleSetting::Severity(Severity::Note)),
        _ => anyhow::bail!("unknown rule setting `{raw}` (expected off|note|warning|error)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal() {
        let cfg = Config::parse(
            r#"
[rules]
no-unwrap = "error"
bool-option-pair = "off"
tracing-format = "warning"

[paths]
exclude = ["vendor/**"]
"#,
        )
        .unwrap();
        assert_eq!(
            cfg.rules.get("no-unwrap"),
            Some(&RuleSetting::Severity(Severity::Error))
        );
        assert_eq!(cfg.rules.get("bool-option-pair"), Some(&RuleSetting::Off));
        assert!(cfg.is_path_excluded(Path::new("vendor/foo/bar.rs")));
        assert!(!cfg.is_path_excluded(Path::new("src/main.rs")));
    }

    #[test]
    fn unknown_setting_errors() {
        let err = Config::parse(
            r#"
[rules]
no-unwrap = "loud"
"#,
        )
        .unwrap_err();
        let s = format!("{err:#}");
        assert!(s.contains("unknown rule setting"), "{s}");
    }
}
