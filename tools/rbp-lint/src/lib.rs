//! rbp-lint: Rust Best Practices Linter
//!
//! Built on top of [`rowan`] via [`ra_ap_syntax`]. Walks the lossless
//! concrete syntax tree and reports violations of the conventions in
//! `nwiizo/rust-best-practices`.

pub mod config;
pub mod diagnostic;
pub mod lints;
pub mod runner;

pub use config::{Config, RuleSetting};
pub use diagnostic::{Diagnostic, Severity};
pub use lints::{LintRule, all_lints};
pub use runner::{lint_file, lint_file_with_config, lint_source, lint_source_with_config};
