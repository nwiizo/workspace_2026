use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ra_ap_syntax::{Edition, SourceFile};

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::lints::util::comment_allows;
use crate::lints::{LintContext, LintRule, all_lints};

pub fn lint_file(path: &Path) -> Result<Vec<Diagnostic>> {
    lint_file_with_config(path, &Config::default())
}

pub fn lint_file_with_config(path: &Path, config: &Config) -> Result<Vec<Diagnostic>> {
    if config.is_path_excluded(path) {
        return Ok(Vec::new());
    }
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(lint_source_with_config(path, &source, &all_lints(), config))
}

pub fn lint_source(path: &Path, source: &str, lints: &[Box<dyn LintRule>]) -> Vec<Diagnostic> {
    lint_source_with_config(path, source, lints, &Config::default())
}

pub fn lint_source_with_config(
    path: &Path,
    source: &str,
    lints: &[Box<dyn LintRule>],
    config: &Config,
) -> Vec<Diagnostic> {
    // Parse with Rust 2024 edition. Edition affects a handful of grammar
    // productions (notably `let` chains, gen blocks); 2024 is a strict
    // superset for our syntactic lints.
    let parse = SourceFile::parse(source, Edition::Edition2024);
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
    diagnostics = apply_suppressions(diagnostics, source);
    diagnostics = diagnostics
        .into_iter()
        .filter_map(|d| config.apply(d))
        .collect();
    diagnostics.sort_by_key(|d| (d.line, d.column, d.rule));
    diagnostics
}

/// Drop any diagnostic that lives inside a `// rbp-lint-allow: <rule>` scope.
///
/// Recognised scopes (line-based, no AST):
/// - **same line, trailing comment**: `let x = y.unwrap(); // rbp-lint-allow: no-unwrap`
/// - **immediately above**: a contiguous block of `//` comments directly
///   preceding the violating line (no blank line between)
/// - **file-top**: comments at the very top of the file, until the first
///   non-comment / non-empty line
fn apply_suppressions(diagnostics: Vec<Diagnostic>, source: &str) -> Vec<Diagnostic> {
    if diagnostics.is_empty() {
        return diagnostics;
    }
    let lines: Vec<&str> = source.lines().collect();

    // file-top comments collected once
    let mut file_top: Vec<&str> = Vec::new();
    for line in &lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") {
            file_top.push(t);
        } else {
            break;
        }
    }

    diagnostics
        .into_iter()
        .filter(|d| !is_suppressed(&lines, &file_top, d))
        .collect()
}

fn is_suppressed(lines: &[&str], file_top: &[&str], d: &Diagnostic) -> bool {
    // Diagnostic line is 1-based.
    let line_idx = (d.line as usize).saturating_sub(1);

    // 1. trailing inline comment on the same line
    if let Some(line) = lines.get(line_idx) {
        if let Some(comment_start) = line.find("//") {
            let trailing = &line[comment_start..];
            if comment_allows(&[trailing], d.rule) {
                return true;
            }
        }
    }

    // 2. immediately-preceding contiguous `//` comments
    let mut block: Vec<&str> = Vec::new();
    for i in (0..line_idx).rev() {
        let raw = lines[i];
        let t = raw.trim();
        if t.is_empty() {
            // a blank line breaks the block
            break;
        }
        if t.starts_with("//") {
            block.push(t);
            continue;
        }
        // first non-empty non-comment line: stop
        break;
    }
    if comment_allows(&block, d.rule) {
        return true;
    }

    // 3. file-top
    comment_allows(file_top, d.rule)
}
