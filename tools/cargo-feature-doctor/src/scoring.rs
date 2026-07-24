use design_gate_core::{Severity, grade_for_severities};

pub use design_gate_core::Grade;

pub(crate) fn severity(affected_combinations: u128, public_api: bool, usage: usize) -> Severity {
    let mut score = 0usize;
    if affected_combinations >= 64 {
        score += 2;
    } else if affected_combinations >= 4 {
        score += 1;
    }
    if public_api {
        score += 1;
    }
    if usage >= 3 {
        score += 1;
    }
    match score {
        0 | 1 => Severity::Low,
        2 => Severity::Medium,
        3 => Severity::High,
        _ => Severity::Critical,
    }
}

pub(crate) fn grade(issues: &[crate::issue::Issue]) -> Grade {
    grade_for_severities(issues.iter().map(|issue| issue.severity))
}
