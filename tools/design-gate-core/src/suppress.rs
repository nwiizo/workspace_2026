use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode};

use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppressionResult<T> {
    pub kept: Vec<T>,
    pub suppressed: usize,
}

pub fn apply_suppressions<T, FPath, FLine, FIssue>(
    issues: Vec<T>,
    file: FPath,
    line: FLine,
    issue_type: FIssue,
    tool_prefix: &str,
    issue_matches: impl Fn(&str, &str) -> bool,
) -> Result<SuppressionResult<T>>
where
    FPath: Fn(&T) -> &Path,
    FLine: Fn(&T) -> usize,
    FIssue: Fn(&T) -> &str,
{
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let mut kept = Vec::new();
    let mut suppressed = 0;
    for issue in issues {
        let path = file(&issue).to_path_buf();
        if !cache.contains_key(&path) {
            let source = fs::read_to_string(&path).map_err(|source| CoreError::ReadFile {
                path: path.clone(),
                source,
            })?;
            cache.insert(path.clone(), source);
        }
        let source = cache.get(&path).expect("source was inserted");
        if is_suppressed(
            source,
            line(&issue),
            issue_type(&issue),
            tool_prefix,
            &issue_matches,
        ) {
            suppressed += 1;
            continue;
        }
        kept.push(issue);
    }
    Ok(SuppressionResult { kept, suppressed })
}

pub fn is_suppressed(
    source: &str,
    line: usize,
    issue_type: &str,
    tool_prefix: &str,
    issue_matches: &impl Fn(&str, &str) -> bool,
) -> bool {
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let idx = line.saturating_sub(1);
    if line_allows(
        lines.get(idx).map(String::as_str),
        issue_type,
        tool_prefix,
        issue_matches,
    ) {
        return true;
    }
    let offset = offset_for_line(source, line);
    let parsed = SourceFile::parse(source, Edition::Edition2024);
    let tree = parsed.tree();
    if let Some(item_start) = enclosing_item_start_line(tree.syntax(), offset, source)
        && preceding_comment_allows(&lines, item_start, issue_type, tool_prefix, issue_matches)
    {
        return true;
    }
    item_start_before_line(&lines, line).is_some_and(|item_start| {
        preceding_comment_allows(&lines, item_start, issue_type, tool_prefix, issue_matches)
    })
}

fn line_allows(
    line: Option<&str>,
    issue_type: &str,
    tool_prefix: &str,
    issue_matches: &impl Fn(&str, &str) -> bool,
) -> bool {
    let Some(line) = line else {
        return false;
    };
    let marker = format!("{tool_prefix}-allow:");
    let Some((_, rest)) = line.split_once(&marker) else {
        return false;
    };
    rest.split(',')
        .map(|entry| entry.split('(').next().unwrap_or(entry))
        .map(|entry| entry.split("--").next().unwrap_or(entry))
        .map(str::trim)
        .any(|entry| entry == "all" || issue_matches(entry, issue_type))
}

fn preceding_comment_allows(
    lines: &[String],
    item_start_line: usize,
    issue_type: &str,
    tool_prefix: &str,
    issue_matches: &impl Fn(&str, &str) -> bool,
) -> bool {
    let item_start = item_start_line.saturating_sub(1);
    for previous in (0..item_start).rev() {
        let Some(line) = lines.get(previous) else {
            break;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if !trimmed.starts_with("//") {
            break;
        }
        if line_allows(Some(line), issue_type, tool_prefix, issue_matches) {
            return true;
        }
    }
    false
}

fn enclosing_item_start_line(root: &SyntaxNode, offset: usize, source: &str) -> Option<usize> {
    let mut best: Option<SyntaxNode> = None;
    for node in root.descendants() {
        let range = node.text_range();
        let start = u32::from(range.start()) as usize;
        let end = u32::from(range.end()) as usize;
        if offset < start || offset > end || !is_item_node(&node) {
            continue;
        }
        if best
            .as_ref()
            .map(|current| range.len() < current.text_range().len())
            .unwrap_or(true)
        {
            best = Some(node);
        }
    }
    best.map(|node| line_for_offset(source, u32::from(node.text_range().start()) as usize))
}

fn is_item_node(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::FN
            | SyntaxKind::STRUCT
            | SyntaxKind::ENUM
            | SyntaxKind::TRAIT
            | SyntaxKind::TYPE_ALIAS
            | SyntaxKind::IMPL
            | SyntaxKind::CONST
            | SyntaxKind::STATIC
            | SyntaxKind::MODULE
            | SyntaxKind::USE
    )
}

fn offset_for_line(source: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut current_line = 1;
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            current_line += 1;
            if current_line == line {
                return idx + 1;
            }
        }
    }
    source.len()
}

fn line_for_offset(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn item_start_before_line(lines: &[String], line: usize) -> Option<usize> {
    let idx = line.saturating_sub(1);
    for line_idx in (0..=idx).rev() {
        let line = lines.get(line_idx)?;
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        if line_idx != idx && ends_item(trimmed) {
            break;
        }
        if starts_item(trimmed) {
            return Some(line_idx + 1);
        }
    }
    None
}

fn ends_item(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or(line).trim_end();
    code.ends_with('}') || code.ends_with(';')
}

fn starts_item(line: &str) -> bool {
    let tokens = line
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    tokens.iter().any(|token| {
        matches!(
            *token,
            "fn" | "struct"
                | "enum"
                | "trait"
                | "type"
                | "impl"
                | "const"
                | "static"
                | "mod"
                | "use"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppresses_inline_and_preceding_item_comments() {
        let matches = |marker: &str, issue: &str| marker == issue;
        let inline = "fn run() { let _ = Some(1).unwrap(); } // demo-allow: panic\n";
        assert!(is_suppressed(inline, 1, "panic", "demo", &matches));
        let preceding = r#"
// demo-allow: panic
pub async fn allowed() {
    let _ = Some(1).unwrap();
}
"#;
        assert!(is_suppressed(preceding, 4, "panic", "demo", &matches));
    }

    #[test]
    fn suppression_count_is_returned() {
        #[derive(Debug)]
        struct Issue {
            file: PathBuf,
            line: usize,
            issue_type: String,
        }
        let dir = tempfile::TempDir::new().expect("tempdir");
        let file = dir.path().join("lib.rs");
        fs::write(
            &file,
            "// demo-allow: panic\nfn allowed() { unwrap(); }\nfn bad() {}\n",
        )
        .expect("write");
        let result = apply_suppressions(
            vec![
                Issue {
                    file: file.clone(),
                    line: 2,
                    issue_type: "panic".to_string(),
                },
                Issue {
                    file,
                    line: 3,
                    issue_type: "panic".to_string(),
                },
            ],
            |issue| issue.file.as_path(),
            |issue| issue.line,
            |issue| issue.issue_type.as_str(),
            "demo",
            |marker, issue| marker == issue,
        )
        .expect("suppression");
        assert_eq!(result.suppressed, 1);
        assert_eq!(result.kept.len(), 1);
    }

    #[test]
    fn preceding_allow_does_not_leak_to_adjacent_doc_item() {
        let matches = |marker: &str, issue: &str| marker == issue;
        let source = "\
// demo-allow: panic
pub trait A {}
/// The issue location may be attributed to this doc comment.
pub trait B {
    fn run(&self);
}
";
        assert!(!is_suppressed(source, 3, "panic", "demo", &matches));
    }

    #[test]
    fn preceding_allow_still_suppresses_enclosing_item() {
        let matches = |marker: &str, issue: &str| marker == issue;
        let source = "\
// demo-allow: panic
/// Allowed item documentation.
pub trait B {
    fn run(&self);
}
";
        assert!(is_suppressed(source, 3, "panic", "demo", &matches));
    }
}
