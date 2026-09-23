use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName};
use ra_ap_syntax::{Edition, SourceFile, SyntaxNode, TextRange};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::issue::{IssueType, RiskAxis};

mod bypass;
mod casts;
mod downcast;
mod erasure;
mod identifiers;
mod naming;
mod swallow;
mod validation;
mod widen;

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub findings: Vec<Finding>,
    pub parse_errors: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub issue_type: IssueType,
    pub risk: RiskAxis,
    pub condition: u8,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub function: String,
    pub target: String,
    pub message: String,
    pub remediation: String,
}

pub(crate) struct FileContext<'a> {
    pub file: &'a Path,
    pub rel_path: &'a str,
    pub source: &'a str,
    pub aliases: &'a ImportAliases,
    pub function_names: &'a HashMap<TextRange, String>,
    pub config: &'a Config,
}

impl FileContext<'_> {
    pub(crate) fn identity_for(&self, node: &SyntaxNode) -> String {
        enclosing_function(node, self.function_names)
            .unwrap_or_else(|| format!("{}:<module>", self.rel_path))
    }

    pub(crate) fn item_identity(&self, name: &str) -> String {
        format!("{}:{}", self.rel_path, name)
    }

    pub(crate) fn line(&self, range: TextRange) -> usize {
        line_for_range(self.source, range)
    }
}

pub(crate) const CONDITION_LOW: u8 = 1;
pub(crate) const CONDITION_MEDIUM: u8 = 2;
pub(crate) const CONDITION_HIGH: u8 = 3;

#[derive(Debug, Clone, Default)]
pub(crate) struct ImportAliases {
    aliases: HashMap<String, String>,
}

impl ImportAliases {
    fn collect(root: &SyntaxNode) -> Self {
        let mut aliases = Self::default();
        for item in root.descendants().filter_map(ast::Use::cast) {
            if let Some(tree) = item.use_tree() {
                aliases.collect_tree(&tree, "");
            }
        }
        aliases
    }

    fn collect_tree(&mut self, tree: &ast::UseTree, prefix: &str) {
        let local = tree
            .path()
            .map(|path| compact_path(path.syntax().text().to_string()))
            .unwrap_or_default();
        let full = join_path(prefix, &local);
        if let Some(list) = tree.use_tree_list() {
            for child in list.use_trees() {
                self.collect_tree(&child, &full);
            }
            return;
        }
        if full.is_empty() || tree.star_token().is_some() {
            return;
        }
        let alias = tree
            .rename()
            .and_then(|rename| rename.name())
            .map(|name| name.text().to_string())
            .or_else(|| full.rsplit("::").next().map(str::to_string));
        if let Some(alias) = alias.filter(|alias| is_ident(alias)) {
            self.aliases.insert(alias, full);
        }
    }

    pub(crate) fn resolve_path(&self, path: &str) -> String {
        let Some((head, tail)) = path.split_once("::") else {
            return self
                .aliases
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_string());
        };
        self.aliases
            .get(head)
            .map(|prefix| format!("{prefix}::{tail}"))
            .unwrap_or_else(|| path.to_string())
    }
}

pub(crate) fn parse_file(
    root: &Path,
    path: &Path,
    source: &str,
    edition: Edition,
    config: &Config,
) -> ParsedFile {
    let rel_path = relative_path(root, path);
    let parsed = SourceFile::parse(source, edition);
    let tree = parsed.tree();
    let parse_errors = parsed.errors().len();
    let aliases = ImportAliases::collect(tree.syntax());
    let mut id_counts = HashMap::new();
    let function_names = function_names(&rel_path, tree.syntax(), &mut id_counts);
    let ctx = FileContext {
        file: path,
        rel_path: &rel_path,
        source,
        aliases: &aliases,
        function_names: &function_names,
        config,
    };
    let mut findings = Vec::new();
    casts::detect(tree.syntax(), &ctx, &mut findings);
    erasure::detect(tree.syntax(), &ctx, &mut findings);
    downcast::detect(tree.syntax(), &ctx, &mut findings);
    widen::detect(tree.syntax(), &ctx, &mut findings);
    swallow::detect(tree.syntax(), &ctx, &mut findings);
    naming::detect(tree.syntax(), &ctx, &mut findings);
    validation::detect(tree.syntax(), &ctx, &mut findings);
    identifiers::detect(tree.syntax(), &ctx, &mut findings);
    bypass::detect(tree.syntax(), &ctx, &mut findings);

    ParsedFile {
        findings,
        parse_errors,
    }
}

pub(crate) fn parse_file_from_disk(
    root: &Path,
    path: &Path,
    edition: Edition,
    config: &Config,
) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_file(root, path, &source, edition, config))
}

fn function_names(
    rel_path: &str,
    root: &SyntaxNode,
    id_counts: &mut HashMap<String, usize>,
) -> HashMap<TextRange, String> {
    let mut names = HashMap::new();
    for func in root.descendants().filter_map(ast::Fn::cast) {
        let name = func
            .name()
            .map(|name| name.text().to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let base = enclosing_owner_type(func.syntax())
            .map(|ty| format!("{rel_path}:{ty}::{name}"))
            .unwrap_or_else(|| format!("{rel_path}:{name}"));
        let id = disambiguate(id_counts, base);
        names.insert(func.syntax().text_range(), id);
    }
    names
}

pub(crate) fn enclosing_owner_type(node: &SyntaxNode) -> Option<String> {
    for ancestor in node.ancestors().skip(1) {
        if let Some(imp) = ast::Impl::cast(ancestor.clone()) {
            let ty = imp.self_ty()?;
            let text = compact_path(ty.syntax().text().to_string());
            let head = text
                .split(['<', ':'])
                .find(|part| !part.is_empty())
                .unwrap_or(&text);
            return (!head.is_empty()).then(|| head.to_string());
        }
        if let Some(tr) = ast::Trait::cast(ancestor) {
            return tr.name().map(|name| name.text().to_string());
        }
    }
    None
}

pub(crate) fn in_trait_impl(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .find_map(ast::Impl::cast)
        .is_some_and(|imp| imp.trait_().is_some())
}

pub(crate) fn enclosing_function(
    node: &SyntaxNode,
    function_names: &HashMap<TextRange, String>,
) -> Option<String> {
    for ancestor in node.ancestors().skip(1) {
        if ast::Fn::can_cast(ancestor.kind()) {
            return function_names.get(&ancestor.text_range()).cloned();
        }
    }
    None
}

pub(crate) fn excluded_test_context(node: &SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| has_test_attr(func.attrs()) || has_cfg_test_attr(func.attrs()))
            .unwrap_or(false)
            || ast::Module::cast(ancestor)
                .map(|module| has_cfg_test_attr(module.attrs()))
                .unwrap_or(false)
    })
}

fn has_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        let text = compact_path(attr.syntax().text().to_string());
        text == "#[test]"
            || text.starts_with("#[tokio::test")
            || text.starts_with("#[async_std::test")
    })
}

fn has_cfg_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs
        .into_iter()
        .any(|attr| compact_path(attr.syntax().text().to_string()).contains("#[cfg(test)]"))
}

pub(crate) fn visibility_condition(visibility: Option<ast::Visibility>) -> u8 {
    let Some(visibility) = visibility else {
        return CONDITION_LOW;
    };
    let text = compact_path(visibility.syntax().text().to_string());
    if text == "pub" {
        CONDITION_HIGH
    } else {
        CONDITION_MEDIUM
    }
}

pub(crate) fn compact_path(value: String) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn join_path(prefix: &str, local: &str) -> String {
    match (prefix.is_empty(), local.is_empty()) {
        (true, true) => String::new(),
        (true, false) => local.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}::{local}"),
    }
}

pub(crate) fn is_ident(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(crate) fn line_for_range(source: &str, range: TextRange) -> usize {
    let start = u32::from(range.start()) as usize;
    source[..start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn disambiguate(counts: &mut HashMap<String, usize>, base: String) -> String {
    let count = counts.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}#{}", *count)
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
pub(crate) fn parse_fixture(source: &str) -> Vec<Finding> {
    parse_file(
        Path::new("/tmp/example"),
        Path::new("/tmp/example/src/lib.rs"),
        source,
        Edition::Edition2024,
        &Config::default(),
    )
    .findings
}
