use crate::issue::{Issue, RiskAxis, Severity};

pub use design_gate_core::Grade;

pub fn severity(risk: RiskAxis, condition: u8, volatility: Option<u8>) -> Severity {
    let impact = match risk {
        RiskAxis::DataLoss => 2,
        RiskAxis::Evidence => 1,
        RiskAxis::Signal => 0,
    };
    let score = impact + condition.min(3) + volatility.unwrap_or(0).min(2);
    match score {
        7.. => Severity::Critical,
        5..=6 => Severity::High,
        3..=4 => Severity::Medium,
        _ => Severity::Low,
    }
}

// Grade tracks what the default view shows: Medium+ findings drive the
// bands, and Low findings (hidden without --all) only demote A to B.
// Hundreds of Lows in a Value-centric design should read as "B with a
// documented tradeoff", not as F.
pub fn grade(issues: &[Issue]) -> Grade {
    let mut low = 0usize;
    let mut critical = 0usize;
    let mut high = 0usize;
    let mut visible_weight = 0usize;
    for issue in issues {
        match issue.severity {
            Severity::Low => low += 1,
            Severity::Medium => visible_weight += 2,
            Severity::High => {
                high += 1;
                visible_weight += 3;
            }
            Severity::Critical => {
                critical += 1;
                visible_weight += 4;
            }
        }
    }
    if visible_weight == 0 {
        return if low == 0 { Grade::A } else { Grade::B };
    }
    if critical == 0 && high == 0 && visible_weight <= 4 {
        Grade::B
    } else if critical == 0 && visible_weight <= 16 {
        Grade::C
    } else if critical <= 2 && visible_weight <= 32 {
        Grade::D
    } else {
        Grade::F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_distribution_has_low_and_critical_bands() {
        assert_eq!(severity(RiskAxis::Signal, 1, Some(0)), Severity::Low);
        assert_eq!(severity(RiskAxis::DataLoss, 3, Some(2)), Severity::Critical);
    }

    #[test]
    fn evidence_axis_never_reaches_critical() {
        assert_eq!(severity(RiskAxis::Evidence, 3, Some(2)), Severity::High);
    }

    #[test]
    fn no_risk_axis_is_fixed_to_one_severity_band() {
        for risk in [RiskAxis::DataLoss, RiskAxis::Evidence, RiskAxis::Signal] {
            let bands = [
                severity(risk, 1, Some(0)),
                severity(risk, 2, Some(1)),
                severity(risk, 3, Some(2)),
            ]
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
            assert!(
                bands.len() >= 2,
                "{risk:?} is fixed to too few severity bands: {bands:?}"
            );
        }
    }

    #[test]
    fn grade_a_requires_no_issues() {
        assert_eq!(grade(&[]), Grade::A);
    }

    fn issue_with(severity: Severity) -> Issue {
        use crate::issue::{IssueKey, IssueType};
        use std::path::PathBuf;
        Issue {
            key: IssueKey {
                issue_type: IssueType::ErasedSignature,
                source: "src/lib.rs:f".to_string(),
                target: "t".to_string(),
            },
            severity,
            file: PathBuf::from("src/lib.rs"),
            rel_path: "src/lib.rs".to_string(),
            line: 1,
            risk: RiskAxis::Evidence,
            message: String::new(),
            remediation: String::new(),
            volatility: 0,
        }
    }

    #[test]
    fn many_lows_alone_stay_at_grade_b() {
        let issues: Vec<Issue> = (0..300).map(|_| issue_with(Severity::Low)).collect();
        assert_eq!(grade(&issues), Grade::B);
    }

    #[test]
    fn visible_findings_drive_lower_grades() {
        let mediums: Vec<Issue> = (0..6).map(|_| issue_with(Severity::Medium)).collect();
        assert_eq!(grade(&mediums), Grade::C);
        let mut heavy: Vec<Issue> = (0..10).map(|_| issue_with(Severity::High)).collect();
        heavy.push(issue_with(Severity::Critical));
        assert_eq!(grade(&heavy), Grade::F);
    }
}
