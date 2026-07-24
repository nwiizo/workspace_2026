use std::path::Path;

use serde::Serialize;

use crate::analyzer::{Analysis, AnalyzeOptions, analyze_path};
use crate::error::{Error, Result};
use crate::issue::Issue;
use crate::scoring::Grade;

#[derive(Debug, Clone, Serialize)]
pub struct BaselineDiff {
    pub new_issues: Vec<Issue>,
    pub resolved_issues: Vec<Issue>,
    pub unchanged: usize,
    pub baseline_grade: Grade,
    pub current_grade: Grade,
}

pub fn diff_against_ref(
    current_path: &Path,
    options: &AnalyzeOptions,
    current: &Analysis,
    git_ref: &str,
) -> Result<BaselineDiff> {
    let worktree =
        design_gate_core::prepare_baseline_worktree(current_path, git_ref, "cargo-test-gap")?;
    let baseline = analyze_path(worktree.baseline_path(), options)
        .map_err(|error| Error::Baseline(error.to_string()))?;
    Ok(diff(&baseline, current))
}

pub fn diff(baseline: &Analysis, current: &Analysis) -> BaselineDiff {
    let diff = design_gate_core::diff_issue_sets(&baseline.issues, &current.issues, |issue| {
        issue.key.core_key()
    });
    BaselineDiff {
        new_issues: diff.new_issues,
        resolved_issues: diff.resolved_issues,
        unchanged: diff.unchanged,
        baseline_grade: baseline.grade,
        current_grade: current.grade,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blind_spot::BlindSpotManifest;
    use crate::issue::{IssueKey, IssueType, Severity};
    use std::path::PathBuf;

    fn analysis(source: &str, target: &str) -> Analysis {
        Analysis {
            project: "fixture".to_string(),
            root: PathBuf::from("."),
            files_analyzed: 1,
            suppressed_issues: 0,
            grade: Grade::B,
            issues: vec![Issue {
                key: IssueKey {
                    issue_type: IssueType::TestGap,
                    source: source.to_string(),
                    target: target.to_string(),
                },
                severity: Severity::High,
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                function: "demo".to_string(),
                risk: 1.0,
                churn: 1.0,
                complexity: 1,
                exposure: 1.0,
                coverage: 0.0,
                message: String::new(),
                remediation: String::new(),
            }],
            blind_spots: BlindSpotManifest::default(),
        }
    }

    #[test]
    fn identical_keys_are_unchanged() {
        let diff = diff(
            &analysis("src/lib.rs:run", "coverage-uncovered;public-api"),
            &analysis("src/lib.rs:run", "coverage-uncovered;public-api"),
        );
        assert_eq!(diff.new_issues.len(), 0);
        assert_eq!(diff.resolved_issues.len(), 0);
        assert_eq!(diff.unchanged, 1);
    }
}
