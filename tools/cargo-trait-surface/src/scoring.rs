use crate::issue::{Issue, IssueType, Severity};

pub use design_gate_core::Grade;

pub fn severity(
    issue_type: IssueType,
    public_reach: bool,
    fan_in: usize,
    magnitude: usize,
) -> Severity {
    if issue_type == IssueType::SingleImplAbstraction {
        return Severity::Low;
    }
    let reach = if public_reach { 2 } else { 0 };
    let frequency = match fan_in {
        0 | 1 => 0,
        2..=4 => 1,
        _ => 2,
    };
    let size = match magnitude {
        0 | 1 => 0,
        2..=4 => 1,
        _ => 2,
    };
    let base = match issue_type {
        IssueType::OversizedTrait => 1,
        IssueType::ObjectSafetyRisk => 2,
        IssueType::BroadBlanketImpl => 1,
        IssueType::UnmockableBoundary => 2,
        IssueType::SingleImplAbstraction => 0,
    };
    match base + reach + frequency + size {
        6.. => Severity::Critical,
        4..=5 => Severity::High,
        2..=3 => Severity::Medium,
        _ => Severity::Low,
    }
}

pub fn grade(issues: &[Issue]) -> Grade {
    design_gate_core::grade_for_severities(
        issues
            .iter()
            .filter(|issue| issue.issue_type() != IssueType::SingleImplAbstraction)
            .map(|issue| issue.severity),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_impl_stays_low() {
        assert_eq!(
            severity(IssueType::SingleImplAbstraction, true, 99, 99),
            Severity::Low
        );
    }

    #[test]
    fn distribution_reaches_multiple_levels() {
        assert_eq!(
            severity(IssueType::OversizedTrait, true, 0, 1),
            Severity::Medium
        );
        assert_eq!(
            severity(IssueType::ObjectSafetyRisk, true, 5, 2),
            Severity::Critical
        );
    }
}
