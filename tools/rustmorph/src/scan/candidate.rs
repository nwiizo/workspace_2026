use crate::simulate::{SafetyScore, Transform};
use crate::types::OwnershipKind;
use serde::{Deserialize, Serialize};

/// Risk level derived from safety score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Moderate,
    Risky,
}

impl RiskLevel {
    pub fn from_score(score: u32) -> Self {
        if score >= 80 {
            Self::Safe
        } else if score >= 50 {
            Self::Moderate
        } else {
            Self::Risky
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Safe => "[Safe]",
            Self::Moderate => "[Warn]",
            Self::Risky => "[Risk]",
        }
    }
}

/// A single refactoring opportunity discovered by the scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCandidate {
    pub function_name: String,
    pub short_name: String,
    pub param_index: usize,
    pub param_name: String,
    pub current_ownership: OwnershipKind,
    pub transform: Transform,
    pub affected_sites: usize,
    pub affected_files: usize,
    pub safety_score: SafetyScore,
}

impl ScanCandidate {
    pub fn risk_level(&self) -> RiskLevel {
        RiskLevel::from_score(self.safety_score.total)
    }
}

/// Aggregate result of a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub job_name: String,
    pub candidates: Vec<ScanCandidate>,
    pub functions_scanned: usize,
    pub triples_evaluated: usize,
    pub applicable_count: usize,
    pub duration_ms: u64,
}

impl ScanReport {
    pub fn safe_candidates(&self) -> Vec<&ScanCandidate> {
        self.candidates
            .iter()
            .filter(|c| c.risk_level() == RiskLevel::Safe)
            .collect()
    }

    pub fn moderate_candidates(&self) -> Vec<&ScanCandidate> {
        self.candidates
            .iter()
            .filter(|c| c.risk_level() == RiskLevel::Moderate)
            .collect()
    }

    pub fn risky_candidates(&self) -> Vec<&ScanCandidate> {
        self.candidates
            .iter()
            .filter(|c| c.risk_level() == RiskLevel::Risky)
            .collect()
    }
}
