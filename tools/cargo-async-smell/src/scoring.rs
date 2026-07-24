use crate::issue::{Issue, IssueType, RiskAxis, Severity};

pub use design_gate_core::Grade;

pub fn severity(
    _issue_type: IssueType,
    risk: RiskAxis,
    condition: u8,
    volatility: Option<u8>,
) -> Severity {
    let impact = match risk {
        RiskAxis::Deadlock => 2,
        RiskAxis::Starvation | RiskAxis::Leak => 1,
        RiskAxis::Latency => 0,
    };
    let score = impact + condition.min(3) + volatility.unwrap_or(0).min(2);
    match score {
        7.. => Severity::Critical,
        5..=6 => Severity::High,
        3..=4 => Severity::Medium,
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
    fn severity_distribution_has_low_and_critical_bands() {
        assert_eq!(
            severity(IssueType::BlockingInAsync, RiskAxis::Latency, 1, Some(0)),
            Severity::Low
        );
        assert_eq!(
            severity(IssueType::GuardAcrossAwait, RiskAxis::Deadlock, 3, Some(2)),
            Severity::Critical
        );
    }

    #[test]
    fn no_issue_type_is_fixed_to_one_severity_band() {
        let cases = [
            (IssueType::GuardAcrossAwait, RiskAxis::Deadlock),
            (IssueType::BlockingInAsync, RiskAxis::Latency),
            (IssueType::UnboundedSpawn, RiskAxis::Starvation),
            (IssueType::DetachedTask, RiskAxis::Leak),
            (IssueType::MissingTimeout, RiskAxis::Latency),
        ];
        for (issue_type, risk) in cases {
            let bands = [
                severity(issue_type, risk, 1, Some(0)),
                severity(issue_type, risk, 2, Some(1)),
                severity(issue_type, risk, 3, Some(2)),
            ]
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
            assert!(
                bands.len() >= 3,
                "{issue_type:?} is fixed to too few severity bands: {bands:?}"
            );
        }
    }

    #[test]
    fn grade_a_requires_no_issues() {
        assert_eq!(grade(&[]), Grade::A);
    }
}
