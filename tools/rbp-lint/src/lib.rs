//! rbp-lint: Rust Best Practices Linter
//!
//! Built on top of [`rowan`] via [`ra_ap_syntax`]. Walks the lossless
//! concrete syntax tree and reports violations of the conventions in
//! `nwiizo/rust-best-practices`.

pub mod diagnostic;
pub mod lints;
pub mod runner;

pub use diagnostic::{Diagnostic, Severity};
pub use lints::{all_lints, LintRule};
pub use runner::{lint_file, lint_source};
