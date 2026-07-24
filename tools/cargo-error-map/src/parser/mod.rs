use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use design_gate_core::relative_path_string;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName, HasVisibility, VisibilityKind};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, TextRange};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub is_lib_reachable: bool,
    pub functions: Vec<FunctionInfo>,
    pub public_signatures: Vec<SignatureInfo>,
    pub enums: Vec<EnumInfo>,
    pub panic_sites: Vec<PanicSite>,
    pub parse_errors: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub id: String,
    pub name: String,
    pub file: PathBuf,
    pub rel_path: String,
    pub module_path: String,
    pub line: usize,
    pub is_public: bool,
    pub is_boundary: bool,
    pub has_question: bool,
    pub has_context: bool,
    pub callees: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SignatureInfo {
    pub source: String,
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumInfo {
    pub name: String,
    pub source: String,
    pub file: PathBuf,
    pub line: usize,
    pub variant_count: usize,
    pub derives_thiserror: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PanicSite {
    pub kind: String,
    pub file: PathBuf,
    pub line: usize,
    pub function: String,
    pub is_boundary: bool,
}

pub(crate) fn parse_file(
    root: &Path,
    path: &Path,
    edition: Edition,
    is_lib_reachable: bool,
    is_boundary: bool,
) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_source(
        root,
        path,
        &source,
        edition,
        is_lib_reachable,
        is_boundary,
    ))
}

pub(crate) fn parse_source(
    root: &Path,
    path: &Path,
    source: &str,
    edition: Edition,
    is_lib_reachable: bool,
    is_boundary: bool,
) -> ParsedFile {
    let rel_path = relative_path_string(root, path);
    let module_path = module_path(&rel_path);
    let parsed = SourceFile::parse(source, edition);
    let tree = parsed.tree();
    let parse_errors = parsed.errors().len();
    let mut public_names = public_item_names(tree.syntax());

    for trait_item in tree.syntax().descendants().filter_map(ast::Trait::cast) {
        if public_reachable(&trait_item) {
            if let Some(name) = trait_item.name() {
                public_names.insert(name.text().to_string());
            }
        }
    }

    let mut id_counts = HashMap::new();
    let mut functions = Vec::new();
    let mut public_signatures = Vec::new();
    let mut enums = Vec::new();
    let mut panic_sites = Vec::new();

    for node in tree.syntax().descendants() {
        if let Some(func) = ast::Fn::cast(node.clone()) {
            let mut info = function_info(
                path,
                &rel_path,
                &module_path,
                source,
                &func,
                is_boundary,
                &public_names,
            );
            info.id = disambiguate(&mut id_counts, format!("{}:{}", rel_path, info.name));
            if is_lib_reachable && info.is_public {
                public_signatures.push(SignatureInfo {
                    source: info.id.clone(),
                    file: path.to_path_buf(),
                    line: info.line,
                    text: item_signature(func.syntax().text().to_string()),
                });
            }
            functions.push(info);
            continue;
        }
        if let Some(type_alias) = ast::TypeAlias::cast(node.clone()) {
            if is_lib_reachable && public_reachable(&type_alias) {
                let name = type_alias
                    .name()
                    .map(|name| name.text().to_string())
                    .unwrap_or_else(|| "type".to_string());
                public_signatures.push(SignatureInfo {
                    source: disambiguate(&mut id_counts, format!("{rel_path}:{name}")),
                    file: path.to_path_buf(),
                    line: line_for_range(source, type_alias.syntax().text_range()),
                    text: item_signature(type_alias.syntax().text().to_string()),
                });
            }
            continue;
        }
        if let Some(strukt) = ast::Struct::cast(node.clone()) {
            if is_lib_reachable && public_reachable(&strukt) {
                let name = strukt
                    .name()
                    .map(|name| name.text().to_string())
                    .unwrap_or_else(|| "struct".to_string());
                public_signatures.push(SignatureInfo {
                    source: disambiguate(&mut id_counts, format!("{rel_path}:{name}")),
                    file: path.to_path_buf(),
                    line: line_for_range(source, strukt.syntax().text_range()),
                    text: compact_item_text(strukt.syntax().text().to_string()),
                });
            }
            continue;
        }
        if let Some(enm) = ast::Enum::cast(node.clone()) {
            let mut enum_info = enum_info(path, source, &enm);
            enum_info.source =
                disambiguate(&mut id_counts, format!("{rel_path}:{}", enum_info.name));
            if is_lib_reachable && public_reachable(&enm) {
                public_signatures.push(SignatureInfo {
                    source: enum_info.source.clone(),
                    file: path.to_path_buf(),
                    line: enum_info.line,
                    text: compact_item_text(enm.syntax().text().to_string()),
                });
            }
            enums.push(enum_info);
        }
    }

    for node in tree.syntax().descendants() {
        if excluded_test_context(&node) {
            continue;
        }
        if let Some(method) = ast::MethodCallExpr::cast(node.clone()) {
            let Some(name) = method.name_ref() else {
                continue;
            };
            let kind = name.text().to_string();
            if kind != "unwrap" && kind != "expect" {
                continue;
            }
            panic_sites.push(PanicSite {
                kind,
                file: path.to_path_buf(),
                line: line_for_range(source, method.syntax().text_range()),
                function: enclosing_function_id(&method.syntax().clone(), &functions, source)
                    .unwrap_or_else(|| format!("{rel_path}:<module>")),
                is_boundary,
            });
            continue;
        }
        if node.kind() == SyntaxKind::MACRO_CALL {
            let text = node.text().to_string();
            if text.trim_start().starts_with("panic!") {
                panic_sites.push(PanicSite {
                    kind: "panic!".to_string(),
                    file: path.to_path_buf(),
                    line: line_for_range(source, node.text_range()),
                    function: enclosing_function_id(&node, &functions, source)
                        .unwrap_or_else(|| format!("{rel_path}:<module>")),
                    is_boundary,
                });
            }
        }
    }

    ParsedFile {
        is_lib_reachable,
        functions,
        public_signatures,
        enums,
        panic_sites,
        parse_errors,
    }
}

fn function_info(
    path: &Path,
    rel_path: &str,
    module_path: &str,
    source: &str,
    func: &ast::Fn,
    is_boundary: bool,
    public_names: &HashSet<String>,
) -> FunctionInfo {
    let name = func
        .name()
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let line = line_for_range(source, func.syntax().text_range());
    let body_text = func
        .body()
        .map(|body| body.syntax().text().to_string())
        .unwrap_or_default();
    let callees = collect_callees(func);
    FunctionInfo {
        id: String::new(),
        name,
        file: path.to_path_buf(),
        rel_path: rel_path.to_string(),
        module_path: module_path.to_string(),
        line,
        is_public: naked_pub(func)
            || enclosing_public_trait(func.syntax())
            || enclosing_public_impl(func.syntax(), public_names),
        is_boundary,
        has_question: body_text.contains('?'),
        has_context: body_text.contains(".context(") || body_text.contains(".with_context("),
        callees,
    }
}

fn enum_info(path: &Path, source: &str, enm: &ast::Enum) -> EnumInfo {
    let name = enm
        .name()
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let variant_count = enm
        .variant_list()
        .map(|list| list.variants().count())
        .unwrap_or(0);
    let derives_thiserror = enm.attrs().any(|attr| {
        let text = attr.syntax().text().to_string();
        text.contains("thiserror::Error") || text.contains("Error")
    });
    EnumInfo {
        name,
        source: String::new(),
        file: path.to_path_buf(),
        line: line_for_range(source, enm.syntax().text_range()),
        variant_count,
        derives_thiserror,
    }
}

fn collect_callees(func: &ast::Fn) -> Vec<String> {
    let mut callees = Vec::new();
    for node in func.syntax().descendants() {
        if let Some(call) = ast::CallExpr::cast(node.clone()) {
            if let Some(expr) = call.expr() {
                if let Some(name) = call_name(expr.syntax().text().to_string()) {
                    callees.push(name);
                }
            }
            continue;
        }
        if let Some(method) = ast::MethodCallExpr::cast(node) {
            if let Some(name) = method.name_ref() {
                callees.push(name.text().to_string());
            }
        }
    }
    callees.sort();
    callees.dedup();
    callees
}

fn call_name(raw: String) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let last = trimmed.rsplit("::").next().unwrap_or(trimmed);
    let name = last
        .trim()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_');
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn item_signature(text: String) -> String {
    let head = text.split('{').next().unwrap_or(text.as_str());
    compact_item_text(head.to_string())
}

fn compact_item_text(text: String) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn line_for_range(source: &str, range: TextRange) -> usize {
    let start = u32::from(range.start()) as usize;
    source[..start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn enclosing_function_id(
    node: &ra_ap_syntax::SyntaxNode,
    functions: &[FunctionInfo],
    source: &str,
) -> Option<String> {
    for ancestor in node.ancestors() {
        let Some(func) = ast::Fn::cast(ancestor) else {
            continue;
        };
        let name = func.name()?.text().to_string();
        let line = line_for_range(source, func.syntax().text_range());
        return functions
            .iter()
            .find(|candidate| candidate.name == name && candidate.line == line)
            .map(|candidate| candidate.id.clone());
    }
    None
}

fn public_item_names(root: &ra_ap_syntax::SyntaxNode) -> HashSet<String> {
    let mut names = HashSet::new();
    for node in root.descendants() {
        if let Some(strukt) = ast::Struct::cast(node.clone()) {
            if public_reachable(&strukt) {
                if let Some(name) = strukt.name() {
                    names.insert(name.text().to_string());
                }
            }
            continue;
        }
        if let Some(enm) = ast::Enum::cast(node.clone()) {
            if public_reachable(&enm) {
                if let Some(name) = enm.name() {
                    names.insert(name.text().to_string());
                }
            }
            continue;
        }
        if let Some(trait_item) = ast::Trait::cast(node) {
            if public_reachable(&trait_item) {
                if let Some(name) = trait_item.name() {
                    names.insert(name.text().to_string());
                }
            }
        }
    }
    names
}

fn public_reachable<T>(node: &T) -> bool
where
    T: HasVisibility,
{
    naked_pub(node)
}

fn naked_pub<T>(node: &T) -> bool
where
    T: HasVisibility,
{
    node.visibility()
        .map(|visibility| matches!(visibility.kind(), VisibilityKind::Pub))
        .unwrap_or(false)
}

fn enclosing_public_trait(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors().skip(1).any(|ancestor| {
        ast::Trait::cast(ancestor)
            .map(|trait_item| public_reachable(&trait_item))
            .unwrap_or(false)
    })
}

fn enclosing_public_impl(node: &ra_ap_syntax::SyntaxNode, public_names: &HashSet<String>) -> bool {
    node.ancestors().skip(1).any(|ancestor| {
        let Some(impl_item) = ast::Impl::cast(ancestor) else {
            return false;
        };
        let text = impl_item.syntax().text().to_string();
        public_names.iter().any(|name| contains_ident(&text, name))
    })
}

fn contains_ident(text: &str, ident: &str) -> bool {
    text.match_indices(ident).any(|(idx, _)| {
        let before = text[..idx].chars().next_back();
        let after = text[idx + ident.len()..].chars().next();
        !before
            .map(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            .unwrap_or(false)
            && !after
                .map(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                .unwrap_or(false)
    })
}

fn excluded_test_context(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| has_test_attr(func.attrs()))
            .unwrap_or(false)
            || ast::Module::cast(ancestor)
                .map(|module| has_cfg_test_attr(module.attrs()))
                .unwrap_or(false)
    })
}

fn has_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        let text = strip_whitespace(&attr.syntax().text().to_string());
        text == "#[test]"
            || text.starts_with("#[tokio::test")
            || text.starts_with("#[async_std::test")
    })
}

fn has_cfg_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs
        .into_iter()
        .any(|attr| strip_whitespace(&attr.syntax().text().to_string()).contains("#[cfg(test)]"))
}

fn strip_whitespace(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
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

fn module_path(rel_path: &str) -> String {
    rel_path
        .trim_start_matches("src/")
        .trim_end_matches(".rs")
        .trim_end_matches("/mod")
        .replace('/', "::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naked_pub_excludes_pub_crate() {
        let root = Path::new("/tmp/example");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            "pub(crate) fn leak() -> anyhow::Result<()> { Ok(()) }",
            Edition::Edition2021,
            true,
            false,
        );
        assert!(!parsed.functions[0].is_public);
    }
}
