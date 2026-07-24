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
        design_gate_core::prepare_baseline_worktree(current_path, git_ref, "cargo-feature-doctor")?;
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
    use crate::analyzer::{FeatureMatrixRow, HackSuggestion};
    use crate::blind_spot::BlindSpotManifest;
    use crate::issue::{IssueKey, IssueType, Severity, Surface};
    use std::path::PathBuf;

    fn analysis(source: &str) -> Analysis {
        Analysis {
            project: "fixture".to_string(),
            root: PathBuf::from("."),
            manifest: PathBuf::from("Cargo.toml"),
            files_analyzed: 1,
            suppressed_issues: 0,
            grade: crate::scoring::Grade::B,
            feature_count: 1,
            combination_estimate: "2^1=2".to_string(),
            issues: vec![Issue {
                key: IssueKey {
                    issue_type: IssueType::OptionalDepExposure,
                    source: source.to_string(),
                    target: "serde".to_string(),
                },
                severity: Severity::High,
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                surface: Surface::PublicApi,
                features: vec!["serde".to_string()],
                message: String::new(),
                remediation: String::new(),
                affected_combinations: 1,
                public_api: true,
                usage: 1,
            }],
            matrix: Vec::<FeatureMatrixRow>::new(),
            hack_suggestions: Vec::<HackSuggestion>::new(),
            blind_spots: BlindSpotManifest::default(),
        }
    }

    #[test]
    fn distinct_sources_are_not_collapsed() {
        let mut baseline = analysis("src/lib.rs:a::run");
        baseline.issues.push(Issue {
            key: IssueKey {
                issue_type: IssueType::OptionalDepExposure,
                source: "src/lib.rs:b::run".to_string(),
                target: "serde".to_string(),
            },
            severity: Severity::High,
            file: PathBuf::from("src/lib.rs"),
            line: 2,
            surface: Surface::PublicApi,
            features: vec!["serde".to_string()],
            message: String::new(),
            remediation: String::new(),
            affected_combinations: 1,
            public_api: true,
            usage: 1,
        });
        let current = Analysis {
            matrix: Vec::<FeatureMatrixRow>::new(),
            hack_suggestions: Vec::<HackSuggestion>::new(),
            ..baseline.clone()
        };
        let delta = diff(&baseline, &current);
        assert_eq!(delta.unchanged, 2);
        assert_eq!(delta.new_issues.len(), 0);
    }

    #[test]
    fn identical_keys_are_unchanged() {
        let diff = diff(&analysis("src/lib.rs:leak"), &analysis("src/lib.rs:leak"));
        assert_eq!(diff.new_issues.len(), 0);
        assert_eq!(diff.resolved_issues.len(), 0);
        assert_eq!(diff.unchanged, 1);
    }
}
