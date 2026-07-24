use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    TestGap,
}

impl IssueType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::TestGap => "test-gap",
        }
    }
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IssueKey {
    pub issue_type: IssueType,
    pub source: String,
    pub target: String,
}

impl IssueKey {
    pub fn core_key(&self) -> design_gate_core::IssueKey {
        design_gate_core::IssueKey::new(self.issue_type.id(), &self.source, &self.target)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub key: IssueKey,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: usize,
    pub function: String,
    pub risk: f64,
    pub churn: f64,
    pub complexity: usize,
    pub exposure: f64,
    pub coverage: f64,
    pub message: String,
    pub remediation: String,
}
