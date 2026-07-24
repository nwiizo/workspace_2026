use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    DefaultLeak,
    ExclusiveUndeclared,
    UntestedCfgPath,
    OptionalDepExposure,
    NonAdditiveFeature,
}

impl IssueType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DefaultLeak => "default-leak",
            Self::ExclusiveUndeclared => "exclusive-undeclared",
            Self::UntestedCfgPath => "untested-cfg-path",
            Self::OptionalDepExposure => "optional-dep-exposure",
            Self::NonAdditiveFeature => "non-additive-feature",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::DefaultLeak => "default feature 漏れ",
            Self::ExclusiveUndeclared => "未宣言の相互排他 feature",
            Self::UntestedCfgPath => "未検査 cfg 経路",
            Self::OptionalDepExposure => "optional dependency の公開 API 漏れ",
            Self::NonAdditiveFeature => "非加法的 feature",
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
pub enum Surface {
    Manifest,
    PublicApi,
    CfgPath,
    FeatureGraph,
}

impl Surface {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::PublicApi => "public-api",
            Self::CfgPath => "cfg-path",
            Self::FeatureGraph => "feature-graph",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::PublicApi => "公開 API",
            Self::CfgPath => "cfg 経路",
            Self::FeatureGraph => "feature graph",
        }
    }
}

impl fmt::Display for Surface {
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
    pub surface: Surface,
    pub features: Vec<String>,
    pub message: String,
    pub remediation: String,
    pub affected_combinations: u128,
    pub public_api: bool,
    pub usage: usize,
}

impl Issue {
    pub fn issue_type(&self) -> IssueType {
        self.key.issue_type
    }
}
