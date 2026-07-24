use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode, TextRange, TextSize};

use crate::error::{BoundaryError, Result};
use crate::model::{IssueType, Location};

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub source: String,
    pub imports: Vec<ImportRef>,
    pub path_refs: Vec<PathRef>,
    pub pub_items: Vec<PubItem>,
    pub parse_error_count: usize,
}

#[derive(Debug, Clone)]
pub struct ImportRef {
    pub path: String,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct PathRef {
    pub path: String,
    pub location: Location,
    pub allows: BTreeSet<IssueType>,
    pub is_use: bool,
}

#[derive(Debug, Clone)]
pub struct PubItem {
    pub name: String,
    pub kind: String,
    pub location: Location,
    pub allows: BTreeSet<IssueType>,
}

impl ParsedFile {
    pub fn allows(&self, line: u32, issue_type: IssueType) -> bool {
        self.path_refs
            .iter()
            .any(|reference| reference.location.line == line && reference.allows(issue_type))
            || self
                .pub_items
                .iter()
                .any(|item| item.location.line == line && item.allows(issue_type))
    }
}

impl PathRef {
    pub fn allows(&self, issue_type: IssueType) -> bool {
        self.allows.contains(&issue_type)
    }
}

impl PubItem {
    pub fn allows(&self, issue_type: IssueType) -> bool {
        self.allows.contains(&issue_type)
    }
}

pub fn parse_file(path: &Path) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| BoundaryError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_source(path, &source))
}

pub fn parse_source(path: &Path, source: &str) -> ParsedFile {
    let parse = SourceFile::parse(source, Edition::Edition2024);
    let tree = parse.tree();
    let parse_error_count = parse.errors().len();
    let mut imports = Vec::new();
    let mut path_refs = Vec::new();
    let mut pub_items = Vec::new();
    let comments = collect_comments(tree.syntax(), source);

    for node in tree.syntax().descendants() {
        if let Some(use_item) = ast::Use::cast(node.clone()) {
            collect_use(
                &use_item,
                path,
                source,
                &comments,
                &mut imports,
                &mut path_refs,
            );
            continue;
        }
        if let Some(path_type) = ast::PathType::cast(node.clone()) {
            if let Some(path_node) = path_type.path()
                && should_collect_path(&path_node)
            {
                push_path_ref(&path_node, path, source, &comments, false, &mut path_refs);
            }
            continue;
        }
        if let Some(path_expr) = ast::PathExpr::cast(node.clone()) {
            if let Some(path_node) = path_expr.path()
                && should_collect_path(&path_node)
            {
                push_path_ref(&path_node, path, source, &comments, false, &mut path_refs);
            }
            continue;
        }
        if let Some(path_pat) = ast::PathPat::cast(node.clone()) {
            if let Some(path_node) = path_pat.path()
                && should_collect_path(&path_node)
            {
                push_path_ref(&path_node, path, source, &comments, false, &mut path_refs);
            }
            continue;
        }
        if let Some(method) = ast::MethodCallExpr::cast(node.clone()) {
            if let Some(name) = method.name_ref() {
                push_name_ref(
                    name.syntax(),
                    path,
                    source,
                    &comments,
                    false,
                    &mut path_refs,
                );
            }
            continue;
        }
        if let Some(item) = pub_item(&node, path, source, &comments) {
            pub_items.push(item);
        }
    }

    sort_dedup_refs(&mut path_refs);

    ParsedFile {
        path: path.to_path_buf(),
        source: source.to_string(),
        imports,
        path_refs,
        pub_items,
        parse_error_count,
    }
}

fn collect_use(
    use_item: &ast::Use,
    file: &Path,
    source: &str,
    comments: &[Comment],
    imports: &mut Vec<ImportRef>,
    refs: &mut Vec<PathRef>,
) {
    let allows = allow_markers(use_item.syntax(), source, comments);
    let Some(use_tree) = use_item.use_tree() else {
        return;
    };
    let mut paths = Vec::new();
    collect_use_tree(&use_tree, "", &mut paths);
    paths.sort();
    paths.dedup();
    for path_text in paths {
        let range = use_tree
            .path()
            .map(|path| path.syntax().text_range())
            .unwrap_or_else(|| use_item.syntax().text_range());
        let location = location(file, source, range, &path_text);
        imports.push(ImportRef {
            path: path_text.clone(),
            location: location.clone(),
        });
        refs.push(PathRef {
            path: path_text,
            location,
            allows: allows.clone(),
            is_use: true,
        });
    }
}

fn collect_use_tree(use_tree: &ast::UseTree, prefix: &str, out: &mut Vec<String>) {
    let path = use_tree
        .path()
        .and_then(|path| path_to_string(&path))
        .unwrap_or_default();
    let next_prefix = join_path(prefix, &path);
    if let Some(list) = use_tree.use_tree_list() {
        for child in list.use_trees() {
            collect_use_tree(&child, &next_prefix, out);
        }
        return;
    }
    if !next_prefix.is_empty() {
        out.push(next_prefix);
    }
}

fn join_path(prefix: &str, tail: &str) -> String {
    let tail = tail.trim().trim_start_matches("::").trim_end_matches("::");
    if prefix.is_empty() {
        tail.to_string()
    } else if tail.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}::{tail}")
    }
}

fn push_path_ref(
    path_node: &ast::Path,
    file: &Path,
    source: &str,
    comments: &[Comment],
    is_use: bool,
    refs: &mut Vec<PathRef>,
) {
    let Some(path_text) = path_to_string(path_node) else {
        return;
    };
    refs.push(PathRef {
        location: location(file, source, path_node.syntax().text_range(), &path_text),
        path: path_text,
        allows: allow_markers(path_node.syntax(), source, comments),
        is_use,
    });
}

fn push_name_ref(
    node: &SyntaxNode,
    file: &Path,
    source: &str,
    comments: &[Comment],
    is_use: bool,
    refs: &mut Vec<PathRef>,
) {
    let path_text = node.text().to_string();
    refs.push(PathRef {
        location: location(file, source, node.text_range(), &path_text),
        path: path_text,
        allows: allow_markers(node, source, comments),
        is_use,
    });
}

fn should_collect_path(path_node: &ast::Path) -> bool {
    if path_node
        .syntax()
        .parent()
        .and_then(ast::Path::cast)
        .is_some()
    {
        return false;
    }
    path_to_string(path_node).is_some()
}

fn path_to_string(path_node: &ast::Path) -> Option<String> {
    let mut parts = Vec::new();
    collect_path_segments(path_node, &mut parts);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("::"))
    }
}

fn collect_path_segments(path_node: &ast::Path, out: &mut Vec<String>) {
    if let Some(qualifier) = path_node.qualifier() {
        collect_path_segments(&qualifier, out);
    }
    if let Some(segment) = path_node.segment()
        && let Some(name) = segment.name_ref()
    {
        out.push(name.text().to_string());
    }
}

fn pub_item(node: &SyntaxNode, file: &Path, source: &str, comments: &[Comment]) -> Option<PubItem> {
    if let Some(item) = pub_named_item::<ast::Struct>(node, "struct", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::Enum>(node, "enum", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::Trait>(node, "trait", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::Fn>(node, "fn", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::TypeAlias>(node, "type", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::Const>(node, "const", file, source, comments) {
        return Some(item);
    }
    if let Some(item) = pub_named_item::<ast::Static>(node, "static", file, source, comments) {
        return Some(item);
    }
    pub_named_item::<ast::Module>(node, "mod", file, source, comments)
}

fn pub_named_item<T>(
    node: &SyntaxNode,
    kind: &str,
    file: &Path,
    source: &str,
    comments: &[Comment],
) -> Option<PubItem>
where
    T: AstNode + HasName + HasVisibility,
{
    let item = T::cast(node.clone())?;
    let visibility = item.visibility()?;
    let name_node = item.name()?;
    let name = name_node.text().to_string();
    let range = visibility.syntax().text_range();
    Some(PubItem {
        name,
        kind: kind.to_string(),
        location: location(file, source, range, &item.syntax().text().to_string()),
        allows: allow_markers(item.syntax(), source, comments),
    })
}

fn location(file: &Path, source: &str, range: TextRange, snippet: &str) -> Location {
    let (line, column) = line_col(source, range.start());
    Location {
        file: file.to_path_buf(),
        line,
        column,
        snippet: snippet.to_string(),
    }
}

#[derive(Debug, Clone)]
struct Comment {
    range: TextRange,
    text: String,
    line: u32,
}

fn collect_comments(root: &SyntaxNode, source: &str) -> Vec<Comment> {
    root.descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::COMMENT)
        .map(|token| Comment {
            line: line_col(source, token.text_range().start()).0,
            range: token.text_range(),
            text: token.text().to_string(),
        })
        .collect()
}

fn allow_markers(node: &SyntaxNode, source: &str, comments: &[Comment]) -> BTreeSet<IssueType> {
    let mut out = BTreeSet::new();
    let range = node.text_range();
    collect_inline_and_inner_markers(source, comments, range, &mut out);
    collect_preceding_markers(source, comments, range.start(), &mut out);
    if let Some(item_range) = enclosing_item_range(node)
        && item_range != range
    {
        collect_preceding_markers(source, comments, item_range.start(), &mut out);
    }
    out
}

fn collect_inline_and_inner_markers(
    source: &str,
    comments: &[Comment],
    range: TextRange,
    out: &mut BTreeSet<IssueType>,
) {
    let (start_line, _) = line_col(source, range.start());
    for comment in comments {
        if range.contains_range(comment.range)
            || (comment.line == start_line
                && comment.range.start() >= range.end()
                && only_horizontal_whitespace(source, range.end(), comment.range.start()))
        {
            collect_marker_types(&comment.text, out);
        }
    }
}

fn collect_preceding_markers(
    source: &str,
    comments: &[Comment],
    target: TextSize,
    out: &mut BTreeSet<IssueType>,
) {
    let mut cursor = target;
    for comment in comments.iter().rev() {
        if comment.range.end() > cursor {
            continue;
        }
        let gap = text_between(source, comment.range.end(), cursor);
        if !gap.chars().all(char::is_whitespace) {
            break;
        }
        if gap.matches('\n').count() > 1 {
            break;
        }
        collect_marker_types(&comment.text, out);
        cursor = comment.range.start();
    }
}

fn only_horizontal_whitespace(source: &str, start: TextSize, end: TextSize) -> bool {
    text_between(source, start, end)
        .chars()
        .all(|ch| ch != '\n' && ch.is_whitespace())
}

fn text_between(source: &str, start: TextSize, end: TextSize) -> &str {
    let start = usize::from(start).min(source.len());
    let end = usize::from(end).min(source.len());
    &source[start..end]
}

fn enclosing_item_range(node: &SyntaxNode) -> Option<TextRange> {
    for ancestor in node.ancestors() {
        if ast::Fn::cast(ancestor.clone()).is_some()
            || ast::Struct::cast(ancestor.clone()).is_some()
            || ast::Enum::cast(ancestor.clone()).is_some()
            || ast::Trait::cast(ancestor.clone()).is_some()
            || ast::TypeAlias::cast(ancestor.clone()).is_some()
            || ast::Const::cast(ancestor.clone()).is_some()
            || ast::Static::cast(ancestor.clone()).is_some()
            || ast::Module::cast(ancestor.clone()).is_some()
            || ast::Use::cast(ancestor.clone()).is_some()
        {
            return Some(ancestor.text_range());
        }
    }
    None
}

fn collect_marker_types(comment: &str, out: &mut BTreeSet<IssueType>) {
    let stripped = strip_comment_delimiters(comment);
    let Some(rest) = stripped.strip_prefix("boundary-allow:") else {
        return;
    };
    let list = rest.split('(').next().unwrap_or(rest);
    let list = list.split("--").next().unwrap_or(list);
    for entry in list.split(',').map(str::trim) {
        match entry {
            "all" => out.extend([
                IssueType::LayerViolation,
                IssueType::InternalCrossing,
                IssueType::PubLeak,
                IssueType::ForbiddenImport,
            ]),
            "layer-violation" => {
                out.insert(IssueType::LayerViolation);
            }
            "internal-crossing" => {
                out.insert(IssueType::InternalCrossing);
            }
            "pub-leak" => {
                out.insert(IssueType::PubLeak);
            }
            "forbidden-import" => {
                out.insert(IssueType::ForbiddenImport);
            }
            _ => {}
        }
    }
}

fn strip_comment_delimiters(comment: &str) -> &str {
    comment
        .trim_start()
        .trim_start_matches("///")
        .trim_start_matches("//!")
        .trim_start_matches("//")
        .trim_start_matches("/**")
        .trim_start_matches("/*!")
        .trim_start_matches("/*")
        .trim_end_matches("*/")
        .trim()
}

fn sort_dedup_refs(refs: &mut Vec<PathRef>) {
    refs.sort_by(|left, right| {
        left.location
            .file
            .cmp(&right.location.file)
            .then(left.location.line.cmp(&right.location.line))
            .then(left.location.column.cmp(&right.location.column))
            .then(left.path.cmp(&right.path))
    });
    refs.dedup_by(|left, right| {
        left.path == right.path
            && left.location.line == right.location.line
            && left.location.column == right.location.column
    });
}

pub fn line_col(source: &str, offset: TextSize) -> (u32, u32) {
    let target = usize::from(offset).min(source.len());
    let mut line = 1u32;
    let mut column = 1u32;
    for (index, ch) in source.char_indices() {
        if index >= target {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_grouped_use() {
        let parsed = parse_source(
            Path::new("x.rs"),
            "use crate::domain::{Order, internal::Secret};",
        );
        let paths: Vec<String> = parsed
            .imports
            .iter()
            .map(|import| import.path.clone())
            .collect();
        assert_eq!(
            paths,
            vec![
                "crate::domain::Order".to_string(),
                "crate::domain::internal::Secret".to_string()
            ]
        );
    }

    #[test]
    fn preceding_allow_comment_matches() {
        let src = "// boundary-allow: layer-violation\nuse crate::infra::Db;";
        let parsed = parse_source(Path::new("x.rs"), src);
        assert!(parsed.allows(2, IssueType::LayerViolation));
    }

    #[test]
    fn ignores_paths_inside_raw_strings() {
        let src = r##"
const FIXTURE: &str = r#"
use crate::domain::internal::Secret;
pub struct Fake;
"#;
use crate::real::Thing;
"##;
        let parsed = parse_source(Path::new("x.rs"), src);
        assert_eq!(parsed.imports.len(), 1);
        assert_eq!(parsed.imports[0].path, "crate::real::Thing");
        assert!(parsed.pub_items.is_empty());
    }

    #[test]
    fn ignores_paths_inside_block_comments() {
        let src = r#"
/*
use crate::domain::internal::Secret;
pub struct Fake;
*/
use crate::real::Thing;
"#;
        let parsed = parse_source(Path::new("x.rs"), src);
        assert_eq!(parsed.imports.len(), 1);
        assert_eq!(parsed.imports[0].path, "crate::real::Thing");
        assert!(parsed.pub_items.is_empty());
    }

    #[test]
    fn collects_generic_path_type_and_calls() {
        let src = r#"
pub struct Service;

fn run(items: Vec<crate::infrastructure::DbHandle>) {
    Service::new();
    items.iter().count();
}
"#;
        let parsed = parse_source(Path::new("x.rs"), src);
        let refs: Vec<String> = parsed
            .path_refs
            .iter()
            .map(|reference| reference.path.clone())
            .collect();
        assert!(refs.contains(&"crate::infrastructure::DbHandle".to_string()));
        assert!(refs.contains(&"Service::new".to_string()));
        assert!(refs.contains(&"iter".to_string()));
    }

    #[test]
    fn non_ascii_columns_are_character_based() {
        let src = "fn main() {\n    let _ = \"日本語\"; crate::infra::Db;\n}\n";
        let parsed = parse_source(Path::new("x.rs"), src);
        let reference = parsed
            .path_refs
            .iter()
            .find(|reference| reference.path == "crate::infra::Db")
            .expect("path ref");
        assert_eq!(reference.location.line, 2);
        assert_eq!(reference.location.column, 20);
    }
}
