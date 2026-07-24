use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use design_gate_core::{RustFileWalkerOptions, relative_path_string, rust_files};
use ra_ap_syntax::ast::{
    self, AstNode, HasAttrs, HasGenericParams, HasName, HasTypeBounds, HasVisibility,
};
use ra_ap_syntax::{
    Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken, TextRange,
};
use rayon::prelude::*;
use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApiSurface {
    pub root: PathBuf,
    pub files_analyzed: usize,
    pub parse_failures: usize,
    pub items: BTreeMap<String, ApiItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiItem {
    pub id: String,
    pub kind: ItemKind,
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub signature: String,
    pub cfg_attrs: BTreeSet<String>,
    pub bounds: BTreeMap<String, BTreeSet<String>>,
    pub derives: BTreeSet<String>,
    pub repr: Option<String>,
    pub non_exhaustive: bool,
    pub all_fields_public: bool,
    pub is_error_enum: bool,
    pub fields: Vec<ApiMember>,
    pub variants: Vec<ApiMember>,
    pub trait_methods: BTreeMap<String, TraitMethod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    Static,
    ReExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiMember {
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraitMethod {
    pub signature: String,
    pub cfg_attrs: BTreeSet<String>,
    pub bounds: BTreeMap<String, BTreeSet<String>>,
    pub has_default: bool,
    pub line: usize,
}

pub fn parse_path(path: &Path) -> Result<ApiSurface> {
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let files = rust_files(
        &root,
        RustFileWalkerOptions {
            prefer_src: true,
            on_no_files: None,
        },
    )?;
    if files.is_empty() {
        return Err(Error::NoRustFiles(root));
    }

    let parsed: Vec<Result<ParsedFile>> = files
        .par_iter()
        .map(|file| parse_file(&root, file))
        .collect();
    let mut surface = ApiSurface {
        root,
        files_analyzed: 0,
        parse_failures: 0,
        items: BTreeMap::new(),
    };
    for file in parsed {
        let file = file?;
        surface.files_analyzed += 1;
        surface.parse_failures += file.parse_failures;
        if !out_of_line_file_publicly_reachable(&surface.root, &file.path) {
            continue;
        }
        for item in file.items {
            surface.items.insert(item.id.clone(), item);
        }
    }
    Ok(surface)
}

#[derive(Debug, Clone)]
struct ParsedFile {
    path: PathBuf,
    parse_failures: usize,
    items: Vec<ApiItem>,
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
    let parsed = SourceFile::parse(source, Edition::Edition2024);
    let tree = parsed.tree();
    let mut items = Vec::new();

    for node in tree.syntax().descendants() {
        if let Some(func) = ast::Fn::cast(node.clone()) {
            if is_public_function(&func) {
                items.push(fn_item(path, &rel_path, source, &func));
            }
            continue;
        }
        if let Some(strukt) = ast::Struct::cast(node.clone()) {
            if public_reachable(strukt.syntax(), &strukt) {
                items.push(struct_item(path, &rel_path, source, &strukt));
            }
            continue;
        }
        if let Some(enm) = ast::Enum::cast(node.clone()) {
            if public_reachable(enm.syntax(), &enm) {
                items.push(enum_item(path, &rel_path, source, &enm));
            }
            continue;
        }
        if let Some(trait_item) = ast::Trait::cast(node.clone()) {
            if public_reachable(trait_item.syntax(), &trait_item) {
                items.push(trait_item_info(path, &rel_path, source, &trait_item));
            }
            continue;
        }
        if let Some(type_alias) = ast::TypeAlias::cast(node.clone()) {
            if public_reachable(type_alias.syntax(), &type_alias) {
                items.push(type_alias_item(path, &rel_path, source, &type_alias));
            }
            continue;
        }
        if let Some(konst) = ast::Const::cast(node.clone()) {
            if public_reachable(konst.syntax(), &konst) {
                items.push(const_item(path, &rel_path, source, &konst));
            }
            continue;
        }
        if let Some(statik) = ast::Static::cast(node.clone()) {
            if public_reachable(statik.syntax(), &statik) {
                items.push(static_item(path, &rel_path, source, &statik));
            }
            continue;
        }
        if let Some(use_item) = ast::Use::cast(node) {
            if public_reachable(use_item.syntax(), &use_item) {
                items.extend(re_export_items(path, &rel_path, source, &use_item));
            }
        }
    }

    ParsedFile {
        path: path.to_path_buf(),
        parse_failures: parsed.errors().len(),
        items,
    }
}

fn out_of_line_file_publicly_reachable(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    let components = relative.components().collect::<Vec<_>>();
    let Some(src_index) = components
        .iter()
        .rposition(|component| component.as_os_str() == "src")
    else {
        return true;
    };
    let source_relative = components[src_index + 1..]
        .iter()
        .map(|component| component.as_os_str().to_owned())
        .collect::<PathBuf>();
    let Some(file_name) = source_relative.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    if source_relative
        .parent()
        .is_none_or(|parent| parent.as_os_str().is_empty())
        && matches!(file_name, "lib.rs" | "main.rs")
    {
        return true;
    }
    if source_relative
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "bin")
    {
        return true;
    }

    let src_root = root.join(components[..=src_index].iter().collect::<PathBuf>());
    let mut module_parts = source_relative
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|component| component.as_os_str().to_owned())
        .collect::<Vec<_>>();
    if file_name != "mod.rs" {
        let Some(stem) = source_relative.file_stem() else {
            return true;
        };
        module_parts.push(stem.to_owned());
    }
    if module_parts.is_empty() {
        return true;
    }

    for index in 0..module_parts.len() {
        let module_name = module_parts[index].to_string_lossy();
        let parent_file = if index == 0 {
            [src_root.join("lib.rs"), src_root.join("main.rs")]
                .into_iter()
                .find(|candidate| candidate.is_file())
        } else {
            let parent_path = module_parts[..index].iter().collect::<PathBuf>();
            [
                src_root.join(&parent_path).with_extension("rs"),
                src_root.join(&parent_path).join("mod.rs"),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file())
        };
        let Some(parent_file) = parent_file else {
            return false;
        };
        if !declares_public_out_of_line_module(&parent_file, &module_name) {
            return false;
        }
    }
    true
}

fn declares_public_out_of_line_module(path: &Path, module_name: &str) -> bool {
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    SourceFile::parse(&source, Edition::Edition2024)
        .tree()
        .syntax()
        .children()
        .filter_map(ast::Module::cast)
        .any(|module| {
            module.name().is_some_and(|name| name.text() == module_name)
                && module.item_list().is_none()
                && naked_pub(&module)
        })
}

fn fn_item(path: &Path, rel_path: &str, source: &str, func: &ast::Fn) -> ApiItem {
    let name = node_name(func);
    let id = item_id(
        rel_path,
        func.syntax(),
        &name,
        inherent_impl_self(func.syntax()),
    );
    let attrs = attrs_for(func.syntax(), func.attrs());
    let mut item = base_item(
        id,
        ItemKind::Fn,
        name,
        path,
        rel_path,
        line_for_range(source, func.syntax().text_range()),
        fn_signature(func),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(func);
    item
}

fn struct_item(path: &Path, rel_path: &str, source: &str, strukt: &ast::Struct) -> ApiItem {
    let name = node_name(strukt);
    let id = item_id(rel_path, strukt.syntax(), &name, None);
    let attrs = attrs_for(strukt.syntax(), strukt.attrs());
    let mut item = base_item(
        id,
        ItemKind::Struct,
        name,
        path,
        rel_path,
        line_for_range(source, strukt.syntax().text_range()),
        nominal_signature(strukt),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(strukt);
    item.derives = derives(attrs.iter().cloned());
    item.repr = repr_attr(attrs.iter().cloned());
    item.non_exhaustive = has_attr(attrs.iter().cloned(), "non_exhaustive");
    collect_fields(strukt, &mut item);
    item
}

fn enum_item(path: &Path, rel_path: &str, source: &str, enm: &ast::Enum) -> ApiItem {
    let name = node_name(enm);
    let id = item_id(rel_path, enm.syntax(), &name, None);
    let attrs = attrs_for(enm.syntax(), enm.attrs());
    let mut item = base_item(
        id,
        ItemKind::Enum,
        name,
        path,
        rel_path,
        line_for_range(source, enm.syntax().text_range()),
        nominal_signature(enm),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(enm);
    item.derives = derives(attrs.iter().cloned());
    item.repr = repr_attr(attrs.iter().cloned());
    item.non_exhaustive = has_attr(attrs.iter().cloned(), "non_exhaustive");
    item.is_error_enum = item.name.ends_with("Error") || item.derives.iter().any(|d| d == "Error");
    if let Some(list) = enm.variant_list() {
        for variant in list.variants() {
            let name = node_name(&variant);
            item.variants.push(ApiMember {
                name,
                signature: compact_node(variant.syntax()),
            });
        }
    }
    item
}

fn trait_item_info(path: &Path, rel_path: &str, source: &str, trait_item: &ast::Trait) -> ApiItem {
    let name = node_name(trait_item);
    let id = item_id(rel_path, trait_item.syntax(), &name, None);
    let attrs = attrs_for(trait_item.syntax(), trait_item.attrs());
    let mut item = base_item(
        id,
        ItemKind::Trait,
        name,
        path,
        rel_path,
        line_for_range(source, trait_item.syntax().text_range()),
        nominal_signature(trait_item),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(trait_item);
    for func in trait_item.syntax().descendants().filter_map(ast::Fn::cast) {
        let method_name = node_name(&func);
        let attrs = attrs_for(func.syntax(), func.attrs());
        item.trait_methods.insert(
            method_name,
            TraitMethod {
                signature: fn_signature(&func),
                cfg_attrs: cfg_attrs(attrs.iter().cloned()),
                bounds: bounds_for(&func),
                has_default: func.body().is_some(),
                line: line_for_range(source, func.syntax().text_range()),
            },
        );
    }
    item
}

fn type_alias_item(
    path: &Path,
    rel_path: &str,
    source: &str,
    type_alias: &ast::TypeAlias,
) -> ApiItem {
    let name = node_name(type_alias);
    let id = item_id(rel_path, type_alias.syntax(), &name, None);
    let attrs = attrs_for(type_alias.syntax(), type_alias.attrs());
    let mut item = base_item(
        id,
        ItemKind::TypeAlias,
        name,
        path,
        rel_path,
        line_for_range(source, type_alias.syntax().text_range()),
        compact_node_without_attrs(type_alias.syntax()),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(type_alias);
    item
}

fn const_item(path: &Path, rel_path: &str, source: &str, konst: &ast::Const) -> ApiItem {
    let name = node_name(konst);
    let id = item_id(rel_path, konst.syntax(), &name, None);
    let attrs = attrs_for(konst.syntax(), konst.attrs());
    let mut item = base_item(
        id,
        ItemKind::Const,
        name,
        path,
        rel_path,
        line_for_range(source, konst.syntax().text_range()),
        const_static_signature(konst.syntax()),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item.bounds = bounds_for(konst);
    item
}

fn static_item(path: &Path, rel_path: &str, source: &str, statik: &ast::Static) -> ApiItem {
    let name = node_name(statik);
    let id = item_id(rel_path, statik.syntax(), &name, None);
    let attrs = attrs_for(statik.syntax(), statik.attrs());
    let mut item = base_item(
        id,
        ItemKind::Static,
        name,
        path,
        rel_path,
        line_for_range(source, statik.syntax().text_range()),
        const_static_signature(statik.syntax()),
    );
    item.cfg_attrs = cfg_attrs(attrs.iter().cloned());
    item
}

fn re_export_items(path: &Path, rel_path: &str, source: &str, use_item: &ast::Use) -> Vec<ApiItem> {
    let Some(use_tree) = use_item.use_tree() else {
        return Vec::new();
    };
    let mut exports = Vec::new();
    collect_use_tree(&use_tree, "", &mut exports);
    exports
        .into_iter()
        .filter_map(|export| {
            let name = export
                .alias
                .clone()
                .or_else(|| export.path.rsplit("::").next().map(str::to_string))?;
            let id = item_id(rel_path, use_item.syntax(), &name, None);
            Some(base_item(
                id,
                ItemKind::ReExport,
                name,
                path,
                rel_path,
                line_for_range(source, use_item.syntax().text_range()),
                format!("pubuse:{}", export.path),
            ))
        })
        .collect()
}

fn base_item(
    id: String,
    kind: ItemKind,
    name: String,
    _path: &Path,
    rel_path: &str,
    line: usize,
    signature: String,
) -> ApiItem {
    ApiItem {
        id,
        kind,
        name,
        file: PathBuf::from(rel_path),
        line,
        signature,
        cfg_attrs: BTreeSet::new(),
        bounds: BTreeMap::new(),
        derives: BTreeSet::new(),
        repr: None,
        non_exhaustive: false,
        all_fields_public: true,
        is_error_enum: false,
        fields: Vec::new(),
        variants: Vec::new(),
        trait_methods: BTreeMap::new(),
    }
}

fn is_public_function(func: &ast::Fn) -> bool {
    public_reachable(func.syntax(), func) && !is_trait_member(func.syntax())
}

fn is_trait_member(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .any(|ancestor| ast::Trait::cast(ancestor).is_some())
}

fn collect_fields(strukt: &ast::Struct, item: &mut ApiItem) {
    let Some(list) = strukt.field_list() else {
        return;
    };
    match list {
        ast::FieldList::RecordFieldList(list) => {
            let fields = list.fields().collect::<Vec<_>>();
            item.all_fields_public = fields.iter().all(naked_pub);
            for field in fields {
                if !naked_pub(&field) {
                    continue;
                }
                let name = node_name(&field);
                let signature = field
                    .ty()
                    .map(|ty| compact_node(ty.syntax()))
                    .unwrap_or_default();
                item.fields.push(ApiMember { name, signature });
            }
        }
        ast::FieldList::TupleFieldList(list) => {
            let fields = list.fields().collect::<Vec<_>>();
            item.all_fields_public = fields.iter().all(naked_pub);
            for (idx, field) in fields.into_iter().enumerate() {
                if !naked_pub(&field) {
                    continue;
                }
                let signature = field
                    .ty()
                    .map(|ty| compact_node(ty.syntax()))
                    .unwrap_or_default();
                item.fields.push(ApiMember {
                    name: idx.to_string(),
                    signature,
                });
            }
        }
    }
}

fn fn_signature(func: &ast::Fn) -> String {
    let head = head_text(func.syntax());
    let mut signature = normalize_generics_and_where(&head);
    signature = normalize_fn_params(&signature);
    compact_text(&signature)
}

fn nominal_signature<N>(node: &N) -> String
where
    N: AstNode + HasGenericParams,
{
    normalize_generics_and_where(&head_text(node.syntax()))
}

fn const_static_signature(node: &SyntaxNode) -> String {
    let raw = head_text(node);
    let before_value = raw
        .split_once('=')
        .map(|(head, _)| head)
        .unwrap_or(raw.as_str())
        .trim_end_matches(';');
    compact_text(before_value)
}

fn head_text(node: &SyntaxNode) -> String {
    let raw = node.text().to_string();
    let head = raw
        .split_once('{')
        .map(|(head, _)| head)
        .or_else(|| raw.split_once(';').map(|(head, _)| head))
        .unwrap_or(raw.as_str());
    compact_text_without_attrs(head)
}

fn item_id(rel_path: &str, node: &SyntaxNode, name: &str, impl_self: Option<String>) -> String {
    let mut modules = node
        .ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .filter_map(|module| module.name().map(|name| name.text().to_string()))
        .collect::<Vec<_>>();
    modules.reverse();
    if let Some(self_ty) = impl_self {
        modules.push(self_ty);
    }
    if modules.is_empty() {
        format!("{rel_path}:{name}")
    } else {
        format!("{rel_path}:{}::{name}", modules.join("::"))
    }
}

fn node_name<N: HasName>(node: &N) -> String {
    node.name()
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}

fn inherent_impl_self(node: &SyntaxNode) -> Option<String> {
    let impl_block = node.ancestors().skip(1).find_map(ast::Impl::cast)?;
    if impl_block.trait_().is_some() {
        return None;
    }
    impl_block.self_ty().map(|ty| compact_node(ty.syntax()))
}

fn naked_pub<N: HasVisibility>(node: &N) -> bool {
    node.visibility()
        .map(|visibility| visibility.visibility_inner().is_none())
        .unwrap_or(false)
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

fn derives(attrs: impl Iterator<Item = ast::Attr>) -> BTreeSet<String> {
    attrs
        .filter(|attr| attr_path(attr) == "derive")
        .flat_map(|attr| derive_names(&attr.syntax().text().to_string()))
        .collect()
}

fn derive_names(text: &str) -> Vec<String> {
    let inner = text
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')').map(|(inner, _)| inner))
        .unwrap_or("");
    inner
        .split(',')
        .filter_map(|part| {
            let name = part.trim().rsplit("::").next()?.trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn repr_attr(attrs: impl Iterator<Item = ast::Attr>) -> Option<String> {
    attrs
        .filter(|attr| attr_path(attr) == "repr")
        .map(|attr| compact_node(attr.syntax()))
        .next()
}

fn has_attr(attrs: impl Iterator<Item = ast::Attr>, expected: &str) -> bool {
    attrs.into_iter().any(|attr| attr_path(&attr) == expected)
}

fn attr_path(attr: &ast::Attr) -> String {
    let direct = attr
        .path()
        .map(|path| path.syntax().text().to_string())
        .unwrap_or_default();
    if direct.is_empty() {
        attr_path_from_text(&attr.syntax().text().to_string())
    } else {
        direct
    }
}

fn attr_path_from_text(text: &str) -> String {
    let trimmed = text
        .trim()
        .trim_start_matches("#![")
        .trim_start_matches("#[");
    trimmed
        .split(|ch: char| ch == '(' || ch == '=' || ch == ']' || ch.is_whitespace())
        .next()
        .unwrap_or_default()
        .to_string()
}

fn attrs_for(node: &SyntaxNode, direct: impl Iterator<Item = ast::Attr>) -> Vec<ast::Attr> {
    let mut attrs = direct.collect::<Vec<_>>();
    attrs.extend(preceding_attrs(node));
    attrs
}

fn preceding_attrs(node: &SyntaxNode) -> Vec<ast::Attr> {
    let mut attrs = Vec::new();
    let mut previous = node.prev_sibling_or_token();
    while let Some(element) = previous {
        match element {
            NodeOrToken::Node(node) => {
                if let Some(attr) = ast::Attr::cast(node.clone()) {
                    attrs.push(attr);
                    previous = node.prev_sibling_or_token();
                } else {
                    break;
                }
            }
            NodeOrToken::Token(token)
                if matches!(token.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT) =>
            {
                previous = token.prev_sibling_or_token();
            }
            NodeOrToken::Token(_) => break,
        }
    }
    attrs.reverse();
    attrs
}

fn cfg_attrs(attrs: impl Iterator<Item = ast::Attr>) -> BTreeSet<String> {
    attrs
        .filter(|attr| matches!(attr_path(attr).as_str(), "cfg" | "cfg_attr"))
        .map(|attr| compact_node(attr.syntax()))
        .collect()
}

fn bounds_for<N>(node: &N) -> BTreeMap<String, BTreeSet<String>>
where
    N: HasGenericParams,
{
    let mut bounds = BTreeMap::new();
    if let Some(params) = node.generic_param_list() {
        for param in params.type_or_const_params() {
            if let ast::TypeOrConstParam::Type(param) = param
                && let Some(name) = param.name()
            {
                collect_type_bounds(name.text().as_str(), param.type_bound_list(), &mut bounds);
            }
        }
    }
    if let Some(where_clause) = node.where_clause() {
        for predicate in where_clause.predicates() {
            if let Some(ty) = predicate.ty() {
                let name = compact_node(ty.syntax());
                collect_type_bounds(&name, predicate.type_bound_list(), &mut bounds);
            }
        }
    }
    bounds
}

fn collect_type_bounds(
    name: &str,
    list: Option<ast::TypeBoundList>,
    bounds: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let Some(list) = list else {
        return;
    };
    let entry = bounds.entry(name.to_string()).or_default();
    for bound in list.bounds() {
        entry.insert(compact_node(bound.syntax()));
    }
}

fn compact_node(node: &SyntaxNode) -> String {
    compact_text(&node.text().to_string())
}

fn compact_node_without_attrs(node: &SyntaxNode) -> String {
    compact_text_without_attrs(&node.text().to_string())
}

fn compact_text(text: &str) -> String {
    compact_text_inner(text, false)
}

fn compact_text_without_attrs(text: &str) -> String {
    compact_text_inner(text, true)
}

fn compact_text_inner(text: &str, skip_attrs: bool) -> String {
    let parsed = SourceFile::parse(text, Edition::Edition2024);
    let compact = parsed
        .tree()
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| {
            let token = element.into_token()?;
            (!skip_attrs || !has_attr_ancestor(&token)).then_some(token)
        })
        .filter(|token| !matches!(token.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT))
        .map(|token| token.text().to_string())
        .collect::<String>();
    let normalized = normalize_optional_trailing_commas(&compact);
    if normalized.is_empty() {
        normalize_optional_trailing_commas(&text.split_whitespace().collect::<String>())
    } else {
        normalized
    }
}

fn has_attr_ancestor(token: &SyntaxToken) -> bool {
    token
        .parent_ancestors()
        .any(|ancestor| ast::Attr::cast(ancestor).is_some())
}

fn normalize_optional_trailing_commas(text: &str) -> String {
    let mut normalized = text.to_string();
    while normalized.contains(",)") {
        normalized = normalized.replace(",)", ")");
    }
    normalized
}

fn normalize_generics_and_where(text: &str) -> String {
    let without_where = remove_where_clause(text);
    normalize_generic_params(&without_where)
}

fn remove_where_clause(text: &str) -> String {
    let compact = compact_text(text);
    let Some(where_pos) = find_top_level_keyword(&compact, "where") else {
        return compact;
    };
    compact[..where_pos].to_string()
}

fn normalize_generic_params(text: &str) -> String {
    let Some(start) = text.find('<') else {
        return text.to_string();
    };
    let Some(end) = matching_delimiter(text, start, '<', '>') else {
        return text.to_string();
    };
    let params = split_top_level(&text[start + 1..end], ',')
        .into_iter()
        .map(|param| strip_type_param_bounds(&param))
        .collect::<Vec<_>>()
        .join(",");
    format!("{}<{}>{}", &text[..start], params, &text[end + 1..])
}

fn strip_type_param_bounds(param: &str) -> String {
    let trimmed = param.trim();
    if trimmed.starts_with("const") || trimmed.starts_with('\'') {
        return trimmed.to_string();
    }
    let colon = find_top_level_char(trimmed, ':');
    let eq = find_top_level_char(trimmed, '=');
    match (colon, eq) {
        (Some(colon), Some(eq)) if eq > colon => {
            format!("{}{}", &trimmed[..colon], &trimmed[eq..])
        }
        (Some(colon), _) => trimmed[..colon].to_string(),
        _ => trimmed.to_string(),
    }
}

fn normalize_fn_params(signature: &str) -> String {
    let Some(start) = signature.find('(') else {
        return signature.to_string();
    };
    let Some(end) = matching_delimiter(signature, start, '(', ')') else {
        return signature.to_string();
    };
    let params = split_top_level(&signature[start + 1..end], ',')
        .into_iter()
        .filter(|param| !param.trim().is_empty())
        .map(|param| normalize_fn_param(&param))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}({}){}",
        &signature[..start],
        params,
        &signature[end + 1..]
    )
}

fn normalize_fn_param(param: &str) -> String {
    let trimmed = param.trim();
    let Some(colon) = find_top_level_char(trimmed, ':') else {
        return trimmed.to_string();
    };
    format!("_:{}", &trimmed[colon + 1..])
}

fn find_top_level_keyword(text: &str, keyword: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let needle = keyword.as_bytes();
    let mut depth = 0i32;
    for idx in 0..bytes.len() {
        match bytes[idx] {
            b'<' | b'(' | b'[' | b'{' => depth += 1,
            b'>' | b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && bytes[idx..].starts_with(needle) {
            return Some(idx);
        }
    }
    None
}

fn matching_delimiter(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    for (idx, ch) in text.char_indices().skip_while(|(idx, _)| *idx < start) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn split_top_level(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    for (idx, ch) in text.char_indices() {
        match ch {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if ch == separator && depth == 0 {
            parts.push(text[start..idx].trim().to_string());
            start = idx + ch.len_utf8();
        }
    }
    parts.push(text[start..].trim().to_string());
    parts
}

fn find_top_level_char(text: &str, needle: char) -> Option<usize> {
    let mut depth = 0i32;
    for (idx, ch) in text.char_indices() {
        match ch {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if ch == needle && depth == 0 {
            return Some(idx);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct UseExport {
    path: String,
    alias: Option<String>,
}

fn collect_use_tree(use_tree: &ast::UseTree, prefix: &str, out: &mut Vec<UseExport>) {
    let path = use_tree
        .path()
        .map(|path| compact_node(path.syntax()))
        .unwrap_or_default();
    let next_prefix = join_path(prefix, &path);
    if let Some(list) = use_tree.use_tree_list() {
        for child in list.use_trees() {
            collect_use_tree(&child, &next_prefix, out);
        }
        return;
    }
    if next_prefix.is_empty() || use_tree.star_token().is_some() {
        return;
    }
    let alias = use_tree
        .rename()
        .and_then(|rename| rename.name())
        .map(|name| name.text().to_string());
    out.push(UseExport {
        path: next_prefix,
        alias,
    });
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

fn line_for_range(source: &str, range: TextRange) -> usize {
    let start = u32::from(range.start()) as usize;
    source[..start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn naked_pub_excludes_pub_crate() {
        let root = Path::new("/tmp/api");
        let path = root.join("src/lib.rs");
        let parsed = parse_source(root, &path, "pub(crate) fn hidden(a: u8) {}");
        assert!(parsed.items.is_empty());
    }

    #[test]
    fn comments_do_not_change_signature() {
        assert_eq!(
            compact_text("pub fn a(\n  /// doc\n  x: u8,\n) -> u8;"),
            compact_text("pub fn a(x:u8)->u8;")
        );
    }

    #[test]
    fn cfg_attrs_are_collected() {
        let root = Path::new("/tmp/api");
        let path = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &path,
            "#[cfg(feature = \"fast\")]\npub fn parse(input: &str) -> usize { input.len() }",
        );
        assert_eq!(parsed.items.len(), 1);
        assert!(!parsed.items[0].cfg_attrs.is_empty());
    }

    #[test]
    fn out_of_line_private_modules_are_not_public_api() {
        let root = TempDir::new().expect("tempdir");
        let src = root.path().join("src");
        fs::create_dir_all(&src).expect("src");
        fs::write(
            src.join("lib.rs"),
            "mod private;\npub mod public;\npub fn root() {}\n",
        )
        .expect("lib");
        fs::write(src.join("private.rs"), "pub fn hidden() {}\n").expect("private");
        fs::write(src.join("public.rs"), "pub fn visible() {}\n").expect("public");

        let surface = parse_path(root.path()).expect("surface");
        assert!(
            surface.items.values().any(|item| item.name == "root"),
            "crate-root API must remain visible"
        );
        assert!(
            surface.items.values().any(|item| item.name == "visible"),
            "pub out-of-line module API must be visible"
        );
        assert!(
            surface.items.values().all(|item| item.name != "hidden"),
            "private out-of-line module API must be excluded"
        );
    }
}
