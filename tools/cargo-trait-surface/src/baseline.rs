use std::path::Path;

use serde::Serialize;

use crate::analyzer::{Analysis, analyze_path};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::issue::Issue;

#[derive(Debug, Clone, Serialize)]
pub struct BaselineDiff {
    pub new_issues: Vec<Issue>,
    pub resolved_issues: Vec<Issue>,
    pub unchanged: usize,
    pub baseline_grade: crate::scoring::Grade,
    pub current_grade: crate::scoring::Grade,
}

pub fn diff_against_ref(
    current_path: &Path,
    config: &Config,
    current: &Analysis,
    git_ref: &str,
) -> Result<BaselineDiff> {
    let worktree =
        design_gate_core::prepare_baseline_worktree(current_path, git_ref, "cargo-trait-surface")?;
    let baseline = analyze_path(worktree.baseline_path(), config)
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
    use crate::issue::{IssueKey, IssueType, Layer, Severity};
    use std::path::PathBuf;

    fn analysis(source: &str) -> Analysis {
        Analysis {
            project: "fixture".to_string(),
            root: PathBuf::from("."),
            files_analyzed: 1,
            suppressed_issues: 0,
            grade: crate::scoring::Grade::B,
            issues: vec![Issue {
                key: IssueKey {
                    issue_type: IssueType::OversizedTrait,
                    source: source.to_string(),
                    target: "surface".to_string(),
                },
                severity: Severity::High,
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                layer: Layer::PublicApi,
                message: String::new(),
                remediation: String::new(),
                fan_in: 0,
            }],
            blind_spots: BlindSpotManifest::default(),
            traits: Vec::new(),
        }
    }

    #[test]
    fn identical_keys_are_unchanged() {
        let diff = diff(&analysis("src/lib.rs:Trait"), &analysis("src/lib.rs:Trait"));
        assert_eq!(diff.new_issues.len(), 0);
        assert_eq!(diff.resolved_issues.len(), 0);
        assert_eq!(diff.unchanged, 1);
    }
}
