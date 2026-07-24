use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use design_gate_core::{RustFileWalkerOptions, relative_path_string, rust_files};
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasGenericParams, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode, TextRange};
use rayon::prelude::*;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct SourceAnalysis {
    pub files_analyzed: usize,
    pub parse_errors: usize,
    pub module_graph: Vec<ModuleSummary>,
    pub public_api: Vec<ApiItem>,
    pub total_public_api: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleSummary {
    pub name: String,
    pub path: String,
    pub pub_item_count: usize,
    pub major_types: Vec<String>,
    pub children: Vec<ModuleSummary>,
}

#[derive(Debug, Clone)]
pub struct ApiItem {
    pub module: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub file: String,
    pub line: usize,
    pub fan_in: usize,
}

#[derive(Debug, Clone, Default)]
struct ModuleStats {
    pub_item_count: usize,
    major_types: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
struct ParsedFile {
    parse_errors: usize,
    module_stats: BTreeMap<String, ModuleStats>,
    api_items: Vec<ApiItem>,
    call_names: Vec<String>,
}

pub fn analyze_source(root: &Path, top: usize) -> Result<SourceAnalysis> {
    let files = rust_files(
        root,
        RustFileWalkerOptions {
            prefer_src: true,
            on_no_files: None,
        },
    )?;
    if files.is_empty() {
        return Ok(SourceAnalysis::default());
    }
    let parsed = files
        .par_iter()
        .map(|file| parse_file(root, file))
        .collect::<Vec<_>>();

    let mut analysis = SourceAnalysis {
        files_analyzed: parsed.len(),
        ..SourceAnalysis::default()
    };
    let mut module_stats = BTreeMap::<String, ModuleStats>::new();
    let mut api_items = Vec::new();
    let mut fan_in = HashMap::<String, usize>::new();

    for file in parsed {
        let file = file?;
        analysis.parse_errors += file.parse_errors;
        for (module, stats) in file.module_stats {
            let entry = module_stats.entry(module).or_default();
            entry.pub_item_count += stats.pub_item_count;
            entry.major_types.extend(stats.major_types);
        }
        for call in file.call_names {
            let counter = fan_in.entry(call).or_insert(0);
            *counter += 1;
        }
        api_items.extend(file.api_items);
    }

    for item in &mut api_items {
        item.fan_in = fan_in.get(&item.name).copied().unwrap_or(0);
    }
    api_items.sort_by(|a, b| {
        b.fan_in
            .cmp(&a.fan_in)
            .then_with(|| a.module.cmp(&b.module))
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    analysis.total_public_api = api_items.len();
    api_items.truncate(top);
    analysis.public_api = api_items;
    analysis.module_graph = build_module_graph(module_stats);
    Ok(analysis)
}

fn parse_file(root: &Path, path: &Path) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_source(root, path, &source))
}

fn parse_source(root: &Path, path: &Path, source: &str) -> ParsedFile {
    let rel_path = relative_path_string(root, path);
    let file_module = module_path_for_rel(&rel_path);
    let parsed = SourceFile::parse(source, Edition::Edition2024);
    let tree = parsed.tree();
    let mut file = ParsedFile {
        parse_errors: parsed.errors().len(),
        ..ParsedFile::default()
    };

    for node in tree.syntax().descendants() {
        if excluded_test_context(&node) {
            continue;
        }
        collect_call_names(&node, &mut file.call_names);
        if let Some(item) = public_api_item(&file_module, &rel_path, source, &node) {
            let stats = file.module_stats.entry(item.module.clone()).or_default();
            stats.pub_item_count += 1;
            if item.kind == "struct" || item.kind == "enum" || item.kind == "trait" {
                stats
                    .major_types
                    .insert(format!("{} {}", item.kind, item.name));
            }
            file.api_items.push(item);
        }
    }
    file.call_names.sort();
    file
}

fn public_api_item(
    file_module: &str,
    rel_path: &str,
    source: &str,
    node: &SyntaxNode,
) -> Option<ApiItem> {
    if let Some(func) = ast::Fn::cast(node.clone())
        && public_reachable(func.syntax(), &func)
        && !is_trait_member(func.syntax())
        && !is_impl_member(func.syntax())
    {
        return Some(api_item(
            file_module,
            rel_path,
            source,
            func.syntax(),
            "fn",
            node_name(&func),
            fn_signature(&func),
        ));
    }
    if let Some(strukt) = ast::Struct::cast(node.clone())
        && public_reachable(strukt.syntax(), &strukt)
    {
        return Some(api_item(
            file_module,
            rel_path,
            source,
            strukt.syntax(),
            "struct",
            node_name(&strukt),
            nominal_signature(&strukt),
        ));
    }
    if let Some(enm) = ast::Enum::cast(node.clone())
        && public_reachable(enm.syntax(), &enm)
    {
        return Some(api_item(
            file_module,
            rel_path,
            source,
            enm.syntax(),
            "enum",
            node_name(&enm),
            nominal_signature(&enm),
        ));
    }
    if let Some(trait_item) = ast::Trait::cast(node.clone())
        && public_reachable(trait_item.syntax(), &trait_item)
    {
        return Some(api_item(
            file_module,
            rel_path,
            source,
            trait_item.syntax(),
            "trait",
            node_name(&trait_item),
            nominal_signature(&trait_item),
        ));
    }
    None
}

fn api_item(
    file_module: &str,
    rel_path: &str,
    source: &str,
    syntax: &SyntaxNode,
    kind: &str,
    name: String,
    signature: String,
) -> ApiItem {
    ApiItem {
        module: module_for_node(file_module, syntax),
        kind: kind.to_string(),
        name,
        signature,
        file: rel_path.to_string(),
        line: line_for_range(source, syntax.text_range()),
        fan_in: 0,
    }
}

fn build_module_graph(stats: BTreeMap<String, ModuleStats>) -> Vec<ModuleSummary> {
    let mut top = BTreeMap::<String, ModuleSummary>::new();
    for (path, stats) in stats {
        let parts = path.split("::").collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }
        let top_name = parts[0].to_string();
        let entry = top
            .entry(top_name.clone())
            .or_insert_with(|| ModuleSummary {
                name: top_name.clone(),
                path: top_name.clone(),
                ..ModuleSummary::default()
            });
        if parts.len() == 1 {
            entry.pub_item_count += stats.pub_item_count;
            entry.major_types.extend(stats.major_types);
        } else {
            let child_name = parts[1].to_string();
            if let Some(child) = entry
                .children
                .iter_mut()
                .find(|child| child.name == child_name)
            {
                child.pub_item_count += stats.pub_item_count;
                child.major_types.extend(stats.major_types);
            } else {
                entry.children.push(ModuleSummary {
                    name: child_name.clone(),
                    path: format!("{top_name}::{child_name}"),
                    pub_item_count: stats.pub_item_count,
                    major_types: stats.major_types.into_iter().collect(),
                    children: Vec::new(),
                });
            }
        }
    }
    let mut modules = top.into_values().collect::<Vec<_>>();
    for module in &mut modules {
        module.major_types.sort();
        module.major_types.truncate(6);
        module.children.sort_by(|a, b| a.path.cmp(&b.path));
        for child in &mut module.children {
            child.major_types.sort();
            child.major_types.truncate(6);
        }
    }
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    modules
}

fn module_path_for_rel(rel_path: &str) -> String {
    let stripped = rel_path.strip_prefix("src/").unwrap_or(rel_path);
    let without_rs = stripped.strip_suffix(".rs").unwrap_or(stripped);
    let parts = without_rs
        .split('/')
        .filter(|part| *part != "lib" && *part != "main" && *part != "mod")
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "crate".to_string()
    } else {
        parts.join("::")
    }
}

fn module_for_node(file_module: &str, node: &SyntaxNode) -> String {
    let mut inline = node
        .ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .filter_map(|module| module.name().map(|name| name.text().to_string()))
        .collect::<Vec<_>>();
    inline.reverse();
    if inline.is_empty() {
        return file_module.to_string();
    }
    if file_module == "crate" {
        inline.join("::")
    } else {
        format!("{file_module}::{}", inline.join("::"))
    }
}

fn collect_call_names(node: &SyntaxNode, names: &mut Vec<String>) {
    if let Some(call) = ast::CallExpr::cast(node.clone()) {
        if let Some(expr) = call.expr()
            && let Some(name) = last_identifier(expr.syntax())
        {
            names.push(name);
        }
        return;
    }
    if let Some(method) = ast::MethodCallExpr::cast(node.clone())
        && let Some(name) = method.name_ref()
    {
        names.push(name.text().to_string());
    }
}

fn fn_signature(func: &ast::Fn) -> String {
    compact_head(func.syntax())
}

fn nominal_signature<N>(node: &N) -> String
where
    N: AstNode + HasGenericParams,
{
    compact_head(node.syntax())
}

fn compact_head(node: &SyntaxNode) -> String {
    let raw = node.text().to_string();
    let head = raw
        .split_once('{')
        .map(|(head, _)| head)
        .or_else(|| raw.split_once(';').map(|(head, _)| head))
        .unwrap_or(raw.as_str());
    compact_text_without_attrs(head)
}

fn compact_text_without_attrs(text: &str) -> String {
    let mut skip_attr = false;
    let without_attrs = text.lines().fold(String::new(), |mut acc, line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[") {
            skip_attr = !trimmed.contains(']');
            return acc;
        }
        if skip_attr {
            if trimmed.contains(']') {
                skip_attr = false;
            }
            return acc;
        }
        acc.push_str(line);
        acc.push(' ');
        acc
    });
    compact_text(&without_attrs)
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn line_for_range(source: &str, range: TextRange) -> usize {
    let offset = usize::from(range.start());
    source[..offset.min(source.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}

fn public_reachable<N: HasVisibility>(syntax: &SyntaxNode, node: &N) -> bool {
    naked_pub(node) && public_module_chain(syntax)
}

fn public_module_chain(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .all(|module| naked_pub(&module))
}

fn naked_pub<N: HasVisibility>(node: &N) -> bool {
    node.visibility()
        .map(|visibility| visibility.visibility_inner().is_none())
        .unwrap_or(false)
}

fn excluded_test_context(node: &SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| has_test_attr(func.attrs()) || has_cfg_test_attr(func.attrs()))
            .unwrap_or(false)
            || ast::Module::cast(ancestor.clone())
                .map(|module| has_cfg_test_attr(module.attrs()))
                .unwrap_or(false)
            || ast::Struct::cast(ancestor.clone())
                .map(|item| has_cfg_test_attr(item.attrs()))
                .unwrap_or(false)
            || ast::Enum::cast(ancestor.clone())
                .map(|item| has_cfg_test_attr(item.attrs()))
                .unwrap_or(false)
            || ast::Trait::cast(ancestor)
                .map(|item| has_cfg_test_attr(item.attrs()))
                .unwrap_or(false)
    })
}

fn has_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        let path = attr_path(&attr);
        path == ["test"] || path == ["tokio", "test"] || path == ["async_std", "test"]
    })
}

fn has_cfg_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        attr.simple_name().as_deref() == Some("cfg")
            && attr
                .meta()
                .and_then(|meta| match meta {
                    ast::Meta::CfgMeta(meta) => Some(meta.syntax().clone()),
                    ast::Meta::TokenTreeMeta(meta) => {
                        meta.token_tree().map(|tree| tree.syntax().clone())
                    }
                    _ => None,
                })
                .map(|tree| contains_ident(&tree, "test"))
                .unwrap_or(false)
    })
}

fn attr_path(attr: &ast::Attr) -> Vec<String> {
    attr.path()
        .map(|path| identifier_path(path.syntax()))
        .unwrap_or_default()
}

fn contains_ident(node: &SyntaxNode, expected: &str) -> bool {
    node.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .any(|token| token.kind() == SyntaxKind::IDENT && token.text() == expected)
}

fn last_identifier(node: &SyntaxNode) -> Option<String> {
    identifier_path(node).into_iter().last()
}

fn identifier_path(node: &SyntaxNode) -> Vec<String> {
    node.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .filter(|token| token.kind() == SyntaxKind::IDENT)
        .map(|token| token.text().to_string())
        .collect()
}

fn node_name<N: HasName>(node: &N) -> String {
    node.name()
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}

fn is_trait_member(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .any(|ancestor| ast::Trait::cast(ancestor).is_some())
}

fn is_impl_member(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .any(|ancestor| ast::Impl::cast(ancestor).is_some())
}

#[allow(dead_code)]
fn _keep_pathbuf_for_rustdoc(_: PathBuf) {}
