use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    GuardAcrossAwait,
    BlockingInAsync,
    UnboundedSpawn,
    DetachedTask,
    MissingTimeout,
}

impl IssueType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::GuardAcrossAwait => "guard-across-await",
            Self::BlockingInAsync => "blocking-in-async",
            Self::UnboundedSpawn => "unbounded-spawn",
            Self::DetachedTask => "detached-task",
            Self::MissingTimeout => "missing-timeout",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::GuardAcrossAwait => "await 越し guard",
            Self::BlockingInAsync => "async 内 blocking",
            Self::UnboundedSpawn => "無制限 spawn",
            Self::DetachedTask => "切り離し task",
            Self::MissingTimeout => "timeout 欠落",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskAxis {
    Deadlock,
    Starvation,
    Leak,
    Latency,
}

impl RiskAxis {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Deadlock => "deadlock",
            Self::Starvation => "starvation",
            Self::Leak => "leak",
            Self::Latency => "latency",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::Deadlock => "デッドロック",
            Self::Starvation => "飢餓",
            Self::Leak => "leak",
            Self::Latency => "レイテンシ",
        }
    }
}

impl fmt::Display for RiskAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub key: IssueKey,
    pub severity: Severity,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub risk: RiskAxis,
    pub message: String,
    pub remediation: String,
    pub volatility: usize,
}

impl Issue {
    pub fn issue_type(&self) -> IssueType {
        self.key.issue_type
    }
}
