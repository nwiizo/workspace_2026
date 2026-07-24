use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName, HasVisibility, VisibilityKind};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, TextRange};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub(crate) public_items: Vec<PublicItem>,
    pub(crate) cfg_sites: Vec<CfgSite>,
    pub(crate) compile_error_guards: Vec<BTreeSet<String>>,
    pub(crate) parse_errors: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PublicItem {
    pub(crate) source: String,
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) type_paths: BTreeSet<String>,
    pub(crate) cfg_exprs: Vec<CfgExpr>,
}

#[derive(Debug, Clone)]
pub(crate) struct CfgSite {
    pub(crate) source: String,
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) expr: CfgExpr,
    pub(crate) public_api: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CfgExpr {
    Feature(String),
    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
    Not(Box<CfgExpr>),
    Other,
}

struct ItemDraft {
    kind: &'static str,
    name: Option<String>,
    owner: Option<String>,
    is_public: bool,
    range: TextRange,
    type_paths: BTreeSet<String>,
    cfg_exprs: Vec<CfgExpr>,
}

impl CfgExpr {
    pub(crate) fn features(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Feature(feature) => {
                out.insert(feature.clone());
            }
            Self::All(items) | Self::Any(items) => {
                for item in items {
                    item.features(out);
                }
            }
            Self::Not(item) => item.features(out),
            Self::Other => {}
        }
    }

    pub(crate) fn required_features(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Feature(feature) => {
                out.insert(feature.clone());
            }
            Self::All(items) | Self::Any(items) => {
                for item in items {
                    item.required_features(out);
                }
            }
            Self::Not(_) | Self::Other => {}
        }
    }

    pub(crate) fn not_features(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Not(item) => collect_positive_features(item, out),
            Self::All(items) | Self::Any(items) => {
                for item in items {
                    item.not_features(out);
                }
            }
            Self::Feature(_) | Self::Other => {}
        }
    }

    pub(crate) fn evaluate(&self, enabled: &BTreeSet<String>) -> Option<bool> {
        match self {
            Self::Feature(feature) => Some(enabled.contains(feature)),
            Self::All(items) => evaluate_all(items, enabled),
            Self::Any(items) => evaluate_any(items, enabled),
            Self::Not(item) => item.evaluate(enabled).map(|value| !value),
            Self::Other => None,
        }
    }

    pub(crate) fn display(&self) -> String {
        match self {
            Self::Feature(feature) => format!("feature={feature}"),
            Self::All(items) => format!(
                "all({})",
                items
                    .iter()
                    .map(Self::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Any(items) => format!(
                "any({})",
                items
                    .iter()
                    .map(Self::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Not(item) => format!("not({})", item.display()),
            Self::Other => "other-cfg".to_string(),
        }
    }
}

fn evaluate_all(items: &[CfgExpr], enabled: &BTreeSet<String>) -> Option<bool> {
    let mut saw_unknown = false;
    for item in items {
        match item.evaluate(enabled) {
            Some(true) => {}
            Some(false) => return Some(false),
            None => saw_unknown = true,
        }
    }
    if saw_unknown { None } else { Some(true) }
}

fn evaluate_any(items: &[CfgExpr], enabled: &BTreeSet<String>) -> Option<bool> {
    let mut saw_unknown = false;
    for item in items {
        match item.evaluate(enabled) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => saw_unknown = true,
        }
    }
    if saw_unknown { None } else { Some(false) }
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
    let public_names = public_item_names(tree.syntax());
    let module_names = module_names(tree.syntax());
    let mut public_items = Vec::new();
    let mut cfg_sites = Vec::new();
    let mut compile_error_guards = Vec::new();

    for node in tree.syntax().descendants() {
        if let Some(func) = ast::Fn::cast(node.clone()) {
            collect_item(
                path,
                &rel_path,
                source,
                ItemDraft {
                    kind: "fn",
                    name: func.name().map(|name| name.text().to_string()),
                    owner: owner_qualifier(func.syntax()),
                    is_public: function_public_api(&func, &public_names),
                    range: func.syntax().text_range(),
                    type_paths: fn_signature_type_paths(&func),
                    cfg_exprs: cfg_exprs_for_node(func.syntax(), func.attrs()),
                },
                &mut public_items,
                &mut cfg_sites,
                &module_names,
            );
            continue;
        }
        if let Some(strukt) = ast::Struct::cast(node.clone()) {
            collect_item(
                path,
                &rel_path,
                source,
                ItemDraft {
                    kind: "struct",
                    name: strukt.name().map(|name| name.text().to_string()),
                    owner: None,
                    is_public: public_reachable(&strukt, strukt.syntax()),
                    range: strukt.syntax().text_range(),
                    type_paths: type_paths_under(strukt.syntax(), None),
                    cfg_exprs: cfg_exprs_for_node(strukt.syntax(), strukt.attrs()),
                },
                &mut public_items,
                &mut cfg_sites,
                &module_names,
            );
            continue;
        }
        if let Some(enm) = ast::Enum::cast(node.clone()) {
            collect_item(
                path,
                &rel_path,
                source,
                ItemDraft {
                    kind: "enum",
                    name: enm.name().map(|name| name.text().to_string()),
                    owner: None,
                    is_public: public_reachable(&enm, enm.syntax()),
                    range: enm.syntax().text_range(),
                    type_paths: type_paths_under(enm.syntax(), None),
                    cfg_exprs: cfg_exprs_for_node(enm.syntax(), enm.attrs()),
                },
                &mut public_items,
                &mut cfg_sites,
                &module_names,
            );
            continue;
        }
        if let Some(alias) = ast::TypeAlias::cast(node.clone()) {
            collect_item(
                path,
                &rel_path,
                source,
                ItemDraft {
                    kind: "type",
                    name: alias.name().map(|name| name.text().to_string()),
                    owner: None,
                    is_public: public_reachable(&alias, alias.syntax()),
                    range: alias.syntax().text_range(),
                    type_paths: type_paths_under(alias.syntax(), None),
                    cfg_exprs: cfg_exprs_for_node(alias.syntax(), alias.attrs()),
                },
                &mut public_items,
                &mut cfg_sites,
                &module_names,
            );
            continue;
        }
        if let Some(trait_item) = ast::Trait::cast(node.clone()) {
            collect_item(
                path,
                &rel_path,
                source,
                ItemDraft {
                    kind: "trait",
                    name: trait_item.name().map(|name| name.text().to_string()),
                    owner: None,
                    is_public: public_reachable(&trait_item, trait_item.syntax()),
                    range: trait_item.syntax().text_range(),
                    type_paths: type_paths_under(trait_item.syntax(), None),
                    cfg_exprs: cfg_exprs_for_node(trait_item.syntax(), trait_item.attrs()),
                },
                &mut public_items,
                &mut cfg_sites,
                &module_names,
            );
            continue;
        }
        if node.kind() == SyntaxKind::MACRO_CALL {
            let text = node.text().to_string();
            if text.trim_start().starts_with("compile_error!") {
                let exprs = cfg_exprs_from_ancestors(&node);
                for expr in exprs {
                    let mut features = BTreeSet::new();
                    collect_guard_pair_features(&expr, &mut features);
                    if features.len() >= 2 {
                        compile_error_guards.push(features);
                    }
                }
            }
        }
    }

    ParsedFile {
        public_items,
        cfg_sites,
        compile_error_guards,
        parse_errors,
    }
}

fn collect_item(
    path: &Path,
    rel_path: &str,
    source: &str,
    draft: ItemDraft,
    public_items: &mut Vec<PublicItem>,
    cfg_sites: &mut Vec<CfgSite>,
    module_names: &[(TextRange, String)],
) {
    let name = draft.name.unwrap_or_else(|| draft.kind.to_string());
    let line = line_for_range(source, draft.range);
    let source_id = format!(
        "{rel_path}:{}",
        qualified_item_name(&name, draft.owner.as_deref(), draft.range, module_names)
    );
    for expr in &draft.cfg_exprs {
        cfg_sites.push(CfgSite {
            source: source_id.clone(),
            file: path.to_path_buf(),
            line,
            expr: expr.clone(),
            public_api: draft.is_public,
        });
    }
    if draft.is_public {
        public_items.push(PublicItem {
            source: source_id,
            file: path.to_path_buf(),
            line,
            type_paths: draft.type_paths,
            cfg_exprs: draft.cfg_exprs,
        });
    }
}

fn public_item_names(root: &ra_ap_syntax::SyntaxNode) -> HashSet<String> {
    let mut names = HashSet::new();
    for node in root.descendants() {
        if let Some(strukt) = ast::Struct::cast(node.clone()) {
            if public_reachable(&strukt, strukt.syntax()) && strukt.name().is_some() {
                if let Some(name) = strukt.name() {
                    names.insert(name.text().to_string());
                }
            }
            continue;
        }
        if let Some(enm) = ast::Enum::cast(node.clone()) {
            if public_reachable(&enm, enm.syntax()) && enm.name().is_some() {
                if let Some(name) = enm.name() {
                    names.insert(name.text().to_string());
                }
            }
            continue;
        }
        if let Some(trait_item) = ast::Trait::cast(node) {
            if public_reachable(&trait_item, trait_item.syntax()) && trait_item.name().is_some() {
                if let Some(name) = trait_item.name() {
                    names.insert(name.text().to_string());
                }
            }
        }
    }
    names
}

fn public_reachable<T>(item: &T, node: &ra_ap_syntax::SyntaxNode) -> bool
where
    T: HasVisibility,
{
    item_is_public(item) && enclosing_modules_public(node)
}

fn item_is_public<T>(item: &T) -> bool
where
    T: HasVisibility,
{
    item.visibility()
        .map(|visibility| visibility.kind())
        .map(|kind| matches!(kind, VisibilityKind::Pub))
        .unwrap_or(false)
}

fn enclosing_modules_public(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .all(|module| item_is_public(&module))
}

fn function_public_api(func: &ast::Fn, public_names: &HashSet<String>) -> bool {
    if enclosing_public_trait(func.syntax()) {
        return true;
    }
    if func
        .syntax()
        .ancestors()
        .skip(1)
        .any(|node| ast::Impl::can_cast(node.kind()))
    {
        return item_is_public(func) && enclosing_public_impl(func.syntax(), public_names);
    }
    public_reachable(func, func.syntax())
}

fn enclosing_public_trait(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors()
        .filter_map(ast::Trait::cast)
        .any(|trait_item| public_reachable(&trait_item, trait_item.syntax()))
}

fn enclosing_public_impl(node: &ra_ap_syntax::SyntaxNode, public_names: &HashSet<String>) -> bool {
    node.ancestors().filter_map(ast::Impl::cast).any(|imp| {
        if !enclosing_modules_public(imp.syntax()) {
            return false;
        }
        let text = imp.syntax().text().to_string();
        public_names.iter().any(|name| text.contains(name))
    })
}

fn module_names(root: &ra_ap_syntax::SyntaxNode) -> Vec<(TextRange, String)> {
    root.descendants()
        .filter_map(ast::Module::cast)
        .filter_map(|module| {
            let name = module.name()?.text().to_string();
            Some((module.syntax().text_range(), name))
        })
        .collect()
}

fn qualified_item_name(
    name: &str,
    owner: Option<&str>,
    range: TextRange,
    module_names: &[(TextRange, String)],
) -> String {
    let mut parts = module_names
        .iter()
        .filter_map(|(module_range, module_name)| {
            if module_range.start() <= range.start() && module_range.end() >= range.end() {
                Some(module_name.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if let Some(owner) = owner {
        parts.push(owner.to_string());
    }
    parts.push(name.to_string());
    parts.join("::")
}

fn owner_qualifier(node: &ra_ap_syntax::SyntaxNode) -> Option<String> {
    for ancestor in node.ancestors().skip(1) {
        if let Some(imp) = ast::Impl::cast(ancestor.clone()) {
            return Some(impl_owner(&imp));
        }
        if let Some(trait_item) = ast::Trait::cast(ancestor) {
            return trait_item.name().map(|name| name.text().to_string());
        }
    }
    None
}

fn impl_owner(imp: &ast::Impl) -> String {
    let text = imp.syntax().text().to_string();
    let header = text
        .split('{')
        .next()
        .unwrap_or(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let candidate = header
        .strip_prefix("impl ")
        .unwrap_or(&header)
        .rsplit_once(" for ")
        .map(|(_, self_ty)| self_ty)
        .unwrap_or_else(|| header.strip_prefix("impl").unwrap_or(&header))
        .trim();
    compact_owner(candidate)
}

fn compact_owner(text: &str) -> String {
    text.chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':' || *ch == '<' || *ch == '>'
        })
        .collect::<String>()
}

fn fn_signature_type_paths(func: &ast::Fn) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if let Some(params) = func.param_list() {
        paths.extend(type_paths_under(params.syntax(), None));
    }
    if let Some(ret_type) = func.ret_type() {
        paths.extend(type_paths_under(ret_type.syntax(), None));
    }
    paths
}

fn type_paths_under(
    node: &ra_ap_syntax::SyntaxNode,
    before: Option<ra_ap_syntax::TextSize>,
) -> BTreeSet<String> {
    node.descendants()
        .filter_map(ast::PathType::cast)
        .filter(|path_type| {
            before
                .map(|limit| path_type.syntax().text_range().end() <= limit)
                .unwrap_or(true)
        })
        .filter_map(|path_type| path_type.path())
        .filter_map(|path| path_root_segment(&path))
        .collect()
}

fn path_root_segment(path: &ast::Path) -> Option<String> {
    let mut current = path.clone();
    while let Some(parent) = current.qualifier() {
        current = parent;
    }
    current
        .segment()?
        .name_ref()
        .map(|name| name.text().to_string())
}

fn cfg_exprs_for_node<'a>(
    node: &ra_ap_syntax::SyntaxNode,
    attrs: impl Iterator<Item = ast::Attr> + 'a,
) -> Vec<CfgExpr> {
    let mut exprs = cfg_exprs_from_attrs(attrs);
    exprs.extend(cfg_exprs_from_ancestors(node));
    exprs
}

fn cfg_exprs_from_ancestors(node: &ra_ap_syntax::SyntaxNode) -> Vec<CfgExpr> {
    let mut exprs = Vec::new();
    for ancestor in node.ancestors().skip(1) {
        if let Some(module) = ast::Module::cast(ancestor) {
            exprs.extend(cfg_exprs_from_attrs(module.attrs()));
        }
    }
    exprs
}

fn cfg_exprs_from_attrs(attrs: impl Iterator<Item = ast::Attr>) -> Vec<CfgExpr> {
    attrs
        .filter(|attr| attr.simple_name().as_deref() == Some("cfg"))
        .filter_map(|attr| cfg_expr_from_attr_text(&attr.syntax().text().to_string()))
        .collect()
}

fn cfg_expr_from_attr_text(text: &str) -> Option<CfgExpr> {
    let trimmed = text.trim();
    let start = trimmed.find("cfg(")?;
    let inner_start = start + "cfg(".len();
    let inner_end = matching_close(trimmed, inner_start.saturating_sub(1))?;
    parse_cfg_expr(trimmed.get(inner_start..inner_end)?.trim())
}

fn parse_cfg_expr(text: &str) -> Option<CfgExpr> {
    let text = text.trim();
    if let Some(inner) = call_inner(text, "all") {
        return Some(CfgExpr::All(parse_cfg_list(inner)));
    }
    if let Some(inner) = call_inner(text, "any") {
        return Some(CfgExpr::Any(parse_cfg_list(inner)));
    }
    if let Some(inner) = call_inner(text, "not") {
        return parse_cfg_expr(inner).map(|expr| CfgExpr::Not(Box::new(expr)));
    }
    if let Some(feature) = feature_value(text) {
        return Some(CfgExpr::Feature(feature));
    }
    Some(CfgExpr::Other)
}

fn call_inner<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    if !text.starts_with(&prefix) {
        return None;
    }
    let end = matching_close(text, name.len())?;
    text.get(name.len() + 1..end)
}

fn parse_cfg_list(text: &str) -> Vec<CfgExpr> {
    split_top_level(text)
        .into_iter()
        .filter_map(|item| parse_cfg_expr(item.trim()))
        .collect()
}

fn split_top_level(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if let Some(part) = text.get(start..idx) {
                    parts.push(part);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    if let Some(part) = text.get(start..) {
        parts.push(part);
    }
    parts
}

fn matching_close(text: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in text.char_indices().filter(|(idx, _)| *idx >= open_idx) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn feature_value(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<String>();
    let rest = normalized.strip_prefix("feature=\"")?;
    let end = rest.find('"')?;
    rest.get(..end).map(ToString::to_string)
}

fn collect_positive_features(expr: &CfgExpr, out: &mut BTreeSet<String>) {
    match expr {
        CfgExpr::Feature(feature) => {
            out.insert(feature.clone());
        }
        CfgExpr::All(items) | CfgExpr::Any(items) => {
            for item in items {
                collect_positive_features(item, out);
            }
        }
        CfgExpr::Not(_) | CfgExpr::Other => {}
    }
}

fn collect_guard_pair_features(expr: &CfgExpr, out: &mut BTreeSet<String>) {
    if let CfgExpr::All(items) = expr {
        for item in items {
            if let CfgExpr::Feature(feature) = item {
                out.insert(feature.clone());
            }
        }
    }
}

fn line_for_range(source: &str, range: TextRange) -> usize {
    let offset = u32::from(range.start()) as usize;
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

pub(crate) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .map(|path| path.replace('\\', "/"))
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_parser_handles_feature_not_and_all() {
        let expr = cfg_expr_from_attr_text(
            r#"#[cfg(all(feature = "rt-tokio", not(feature = "rt-async-std")))]"#,
        );
        let Some(expr) = expr else {
            panic!("cfg parsed");
        };
        let mut features = BTreeSet::new();
        let mut not_features = BTreeSet::new();
        expr.features(&mut features);
        expr.not_features(&mut not_features);
        assert!(features.contains("rt-tokio"));
        assert!(not_features.contains("rt-async-std"));
    }

    #[test]
    fn block_comment_cfg_is_not_a_site() {
        let parsed = parse_source(
            Path::new("/tmp/demo"),
            Path::new("/tmp/demo/src/lib.rs"),
            r#"
            /*
            #[cfg(feature = "ghost")]
            pub fn ghost() {}
            */
            pub fn real() {}
            "#,
            Edition::Edition2024,
        );
        assert!(parsed.cfg_sites.is_empty());
    }
}
