use crate::issue::{Issue, IssueType, Severity};

pub use design_gate_core::Grade;

pub fn severity(
    issue_type: IssueType,
    public_reach: bool,
    boundary: bool,
    fan_in: usize,
) -> Severity {
    let reach = if public_reach { 3 } else { 1 };
    let layer = if boundary { 1 } else { 2 };
    let frequency = match fan_in {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    };
    let base = match issue_type {
        IssueType::AnyhowLeak | IssueType::DynErrorExposure => 2,
        IssueType::BoundaryPanic => 2,
        IssueType::MissingContext => 1,
        IssueType::ErrorEnumBloat => 1,
    };
    match reach + layer + frequency + base {
        9.. => Severity::Critical,
        7..=8 => Severity::High,
        5..=6 => Severity::Medium,
        _ => Severity::Low,
    }
}

pub fn grade(issues: &[Issue]) -> Grade {
    design_gate_core::grade_for_severities(issues.iter().map(|issue| issue.severity))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_severity_is_reachable() {
        assert_eq!(
            severity(IssueType::MissingContext, false, true, 0),
            Severity::Low
        );
    }

    #[test]
    fn grade_a_requires_no_issues() {
        assert_eq!(grade(&[]), Grade::A);
    }

    #[test]
    fn public_panic_needs_high_fan_in_for_critical() {
        assert_eq!(
            severity(IssueType::BoundaryPanic, true, false, 1),
            Severity::High
        );
        assert_eq!(
            severity(IssueType::BoundaryPanic, true, false, 5),
            Severity::Critical
        );
    }
}
