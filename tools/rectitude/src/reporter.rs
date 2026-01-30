//! Test reporting module for Rectitude
//!
//! Provides structured test reports in multiple formats:
//! - **text**: Human-readable summary (default)
//! - **json**: JSON format for programmatic consumption
//! - **tap**: Test Anything Protocol for CI integration
//! - **dot**: Progress dots for quick feedback
//! - **list**: Detailed list with step info

use crate::scenario::ScenarioResult;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

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

    /// Format the report using the specified formatter
    pub fn format(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Text => self.to_text(),
            ReportFormat::Json => self.to_json(),
            ReportFormat::Tap => self.to_tap(),
            ReportFormat::Dot => self.to_dot(),
            ReportFormat::List => self.to_list(),
        }
    }

    /// Convert to text format (same as print_summary but returns String)
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        writeln!(output, "=== Test Report ===").ok();
        writeln!(
            output,
            "Timestamp: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .ok();
        writeln!(output, "Duration: {}ms\n", self.total_duration_ms).ok();

        writeln!(output, "Summary:").ok();
        writeln!(output, "  Total:   {}", self.summary.total).ok();
        writeln!(
            output,
            "  Passed:  {} ({:.1}%)",
            self.summary.passed, self.summary.pass_rate
        )
        .ok();
        writeln!(output, "  Failed:  {}", self.summary.failed).ok();
        writeln!(output).ok();

        for scenario in &self.scenarios {
            let status = if scenario.success { "PASS" } else { "FAIL" };
            let icon = if scenario.success { "✓" } else { "✗" };

            writeln!(
                output,
                "{} [{}] {} ({}ms)",
                icon, status, scenario.name, scenario.duration_ms
            )
            .ok();

            if !scenario.tags.is_empty() {
                writeln!(output, "  Tags: {}", scenario.tags.join(", ")).ok();
            }

            if !scenario.success {
                for (step_name, result) in &scenario.steps {
                    if !result.success {
                        let msg = result.message.as_deref().unwrap_or("(no message)");
                        writeln!(output, "    ✗ {}: {}", step_name, msg).ok();
                    }
                }
            }
        }

        writeln!(output).ok();
        if self.summary.failed == 0 {
            writeln!(output, "All tests passed!").ok();
        } else {
            writeln!(output, "{} test(s) failed", self.summary.failed).ok();
        }

        output
    }

    /// Convert to TAP (Test Anything Protocol) format
    ///
    /// TAP is widely supported by CI systems and test runners.
    /// See: <https://testanything.org/>
    pub fn to_tap(&self) -> String {
        let mut output = String::new();

        writeln!(output, "TAP version 14").ok();
        writeln!(output, "1..{}", self.scenarios.len()).ok();

        for (i, scenario) in self.scenarios.iter().enumerate() {
            let test_number = i + 1;
            let status = if scenario.success { "ok" } else { "not ok" };

            writeln!(output, "{} {} - {}", status, test_number, scenario.name).ok();

            // Add YAML diagnostic block for failures
            if !scenario.success {
                writeln!(output, "  ---").ok();
                writeln!(output, "  message: 'Scenario failed'").ok();
                writeln!(output, "  severity: fail").ok();
                writeln!(output, "  duration_ms: {}", scenario.duration_ms).ok();

                // Add failed steps
                let failed_steps: Vec<_> =
                    scenario.steps.iter().filter(|(_, r)| !r.success).collect();

                if !failed_steps.is_empty() {
                    writeln!(output, "  failed_steps:").ok();
                    for (step_name, result) in failed_steps {
                        let msg = result.message.as_deref().unwrap_or("(no message)");
                        writeln!(output, "    - name: '{}'", step_name).ok();
                        writeln!(output, "      message: '{}'", msg.replace('\'', "''")).ok();
                    }
                }

                writeln!(output, "  ...").ok();
            }
        }

        output
    }

    /// Convert to dot format (progress dots)
    ///
    /// Simple format that shows a dot for each test:
    /// - `.` for passing tests
    /// - `F` for failing tests
    /// - `S` for skipped tests
    pub fn to_dot(&self) -> String {
        let mut output = String::new();

        for scenario in &self.scenarios {
            if scenario.success {
                // Check if any steps were skipped
                let has_skip = scenario.steps.iter().any(|(_, r)| r.skipped);
                if has_skip {
                    write!(output, "S").ok();
                } else {
                    write!(output, ".").ok();
                }
            } else {
                write!(output, "F").ok();
            }
        }

        writeln!(output).ok();
        writeln!(output).ok();
        writeln!(
            output,
            "{} scenarios, {} passed, {} failed",
            self.summary.total, self.summary.passed, self.summary.failed
        )
        .ok();
        writeln!(output, "Duration: {}ms", self.total_duration_ms).ok();

        output
    }

    /// Convert to list format (detailed)
    ///
    /// Shows each scenario with its steps and timing.
    pub fn to_list(&self) -> String {
        let mut output = String::new();

        for scenario in &self.scenarios {
            let icon = if scenario.success { "✓" } else { "✗" };
            writeln!(
                output,
                "{} {} [{}ms]",
                icon, scenario.name, scenario.duration_ms
            )
            .ok();

            for (step_name, result) in &scenario.steps {
                let step_icon = if result.skipped {
                    "⊘"
                } else if result.success {
                    "✓"
                } else {
                    "✗"
                };

                let msg = result.message.as_deref().unwrap_or("");
                if msg.is_empty() {
                    writeln!(output, "  {} {}", step_icon, step_name).ok();
                } else {
                    writeln!(output, "  {} {} - {}", step_icon, step_name, msg).ok();
                }
            }

            if !scenario.tags.is_empty() {
                writeln!(output, "  (tags: {})", scenario.tags.join(", ")).ok();
            }

            writeln!(output).ok();
        }

        // Summary
        writeln!(output, "---").ok();
        writeln!(
            output,
            "Total: {} | Passed: {} | Failed: {} | Duration: {}ms",
            self.summary.total, self.summary.passed, self.summary.failed, self.total_duration_ms
        )
        .ok();

        output
    }

    /// Print the report to stdout with the specified format
    pub fn print(&self, format: ReportFormat) {
        print!("{}", self.format(format));
        io::stdout().flush().ok();
    }
}

/// Report output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportFormat {
    /// Human-readable text summary
    #[default]
    Text,
    /// JSON format
    Json,
    /// TAP (Test Anything Protocol)
    Tap,
    /// Dot progress (. for pass, F for fail)
    Dot,
    /// Detailed list with steps
    List,
}

impl ReportFormat {
    /// Parse format from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "tap" => Some(Self::Tap),
            "dot" | "dots" => Some(Self::Dot),
            "list" => Some(Self::List),
            _ => None,
        }
    }

    /// Get all supported format names
    pub fn supported() -> &'static [&'static str] {
        &["text", "json", "tap", "dot", "list"]
    }
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::Tap => write!(f, "tap"),
            Self::Dot => write!(f, "dot"),
            Self::List => write!(f, "list"),
        }
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
                    skipped: false,
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

    #[test]
    fn test_tap_output() {
        let report = TestReport::from_results(vec![
            mock_scenario_result("passing_test", true, vec![]),
            mock_scenario_result("failing_test", false, vec![]),
        ]);

        let tap = report.to_tap();
        assert!(tap.contains("TAP version 14"));
        assert!(tap.contains("1..2"));
        assert!(tap.contains("ok 1 - passing_test"));
        assert!(tap.contains("not ok 2 - failing_test"));
    }

    #[test]
    fn test_dot_output() {
        let report = TestReport::from_results(vec![
            mock_scenario_result("test1", true, vec![]),
            mock_scenario_result("test2", false, vec![]),
            mock_scenario_result("test3", true, vec![]),
        ]);

        let dot = report.to_dot();
        assert!(dot.contains(".F."));
        assert!(dot.contains("3 scenarios"));
    }

    #[test]
    fn test_list_output() {
        let report = TestReport::from_results(vec![mock_scenario_result(
            "detailed_test",
            true,
            vec!["tag1".to_string()],
        )]);

        let list = report.to_list();
        assert!(list.contains("✓ detailed_test"));
        assert!(list.contains("step1"));
        assert!(list.contains("tags: tag1"));
    }

    #[test]
    fn test_report_format_parse() {
        assert_eq!(ReportFormat::parse("text"), Some(ReportFormat::Text));
        assert_eq!(ReportFormat::parse("JSON"), Some(ReportFormat::Json));
        assert_eq!(ReportFormat::parse("tap"), Some(ReportFormat::Tap));
        assert_eq!(ReportFormat::parse("dot"), Some(ReportFormat::Dot));
        assert_eq!(ReportFormat::parse("list"), Some(ReportFormat::List));
        assert_eq!(ReportFormat::parse("invalid"), None);
    }
}
