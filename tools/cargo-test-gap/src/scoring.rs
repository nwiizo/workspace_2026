use crate::issue::{Issue, Severity};

pub use design_gate_core::Grade;

pub fn risk(churn: f64, complexity: usize, exposure: f64, coverage: f64) -> f64 {
    churn * complexity as f64 * exposure / (coverage + 1.0)
}

pub fn severity_for_risk(risk: f64) -> Severity {
    if risk >= 80.0 {
        Severity::Critical
    } else if risk >= 30.0 {
        Severity::High
    } else if risk >= 8.0 {
        Severity::Medium
    } else {
        Severity::Low
    }
}

pub fn grade(issues: &[Issue]) -> Grade {
    if issues.is_empty() {
        return Grade::A;
    }
    let severe = issues
        .iter()
        .filter(|issue| matches!(issue.severity, Severity::High | Severity::Critical))
        .count();
    let ratio = severe as f64 / issues.len() as f64;
    if ratio == 0.0 {
        Grade::A
    } else if ratio <= 0.02 {
        Grade::B
    } else if ratio <= 0.05 {
        Grade::C
    } else if ratio <= 0.10 {
        Grade::D
    } else {
        Grade::F
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{IssueKey, IssueType};
    use std::path::PathBuf;

    fn issue(severity: Severity) -> Issue {
        Issue {
            key: IssueKey {
                issue_type: IssueType::TestGap,
                source: "src/lib.rs:f".to_string(),
                target: "internal".to_string(),
            },
            severity,
            file: PathBuf::from("src/lib.rs"),
            line: 1,
            function: "f".to_string(),
            risk: 0.0,
            churn: 0.0,
            complexity: 1,
            exposure: 1.0,
            coverage: 0.0,
            message: String::new(),
            remediation: String::new(),
        }
    }

    #[test]
    fn normalized_grade_ignores_low_and_uses_high_critical_ratio() {
        let lows = vec![issue(Severity::Low); 100];
        assert_eq!(grade(&lows), Grade::A);

        let mut mixed = vec![issue(Severity::Low); 99];
        mixed.push(issue(Severity::High));
        assert_eq!(grade(&mixed), Grade::B);

        let mut severe = vec![issue(Severity::Low); 8];
        severe.push(issue(Severity::High));
        severe.push(issue(Severity::Critical));
        assert_eq!(grade(&severe), Grade::F);
    }
}
