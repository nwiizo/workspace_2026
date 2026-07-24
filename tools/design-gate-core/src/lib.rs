pub mod baseline;
pub mod blind_spot;
pub mod cli;
pub mod gate;
pub mod issue;
pub mod output;
pub mod scoring;
pub mod suppress;
pub mod walker;

mod error;

pub use baseline::{
    BaselineWorktree, WorktreeGuard, prepare_baseline_worktree, relative_subpath, repo_root,
    run_git, run_git_with,
};
pub use blind_spot::{BlindSpot, BlindSpotManifest};
pub use cli::{USAGE_ERROR_EXIT, absorb_cargo_subcommand, select_mode, warn_ignored_modes};
pub use error::{CoreError, Result};
pub use gate::{GateReport, format_gate_line, gate_report};
pub use issue::{IssueKey, diff_issue_sets, sort_dedup_by_key, unique_by_key};
pub use output::{OutputOptions, localized, localized_severity, localized_severity_by_name};
pub use scoring::{Grade, Severity, grade_for_severities, grade_from_score};
pub use suppress::{SuppressionResult, apply_suppressions, is_suppressed};
pub use walker::{
    NoRustFiles, RustFileWalkerOptions, relative_path, relative_path_string, rust_files,
};
