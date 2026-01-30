//! Test reporting module for Rectitude
//!
//! Provides structured test reports in multiple formats.

use crate::scenario::ScenarioResult;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Summary of test execution
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// Total number of scenarios
    pub total: usize,
    /// Number of passed scenarios
    pub passed: usize,
    /// Number of failed scenarios
    pub failed: usize,
    /// Pass rate as percentage
    pub pass_rate: f64,
}

/// Complete test report
#[derive(Debug, Clone, Serialize)]
pub struct TestReport {
    /// Report generation timestamp
    pub timestamp: DateTime<Utc>,
    /// Total execution time in milliseconds
    pub total_duration_ms: u64,
    /// Summary statistics
    pub summary: Summary,
    /// Individual scenario results
    pub scenarios: Vec<ScenarioResult>,
}

impl TestReport {
    /// Create a new report from scenario results
    pub fn from_results(results: Vec<ScenarioResult>) -> Self {
        let passed = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();
        let total = results.len();
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            timestamp: Utc::now(),
            total_duration_ms: results.iter().map(|r| r.duration_ms).sum(),
            summary: Summary {
                total,
                passed,
                failed,
                pass_rate,
            },
            scenarios: results,
        }
    }

    /// Convert report to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert report to compact JSON (no pretty printing)
    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Print human-readable report to stdout
    pub fn print_summary(&self) {
        println!("=== Test Report ===");
        println!(
            "Timestamp: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("Duration: {}ms\n", self.total_duration_ms);

        println!("Summary:");
        println!("  Total:   {}", self.summary.total);
        println!(
            "  Passed:  {} ({:.1}%)",
            self.summary.passed, self.summary.pass_rate
        );
        println!("  Failed:  {}", self.summary.failed);
        println!();

        // Print individual scenario results
        for scenario in &self.scenarios {
            let status = if scenario.success { "PASS" } else { "FAIL" };
            let icon = if scenario.success { "✓" } else { "✗" };

            println!(
                "{} [{}] {} ({}ms)",
                icon, status, scenario.name, scenario.duration_ms
            );

            // Print tags if any
            if !scenario.tags.is_empty() {
                println!("  Tags: {}", scenario.tags.join(", "));
            }

            // Print step results for failed scenarios
            if !scenario.success {
                for (step_name, result) in &scenario.steps {
                    if !result.success {
                        let msg = result.message.as_deref().unwrap_or("(no message)");
                        println!("    ✗ {}: {}", step_name, msg);
                    }
                }
            }
        }

        println!();

        // Final status
        if self.summary.failed == 0 {
            println!("All tests passed!");
        } else {
            println!("{} test(s) failed", self.summary.failed);
        }
    }

    /// Get only failed scenarios
    pub fn failed_scenarios(&self) -> Vec<&ScenarioResult> {
        self.scenarios.iter().filter(|r| !r.success).collect()
    }

    /// Get only passed scenarios
    pub fn passed_scenarios(&self) -> Vec<&ScenarioResult> {
        self.scenarios.iter().filter(|r| r.success).collect()
    }

    /// Filter scenarios by tag
    pub fn scenarios_with_tag(&self, tag: &str) -> Vec<&ScenarioResult> {
        self.scenarios
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }
}

/// Builder for collecting multiple scenario results
pub struct ReportBuilder {
    results: Vec<ScenarioResult>,
}

impl ReportBuilder {
    /// Create a new report builder
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Add a scenario result
    pub fn add_result(mut self, result: ScenarioResult) -> Self {
        self.results.push(result);
        self
    }

    /// Add multiple scenario results
    pub fn add_results(mut self, results: impl IntoIterator<Item = ScenarioResult>) -> Self {
        self.results.extend(results);
        self
    }

    /// Build the final report
    pub fn build(self) -> TestReport {
        TestReport::from_results(self.results)
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::StepResult;
    use std::collections::HashMap;

    fn mock_scenario_result(name: &str, success: bool, tags: Vec<String>) -> ScenarioResult {
        ScenarioResult {
            name: name.to_string(),
            success,
            steps: vec![(
                "step1".to_string(),
                StepResult {
                    success,
                    message: if success {
                        None
                    } else {
                        Some("Failed".to_string())
                    },
                    data: HashMap::new(),
                },
            )],
            duration_ms: 100,
            tags,
        }
    }

    #[test]
    fn test_report_from_results() {
        let results = vec![
            mock_scenario_result("test1", true, vec!["security".to_string()]),
            mock_scenario_result("test2", false, vec!["auth".to_string()]),
            mock_scenario_result("test3", true, vec![]),
        ];

        let report = TestReport::from_results(results);

        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.passed, 2);
        assert_eq!(report.summary.failed, 1);
        assert!((report.summary.pass_rate - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_report_builder() {
        let report = ReportBuilder::new()
            .add_result(mock_scenario_result("test1", true, vec![]))
            .add_result(mock_scenario_result("test2", true, vec![]))
            .build();

        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.passed, 2);
    }

    #[test]
    fn test_filter_by_tag() {
        let results = vec![
            mock_scenario_result("test1", true, vec!["security".to_string()]),
            mock_scenario_result("test2", true, vec!["auth".to_string()]),
        ];

        let report = TestReport::from_results(results);
        let security_tests = report.scenarios_with_tag("security");

        assert_eq!(security_tests.len(), 1);
        assert_eq!(security_tests[0].name, "test1");
    }

    #[test]
    fn test_json_output() {
        let report = TestReport::from_results(vec![mock_scenario_result("test1", true, vec![])]);

        let json = report.to_json();
        assert!(json.contains("test1"));
        assert!(json.contains("passed"));
    }
}
