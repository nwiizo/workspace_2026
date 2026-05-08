use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ra_ap_syntax::{Edition, SourceFile};

use crate::diagnostic::Diagnostic;
use crate::lints::{all_lints, LintContext, LintRule};

pub fn lint_file(path: &Path) -> Result<Vec<Diagnostic>> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(lint_source(path, &source, &all_lints()))
}

pub fn lint_source(path: &Path, source: &str, lints: &[Box<dyn LintRule>]) -> Vec<Diagnostic> {
    // Parse with Rust 2021 edition by default. The Edition only affects
    // a handful of grammar productions; for our syntactic lints it's
    // good enough.
    let parse = SourceFile::parse(source, Edition::Edition2021);
    let tree = parse.tree();
    let ctx = LintContext {
        file: path,
        source,
        tree: &tree,
    };
    let mut diagnostics = Vec::new();
    for lint in lints {
        lint.check(&ctx, &mut diagnostics);
    }
    diagnostics.sort_by_key(|d| (d.line, d.column, d.rule));
    diagnostics
}
