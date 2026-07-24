use std::path::Path;

use crate::analyzer::{AnalysisOptions, analyze_path};
use crate::error::Result;
use crate::model::{BaselineDiff, BoundaryReport, Severity};

#[derive(Debug, Clone)]
pub struct BaselineOptions {
    pub git_ref: String,
    pub fail_on: Severity,
}

pub fn diff_against_ref(
    path: &Path,
    options: &BaselineOptions,
    current: &BoundaryReport,
) -> Result<BaselineDiff> {
    let worktree =
        design_gate_core::prepare_baseline_worktree(path, &options.git_ref, "cargo-boundary")?;
    let baseline_report = analyze_path(
        worktree.baseline_path(),
        &AnalysisOptions { include_low: true },
    )?;
    Ok(diff_reports(&options.git_ref, &baseline_report, current))
}

pub fn diff_reports(
    git_ref: &str,
    baseline: &BoundaryReport,
    current: &BoundaryReport,
) -> BaselineDiff {
    let diff = design_gate_core::diff_issue_sets(&baseline.issues, &current.issues, |issue| {
        issue.key.core_key()
    });
    BaselineDiff {
        git_ref: git_ref.to_string(),
        new_issues: diff.new_issues,
        resolved_issues: diff.resolved_issues,
        unchanged: diff.unchanged,
    }
}
