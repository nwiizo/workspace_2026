use serde::Serialize;

use super::location::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    UnsafeBlock,
    UnsafeFunction,
    UnsafeReach,
    UnnecessaryClone,
    ExcessiveBorrow,
    UnintendedMove,
    Custom,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::UnsafeBlock => write!(f, "unsafe-block"),
            Category::UnsafeFunction => write!(f, "unsafe-function"),
            Category::UnsafeReach => write!(f, "unsafe-reach"),
            Category::UnnecessaryClone => write!(f, "unnecessary-clone"),
            Category::ExcessiveBorrow => write!(f, "excessive-borrow"),
            Category::UnintendedMove => write!(f, "unintended-move"),
            Category::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UnsafeReachInfo {
    pub unsafe_location: SourceLocation,
    pub affected_functions: Vec<String>,
    pub call_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    pub location: SourceLocation,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_locations: Vec<SourceLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsafe_reach: Option<UnsafeReachInfo>,
    /// Whether a SAFETY comment was found for this finding (for coverage stats).
    #[serde(skip)]
    pub has_safety_comment: bool,
}
