use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Breaking,
    Risky,
    Safe,
}

impl Classification {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Breaking => "breaking",
            Self::Risky => "risky",
            Self::Safe => "safe",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::Breaking => "破壊的変更",
            Self::Risky => "リスク変更",
            Self::Safe => "安全な変更",
        }
    }
}

impl fmt::Display for Classification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Removed,
    Added,
    SignatureChanged,
    CfgChanged,
    BoundAdded,
    BoundRemoved,
    FieldAdded,
    FieldRemoved,
    FieldTypeChanged,
    VariantAdded,
    VariantRemoved,
    TraitMethodAdded,
    TraitMethodDefaultRemoved,
    DeriveRemoved,
    ReprChanged,
    OrderChanged,
    ReExportRemoved,
}

impl ChangeKind {
    // Slugs are part of stable issue keys; keep them English even in Japanese output.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Removed => "removed",
            Self::Added => "added",
            Self::SignatureChanged => "signature-changed",
            Self::CfgChanged => "cfg-changed",
            Self::BoundAdded => "bound-added",
            Self::BoundRemoved => "bound-removed",
            Self::FieldAdded => "field-added",
            Self::FieldRemoved => "field-removed",
            Self::FieldTypeChanged => "field-type-changed",
            Self::VariantAdded => "variant-added",
            Self::VariantRemoved => "variant-removed",
            Self::TraitMethodAdded => "trait-method-added",
            Self::TraitMethodDefaultRemoved => "trait-method-default-removed",
            Self::DeriveRemoved => "derive-removed",
            Self::ReprChanged => "repr-changed",
            Self::OrderChanged => "order-changed",
            Self::ReExportRemoved => "re-export-removed",
        }
    }
}

impl fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IssueKey {
    pub classification: Classification,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub key: IssueKey,
    pub change_kind: ChangeKind,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
    pub remediation: String,
}

impl Issue {
    pub fn classification(&self) -> Classification {
        self.key.classification
    }
}
