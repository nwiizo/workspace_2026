use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasGenericParams, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode, TextRange};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub traits: Vec<TraitInfo>,
    pub impls: Vec<ImplInfo>,
    pub dyn_uses: Vec<DynUse>,
    pub io_dependencies: Vec<IoDependency>,
    pub parse_errors: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    pub name: String,
    pub source: String,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub public: bool,
    pub has_async_trait_attr: bool,
    pub methods: Vec<MethodInfo>,
    pub associated_type_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodInfo {
    pub name: String,
    pub line: usize,
    pub is_async: bool,
    pub has_generic_params: bool,
    pub returns_self: bool,
    pub takes_self_type: bool,
    pub has_where_self_sized: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ImplInfo {
    pub trait_name: Option<String>,
    pub target: String,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub in_test: bool,
    pub broad_blanket: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DynUse {
    pub trait_name: String,
    pub rel_path: String,
    pub line: usize,
    pub public_context: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct IoDependency {
    pub item: String,
    pub concrete_type: String,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
}

pub(crate) fn parse_file(root: &Path, path: &Path, edition: Edition) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_source(root, path, &source, edition))
}

pub(crate) fn parse_source(root: &Path, path: &Path, source: &str, edition: Edition) -> ParsedFile {
    let rel_path = relative_path(root, path);
    let parsed = SourceFile::parse(source, edition);
    let tree = parsed.tree();
    let parse_errors = parsed.errors().len();
    let mut traits = Vec::new();
    let mut impls = Vec::new();
    let mut dyn_uses = Vec::new();
    let mut io_dependencies = Vec::new();

    for node in tree.syntax().descendants() {
        if let Some(trait_item) = ast::Trait::cast(node.clone()) {
            if excluded_cfg_test_context(trait_item.syntax()) {
                continue;
            }
            if let Some(info) = trait_info(path, &rel_path, source, &trait_item) {
                traits.push(info);
            }
            continue;
        }
        if let Some(impl_item) = ast::Impl::cast(node.clone()) {
            if let Some(info) = impl_info(path, &rel_path, source, &impl_item) {
                impls.push(info);
            }
            continue;
        }
        if let Some(dyn_trait) = ast::DynTraitType::cast(node.clone()) {
            if let Some(use_site) = dyn_use(&rel_path, source, &dyn_trait) {
                dyn_uses.push(use_site);
            }
            continue;
        }
        collect_public_io_dependency(path, &rel_path, source, &node, &mut io_dependencies);
    }

    ParsedFile {
        traits,
        impls,
        dyn_uses,
        io_dependencies,
        parse_errors,
    }
}

fn trait_info(
    path: &Path,
    rel_path: &str,
    source: &str,
    trait_item: &ast::Trait,
) -> Option<TraitInfo> {
    let name = trait_item.name()?.text().to_string();
    let mut methods = Vec::new();
    let mut associated_type_count = 0;
    if let Some(list) = trait_item.assoc_item_list() {
        for assoc in list.assoc_items() {
            match assoc {
                ast::AssocItem::Fn(func) => methods.push(method_info(source, &func)),
                ast::AssocItem::TypeAlias(_) => associated_type_count += 1,
                ast::AssocItem::Const(_) | ast::AssocItem::MacroCall(_) => {}
            }
        }
    }
    Some(TraitInfo {
        source: format!("{rel_path}:{name}"),
        name,
        file: path.to_path_buf(),
        rel_path: rel_path.to_string(),
        line: line_for_range(source, trait_item.syntax().text_range()),
        public: naked_pub(trait_item),
        has_async_trait_attr: has_async_trait_attr(trait_item.attrs()),
        methods,
        associated_type_count,
    })
}

fn method_info(source: &str, func: &ast::Fn) -> MethodInfo {
    let name = func
        .name()
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    MethodInfo {
        name,
        line: line_for_range(source, func.syntax().text_range()),
        is_async: func.async_token().is_some(),
        has_generic_params: func.generic_param_list().is_some(),
        returns_self: func
            .ret_type()
            .map(|ret| node_has_ident(ret.syntax(), "Self"))
            .unwrap_or(false),
        takes_self_type: func
            .param_list()
            .map(|params| node_has_ident(params.syntax(), "Self"))
            .unwrap_or(false),
        has_where_self_sized: func
            .where_clause()
            .map(|clause| where_clause_self_sized(clause.syntax()))
            .unwrap_or(false),
    }
}

fn impl_info(path: &Path, rel_path: &str, source: &str, impl_item: &ast::Impl) -> Option<ImplInfo> {
    let line = line_for_range(source, impl_item.syntax().text_range());
    let header = header_tokens_before_body(impl_item.syntax());
    let for_pos = header.iter().position(|token| token == "for")?;
    let before_for = &header[..for_pos];
    let trait_name = last_type_ident(before_for);
    let target = impl_target_ident(&header).unwrap_or_else(|| "<unknown>".to_string());
    let broad_blanket = broad_blanket_bound(&header, &target);
    Some(ImplInfo {
        trait_name,
        target,
        file: path.to_path_buf(),
        rel_path: rel_path.to_string(),
        line,
        in_test: path_is_test(path) || excluded_cfg_test_context(impl_item.syntax()),
        broad_blanket,
    })
}

fn dyn_use(rel_path: &str, source: &str, dyn_trait: &ast::DynTraitType) -> Option<DynUse> {
    let bound_list = dyn_trait.type_bound_list()?;
    let mut names = identifiers(bound_list.syntax())
        .into_iter()
        .filter(|name| !matches!(name.as_str(), "dyn" | "Send" | "Sync" | "Unpin" | "static"))
        .collect::<Vec<_>>();
    names.retain(|name| name.chars().next().map(char::is_uppercase).unwrap_or(false));
    let trait_name = names.into_iter().next()?;
    Some(DynUse {
        trait_name,
        rel_path: rel_path.to_string(),
        line: line_for_range(source, dyn_trait.syntax().text_range()),
        public_context: enclosing_public_api(dyn_trait.syntax()),
    })
}

fn collect_public_io_dependency(
    path: &Path,
    rel_path: &str,
    source: &str,
    node: &SyntaxNode,
    deps: &mut Vec<IoDependency>,
) {
    if excluded_cfg_test_context(node) {
        return;
    }
    let Some(item) = public_api_item(node) else {
        return;
    };
    for path_type in item
        .scan_nodes
        .iter()
        .flat_map(|scan_node| scan_node.descendants())
        .filter_map(ast::PathType::cast)
    {
        let Some(concrete_type) = concrete_io_type(path_type.syntax()) else {
            continue;
        };
        if item.scan_nodes.iter().any(|scan_node| {
            scan_node.descendants().any(|child| {
                ast::DynTraitType::cast(child)
                    .map(|dyn_trait| {
                        dyn_trait
                            .syntax()
                            .text_range()
                            .contains_range(path_type.syntax().text_range())
                    })
                    .unwrap_or(false)
            })
        }) {
            continue;
        }
        deps.push(IoDependency {
            item: item.name.clone(),
            concrete_type,
            file: path.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, path_type.syntax().text_range()),
        });
    }
}

struct PublicApiItem {
    name: String,
    scan_nodes: Vec<SyntaxNode>,
}

fn public_api_item(node: &SyntaxNode) -> Option<PublicApiItem> {
    if let Some(func) = ast::Fn::cast(node.clone()) {
        if naked_pub(&func) {
            let name = func.name()?.text().to_string();
            let mut scan_nodes = Vec::new();
            if let Some(params) = func.param_list() {
                scan_nodes.push(params.syntax().clone());
            }
            if let Some(ret_type) = func.ret_type() {
                scan_nodes.push(ret_type.syntax().clone());
            }
            return Some(PublicApiItem {
                name: qualified_item_name(func.syntax(), name),
                scan_nodes,
            });
        }
    }
    if let Some(strukt) = ast::Struct::cast(node.clone()) {
        if naked_pub(&strukt) {
            let name = strukt.name()?.text().to_string();
            return Some(PublicApiItem {
                name,
                scan_nodes: vec![strukt.syntax().clone()],
            });
        }
    }
    if let Some(enm) = ast::Enum::cast(node.clone()) {
        if naked_pub(&enm) {
            let name = enm.name()?.text().to_string();
            return Some(PublicApiItem {
                name,
                scan_nodes: vec![enm.syntax().clone()],
            });
        }
    }
    if let Some(alias) = ast::TypeAlias::cast(node.clone()) {
        if naked_pub(&alias) {
            let name = alias.name()?.text().to_string();
            return Some(PublicApiItem {
                name,
                scan_nodes: vec![alias.syntax().clone()],
            });
        }
    }
    None
}

fn qualified_item_name(node: &SyntaxNode, name: String) -> String {
    enclosing_impl_type(node)
        .map(|type_name| format!("{type_name}::{name}"))
        .unwrap_or(name)
}

fn concrete_io_type(node: &SyntaxNode) -> Option<String> {
    let names = identifiers(node);
    let suffix = names.join("::");
    let known = [
        "std::fs::File",
        "tokio::fs::File",
        "std::net::TcpStream",
        "std::net::TcpListener",
        "tokio::net::TcpStream",
        "tokio::net::TcpListener",
        "std::process::Command",
        "std::process::Child",
        "std::time::SystemTime",
        "std::time::Instant",
    ];
    known
        .iter()
        .find(|item| suffix.ends_with(**item) || suffix == item.rsplit("::").next().unwrap_or(item))
        .map(|item| (*item).to_string())
}

fn naked_pub<T>(node: &T) -> bool
where
    T: HasVisibility,
{
    node.visibility()
        .map(|visibility| visibility.visibility_inner().is_none())
        .unwrap_or(false)
}

fn excluded_cfg_test_context(node: &SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| has_test_attr(func.attrs()))
            .unwrap_or(false)
            || has_cfg_test_attr_on_node(&ancestor)
    })
}

fn has_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        let text = compact_tokens(attr.syntax());
        text == "#[test]"
            || text.starts_with("#[tokio::test")
            || text.starts_with("#[async_std::test")
    })
}

fn has_cfg_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs
        .into_iter()
        .any(|attr| compact_tokens(attr.syntax()).contains("#[cfg(test)]"))
}

fn has_async_trait_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs
        .into_iter()
        .any(|attr| compact_tokens(attr.syntax()).contains("async_trait"))
}

fn has_cfg_test_attr_on_node(node: &SyntaxNode) -> bool {
    ast::Module::cast(node.clone())
        .map(|module| has_cfg_test_attr(module.attrs()))
        .unwrap_or(false)
        || ast::Impl::cast(node.clone())
            .map(|impl_item| has_cfg_test_attr(impl_item.attrs()))
            .unwrap_or(false)
        || ast::Struct::cast(node.clone())
            .map(|strukt| has_cfg_test_attr(strukt.attrs()))
            .unwrap_or(false)
        || ast::Enum::cast(node.clone())
            .map(|enm| has_cfg_test_attr(enm.attrs()))
            .unwrap_or(false)
        || ast::Trait::cast(node.clone())
            .map(|trait_item| has_cfg_test_attr(trait_item.attrs()))
            .unwrap_or(false)
        || ast::Fn::cast(node.clone())
            .map(|func| has_cfg_test_attr(func.attrs()))
            .unwrap_or(false)
        || ast::TypeAlias::cast(node.clone())
            .map(|alias| has_cfg_test_attr(alias.attrs()))
            .unwrap_or(false)
}

fn path_is_test(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.contains("/tests/")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_tests.rs")
}

fn enclosing_public_api(node: &SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| naked_pub(&func))
            .unwrap_or(false)
            || ast::Struct::cast(ancestor.clone())
                .map(|strukt| naked_pub(&strukt))
                .unwrap_or(false)
            || ast::Enum::cast(ancestor.clone())
                .map(|enm| naked_pub(&enm))
                .unwrap_or(false)
            || ast::Trait::cast(ancestor)
                .map(|trait_item| naked_pub(&trait_item))
                .unwrap_or(false)
    })
}

fn header_tokens_before_body(node: &SyntaxNode) -> Vec<String> {
    let mut tokens = Vec::new();
    for token in node
        .descendants_with_tokens()
        .filter_map(|item| item.into_token())
    {
        if token.kind() == SyntaxKind::L_CURLY {
            break;
        }
        let text = token.text().trim();
        if !text.is_empty() {
            tokens.push(text.to_string());
        }
    }
    tokens
}

fn broad_blanket_bound(header: &[String], target: &str) -> Option<String> {
    let generic_names = generic_param_names(header);
    if !generic_names.iter().any(|name| name == target) {
        return None;
    }
    let mut bounds = generic_bounds(header, target);
    let broad = [
        "Clone", "Copy", "Debug", "Display", "Default", "Send", "Sync", "Sized", "Any",
    ];
    bounds.sort();
    bounds.dedup();
    if bounds
        .iter()
        .any(|bound| !broad.iter().any(|candidate| candidate == bound))
    {
        return None;
    }
    if bounds.is_empty() {
        Some("unconstrained".to_string())
    } else {
        Some(bounds.join(" + "))
    }
}

fn generic_param_names(header: &[String]) -> Vec<String> {
    generic_param_segments(header)
        .iter()
        .filter_map(|segment| first_type_ident(segment))
        .collect()
}

fn generic_bounds(header: &[String], target: &str) -> Vec<String> {
    let mut bounds = Vec::new();
    for segment in generic_param_segments(header) {
        if first_type_ident(&segment).as_deref() == Some(target) {
            if let Some(colon) = segment.iter().position(|token| token == ":") {
                bounds.extend(bound_idents(&segment[colon + 1..], target));
            }
        }
    }
    if let Some(where_pos) = header.iter().position(|token| token == "where") {
        for segment in split_comma_segments(&header[where_pos + 1..]) {
            if first_type_ident(&segment).as_deref() == Some(target) {
                if let Some(colon) = segment.iter().position(|token| token == ":") {
                    bounds.extend(bound_idents(&segment[colon + 1..], target));
                }
            }
        }
    }
    bounds
}

fn generic_param_segments(header: &[String]) -> Vec<Vec<String>> {
    let Some(lt) = header.iter().position(|token| token == "<") else {
        return Vec::new();
    };
    let Some(gt) = header[lt + 1..]
        .iter()
        .position(|token| token == ">")
        .map(|offset| lt + 1 + offset)
    else {
        return Vec::new();
    };
    if gt <= lt {
        return Vec::new();
    }
    split_comma_segments(&header[lt + 1..gt])
}

fn split_comma_segments(tokens: &[String]) -> Vec<Vec<String>> {
    tokens
        .split(|token| token == ",")
        .map(|segment| segment.to_vec())
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn bound_idents(tokens: &[String], target: &str) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| is_type_ident(token) && token.as_str() != target)
        .cloned()
        .collect()
}

fn impl_target_ident(header: &[String]) -> Option<String> {
    if let Some(for_pos) = header.iter().position(|token| token == "for") {
        return first_type_ident(&header[for_pos + 1..]);
    }
    let impl_pos = header.iter().position(|token| token == "impl")?;
    let mut start = impl_pos + 1;
    if header.get(start).map(String::as_str) == Some("<") {
        if let Some(gt_offset) = header[start + 1..].iter().position(|token| token == ">") {
            start += gt_offset + 2;
        }
    }
    first_type_ident(&header[start..])
}

fn enclosing_impl_type(node: &SyntaxNode) -> Option<String> {
    node.ancestors()
        .skip(1)
        .find_map(ast::Impl::cast)
        .and_then(|impl_item| {
            let header = header_tokens_before_body(impl_item.syntax());
            impl_target_ident(&header)
        })
}

fn first_type_ident(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .find(|token| is_type_ident(token))
        .map(|token| token.to_string())
}

fn last_type_ident(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .rev()
        .find(|token| is_type_ident(token))
        .map(|token| token.to_string())
}

fn is_type_ident(token: &str) -> bool {
    token
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic() && ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn identifiers(node: &SyntaxNode) -> Vec<String> {
    node.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .filter(|token| matches!(token.kind(), SyntaxKind::IDENT | SyntaxKind::SELF_TYPE_KW))
        .map(|token| token.text().to_string())
        .collect()
}

fn node_has_ident(node: &SyntaxNode, ident: &str) -> bool {
    identifiers(node).iter().any(|name| name == ident)
}

fn where_clause_self_sized(node: &SyntaxNode) -> bool {
    let names = identifiers(node);
    names.windows(2).any(|window| {
        window.first().map(String::as_str) == Some("Self")
            && window.get(1).map(String::as_str) == Some("Sized")
    })
}

fn compact_tokens(node: &SyntaxNode) -> String {
    node.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .map(|token| token.text().trim().to_string())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

fn line_for_range(source: &str, range: TextRange) -> usize {
    let start = u32::from(range.start()) as usize;
    source[..start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
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
mod tests {
    use super::*;

    #[test]
    fn block_comment_trait_is_not_parsed() {
        let root = Path::new("/tmp/project");
        let parsed = parse_source(
            root,
            &root.join("src/lib.rs"),
            "/* pub trait Fake { fn a(&self); } */\npub trait Real { fn a(&self); }",
            Edition::Edition2024,
        );
        assert_eq!(parsed.traits.len(), 1);
        assert_eq!(parsed.traits[0].name, "Real");
    }

    #[test]
    fn pub_crate_is_not_public() {
        let root = Path::new("/tmp/project");
        let parsed = parse_source(
            root,
            &root.join("src/lib.rs"),
            "pub(crate) trait Internal { fn a(&self); }",
            Edition::Edition2024,
        );
        assert!(!parsed.traits[0].public);
    }
}
