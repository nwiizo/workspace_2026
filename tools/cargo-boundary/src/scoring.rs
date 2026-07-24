use crate::git::Volatility;
use crate::model::{Grade, Issue, Severity};

pub fn issue_score(depth: usize, occurrences: usize, volatility: Volatility) -> f64 {
    depth.max(1) as f64 * occurrences.max(1) as f64 * volatility.multiplier()
}

pub fn severity(score: f64) -> Severity {
    if score >= 8.0 {
        Severity::Critical
    } else if score >= 4.0 {
        Severity::High
    } else if score >= 2.0 {
        Severity::Medium
    } else {
        Severity::Low
    }
}

pub fn project_score(issues: &[Issue]) -> f64 {
    let penalty: f64 = issues.iter().map(|issue| issue.severity.penalty()).sum();
    (100.0 - penalty).max(0.0)
}

pub fn grade(score: f64) -> Grade {
    design_gate_core::grade_from_score(score)
}
