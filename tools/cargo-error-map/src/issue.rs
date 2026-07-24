use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    AnyhowLeak,
    ErrorEnumBloat,
    MissingContext,
    BoundaryPanic,
    DynErrorExposure,
}

impl IssueType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::AnyhowLeak => "anyhow-leak",
            Self::ErrorEnumBloat => "error-enum-bloat",
            Self::MissingContext => "missing-context",
            Self::BoundaryPanic => "boundary-panic",
            Self::DynErrorExposure => "dyn-error-exposure",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::AnyhowLeak => "anyhow 漏れ",
            Self::ErrorEnumBloat => "error enum 肥大化",
            Self::MissingContext => "context 欠落",
            Self::BoundaryPanic => "境界外 panic",
            Self::DynErrorExposure => "dyn Error 露出",
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
pub enum Layer {
    PublicApi,
    Boundary,
    Internal,
}

impl Layer {
    pub const fn id(self) -> &'static str {
        match self {
            Self::PublicApi => "public-api",
            Self::Boundary => "boundary",
            Self::Internal => "internal",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::PublicApi => "公開 API",
            Self::Boundary => "境界",
            Self::Internal => "内部",
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub key: IssueKey,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: usize,
    pub layer: Layer,
    pub message: String,
    pub remediation: String,
    pub fan_in: usize,
}

impl Issue {
    pub fn issue_type(&self) -> IssueType {
        self.key.issue_type
    }
}
