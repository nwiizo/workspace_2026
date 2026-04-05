use serde::Deserialize;

use crate::diagnostics::Severity;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UnsafeRulesConfig {
    pub enabled: bool,
    pub max_unsafe_reach: Option<usize>,
    pub warn_unsafe_reach: Option<usize>,
    pub require_safety_comment: bool,
    pub flag_unsafe_trait_impls: bool,
}

impl Default for UnsafeRulesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_unsafe_reach: None,
            warn_unsafe_reach: Some(5),
            require_safety_comment: true,
            flag_unsafe_trait_impls: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OwnershipRulesConfig {
    pub enabled: bool,
    pub detect_unnecessary_clone: bool,
    pub max_borrow_depth: Option<usize>,
    pub detect_unintended_moves: bool,
}

impl Default for OwnershipRulesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            detect_unnecessary_clone: true,
            max_borrow_depth: Some(3),
            detect_unintended_moves: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomRule {
    pub id: String,
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: Severity,
}

fn default_severity() -> Severity {
    Severity::Warning
}

impl<'de> Deserialize<'de> for Severity {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "info" => Ok(Severity::Info),
            "warning" | "warn" => Ok(Severity::Warning),
            "error" | "err" => Ok(Severity::Error),
            _ => Err(serde::de::Error::custom(format!(
                "unknown severity: {s}, expected info/warning/error"
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct RulesConfig {
    pub r#unsafe: UnsafeRulesConfig,
    pub ownership: OwnershipRulesConfig,
    #[serde(default)]
    pub custom: Vec<CustomRule>,
}
