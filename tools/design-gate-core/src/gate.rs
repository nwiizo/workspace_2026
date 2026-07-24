use serde::{Deserialize, Serialize};

use crate::Severity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReport {
    pub passed: bool,
    pub fail_on: String,
    pub failing: usize,
}

pub fn gate_report<T, F>(issues: &[T], fail_on: Severity, severity: F) -> GateReport
where
    F: Fn(&T) -> Severity,
{
    let failing = issues
        .iter()
        .filter(|issue| severity(issue) >= fail_on)
        .count();
    GateReport {
        passed: failing == 0,
        fail_on: fail_on.id().to_string(),
        failing,
    }
}

pub fn format_gate_line(gate: &GateReport) -> String {
    if gate.passed {
        "check: PASS".to_string()
    } else {
        format!(
            "check: FAIL (fail-on={}, {} issue(s) at/above threshold)",
            gate.fail_on, gate.failing
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_counts_threshold_and_formats_lines() {
        let gate = gate_report(
            &[Severity::Low, Severity::High, Severity::Critical],
            Severity::High,
            |severity| *severity,
        );
        assert_eq!(
            gate,
            GateReport {
                passed: false,
                fail_on: "high".to_string(),
                failing: 2
            }
        );
        assert_eq!(
            format_gate_line(&gate),
            "check: FAIL (fail-on=high, 2 issue(s) at/above threshold)"
        );
        assert_eq!(
            format_gate_line(&gate_report(&[Severity::Low], Severity::High, |severity| {
                *severity
            })),
            "check: PASS"
        );
    }
}
