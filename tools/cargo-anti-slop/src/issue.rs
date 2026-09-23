use std::fmt;
use std::path::PathBuf;

pub use design_gate_core::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    ChainedCast,
    ErasedSignature,
    ErasedAlias,
    ErasedDictionary,
    RuntimeDowncast,
    WidenThenAssert,
    DefaultSwallow,
    VagueSymbolName,
    BoolValidation,
    RawIdParams,
    ConstructorBypass,
    OrphanFile,
}

impl IssueType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::ChainedCast => "chained-cast",
            Self::ErasedSignature => "erased-signature",
            Self::ErasedAlias => "erased-alias",
            Self::ErasedDictionary => "erased-dictionary",
            Self::RuntimeDowncast => "runtime-downcast",
            Self::WidenThenAssert => "widen-then-assert",
            Self::DefaultSwallow => "default-swallow",
            Self::VagueSymbolName => "vague-symbol-name",
            Self::BoolValidation => "bool-validation",
            Self::RawIdParams => "raw-id-params",
            Self::ConstructorBypass => "constructor-bypass",
            Self::OrphanFile => "orphan-file",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::ChainedCast => "連鎖 cast",
            Self::ErasedSignature => "型消去 signature",
            Self::ErasedAlias => "型消去 alias",
            Self::ErasedDictionary => "型消去 dictionary",
            Self::RuntimeDowncast => "実行時 downcast",
            Self::WidenThenAssert => "widen 後 assert",
            Self::DefaultSwallow => "default 握り潰し",
            Self::VagueSymbolName => "低シグナル命名",
            Self::BoolValidation => "bool 検証",
            Self::RawIdParams => "生 ID 引数",
            Self::ConstructorBypass => "constructor 迂回",
            Self::OrphanFile => "迷子ファイル",
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
    DataLoss,
    Evidence,
    Signal,
}

impl RiskAxis {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DataLoss => "data-loss",
            Self::Evidence => "evidence",
            Self::Signal => "signal",
        }
    }

    pub const fn label_ja(self) -> &'static str {
        match self {
            Self::DataLoss => "データ損失",
            Self::Evidence => "型根拠",
            Self::Signal => "シグナル",
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
