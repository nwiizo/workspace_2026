use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::{BlindSpot, BlindSpotManifest, GateReport, Grade, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    LayerViolation,
    InternalCrossing,
    PubLeak,
    ForbiddenImport,
}

impl IssueType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LayerViolation => "layer-violation",
            Self::InternalCrossing => "internal-crossing",
            Self::PubLeak => "pub-leak",
            Self::ForbiddenImport => "forbidden-import",
        }
    }
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IssueKey {
    pub issue_type: IssueType,
    pub source: String,
    pub target: String,
}

impl IssueKey {
    pub fn core_key(&self) -> design_gate_core::IssueKey {
        design_gate_core::IssueKey::new(self.issue_type.as_str(), &self.source, &self.target)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub key: IssueKey,
    pub severity: Severity,
    pub score: f64,
    pub depth: usize,
    pub occurrences: usize,
    pub source_layer: Option<String>,
    pub target_layer: Option<String>,
    pub locations: Vec<Location>,
    pub message: String,
    pub message_ja: String,
    pub suggestion: String,
    pub suggestion_ja: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub name: String,
    pub rank: usize,
    pub paths: Vec<String>,
    pub source: LayerSource,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerSource {
    Config,
    Heuristic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub analyzed_files: usize,
    pub issue_count: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineDiff {
    pub git_ref: String,
    pub new_issues: Vec<Issue>,
    pub resolved_issues: Vec<Issue>,
    pub unchanged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryReport {
    pub project: String,
    pub root: PathBuf,
    pub score: f64,
    pub grade: Grade,
    pub summary: Summary,
    pub issues: Vec<Issue>,
    pub layers: Vec<LayerInfo>,
    pub blind_spots: BlindSpotManifest,
    pub baseline: Option<BaselineDiff>,
    #[serde(skip)]
    pub include_low: bool,
    #[serde(skip)]
    pub no_rust_files: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateReport>,
}

impl Summary {
    pub fn from_issues(analyzed_files: usize, issues: &[Issue]) -> Self {
        let mut counts = BTreeMap::new();
        for issue in issues {
            *counts.entry(issue.severity).or_insert(0usize) += 1;
        }
        Self {
            analyzed_files,
            issue_count: issues.len(),
            critical: *counts.get(&Severity::Critical).unwrap_or(&0),
            high: *counts.get(&Severity::High).unwrap_or(&0),
            medium: *counts.get(&Severity::Medium).unwrap_or(&0),
            low: *counts.get(&Severity::Low).unwrap_or(&0),
        }
    }
}
