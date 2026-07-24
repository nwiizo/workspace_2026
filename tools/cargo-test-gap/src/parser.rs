use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use design_gate_core::relative_path_string;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode, TextRange};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub functions: Vec<FunctionInfo>,
    pub parse_errors: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub is_public: bool,
    pub is_test: bool,
    pub returns_result: bool,
    pub complexity: usize,
    pub callees: Vec<String>,
}

pub fn parse_file(root: &Path, path: &Path, edition: Edition) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_source(root, path, &source, edition))
}

pub fn parse_source(root: &Path, path: &Path, source: &str, edition: Edition) -> ParsedFile {
    let rel_path = relative_path_string(root, path);
    let parsed = SourceFile::parse(source, edition);
    let tree = parsed.tree();
    let parse_errors = parsed.errors().len();
    let mut id_counts = HashMap::new();
    let mut functions = Vec::new();
    for func in tree.syntax().descendants().filter_map(ast::Fn::cast) {
        if func.body().is_none() {
            continue;
        }
        let Some(name) = func.name().map(|name| name.text().to_string()) else {
            continue;
        };
        let owner = enclosing_impl_type(func.syntax());
        let qualified_name = owner
            .as_ref()
            .map(|owner| format!("{owner}::{name}"))
            .unwrap_or_else(|| name.clone());
        let id = disambiguate(&mut id_counts, format!("{rel_path}:{qualified_name}"));
        let line = line_for_range(source, func.syntax().text_range());
        functions.push(FunctionInfo {
            id,
            name,
            qualified_name,
            file: path.to_path_buf(),
            rel_path: rel_path.clone(),
            line,
            is_public: naked_pub(&func),
            is_test: excluded_test_context(func.syntax()),
            returns_result: returns_result(&func),
            complexity: complexity(&func),
            callees: collect_callees(&func),
        });
    }
    ParsedFile {
        functions,
        parse_errors,
    }
}

fn complexity(func: &ast::Fn) -> usize {
    let mut score = 1usize;
    for node in func.syntax().descendants() {
        if belongs_to_nested_fn(func.syntax(), &node) {
            continue;
        }
        if ast::IfExpr::cast(node.clone()).is_some()
            || ast::LoopExpr::cast(node.clone()).is_some()
            || ast::WhileExpr::cast(node.clone()).is_some()
            || ast::ForExpr::cast(node.clone()).is_some()
        {
            score += 1;
            continue;
        }
        if let Some(match_expr) = ast::MatchExpr::cast(node.clone()) {
            // Match arms are counted directly rather than as "arms - 1" to keep
            // the score intentionally conservative for broad branching code.
            score += match_expr
                .match_arm_list()
                .map(|list| list.arms().count())
                .unwrap_or(1);
            continue;
        }
        if let Some(bin) = ast::BinExpr::cast(node)
            && matches!(bin.op_kind(), Some(ra_ap_syntax::ast::BinaryOp::LogicOp(_)))
        {
            score += 1;
        }
    }
    score
}

fn collect_callees(func: &ast::Fn) -> Vec<String> {
    let mut callees = Vec::new();
    for node in func.syntax().descendants() {
        if belongs_to_nested_fn(func.syntax(), &node) {
            continue;
        }
        if let Some(call) = ast::CallExpr::cast(node.clone()) {
            if let Some(expr) = call.expr() {
                callees.extend(call_names(expr.syntax()));
            }
            continue;
        }
        if let Some(method) = ast::MethodCallExpr::cast(node)
            && let Some(name) = method.name_ref()
        {
            callees.push(name.text().to_string());
        }
    }
    callees.sort();
    callees.dedup();
    callees
}

fn call_names(node: &SyntaxNode) -> Vec<String> {
    let idents = identifier_path(node);
    let mut names = Vec::new();
    if let Some(last) = idents.last() {
        names.push(last.clone());
    }
    if idents.len() > 1 {
        names.push(idents.join("::"));
    }
    names
}

fn returns_result(func: &ast::Fn) -> bool {
    let Some(ret) = func.ret_type() else {
        return false;
    };
    identifier_path(ret.syntax())
        .iter()
        .any(|ident| ident == "Result")
}

fn identifier_path(node: &SyntaxNode) -> Vec<String> {
    node.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .filter(|token| token.kind() == SyntaxKind::IDENT)
        .map(|token| token.text().to_string())
        .collect()
}

fn excluded_test_context(node: &SyntaxNode) -> bool {
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

fn naked_pub<T>(node: &T) -> bool
where
    T: HasVisibility,
{
    node.visibility()
        .map(|visibility| visibility.visibility_inner().is_none())
        .unwrap_or(false)
}

fn enclosing_impl_type(node: &SyntaxNode) -> Option<String> {
    let impl_item = node.ancestors().skip(1).find_map(ast::Impl::cast)?;
    let self_ty = impl_item.self_ty()?;
    let mut idents = Vec::new();
    for token in self_ty
        .syntax()
        .descendants_with_tokens()
        .filter_map(|item| item.into_token())
    {
        if token.kind() == SyntaxKind::L_ANGLE {
            break;
        }
        if token.kind() == SyntaxKind::IDENT {
            idents.push(token.text().to_string());
        }
    }
    idents.last().cloned()
}

fn belongs_to_nested_fn(root: &SyntaxNode, node: &SyntaxNode) -> bool {
    if node != root && ast::Fn::cast(node.clone()).is_some() {
        return true;
    }
    node.ancestors()
        .skip(1)
        .take_while(|ancestor| ancestor != root)
        .any(|ancestor| ast::Fn::cast(ancestor).is_some())
}

fn line_for_range(source: &str, range: TextRange) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_test_attr_uses_attr_path_and_excludes_function() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
#[cfg(test)]
mod tests {
    fn helper() {}
}
"#,
            Edition::Edition2024,
        );
        assert!(parsed.functions[0].is_test);
    }

    #[test]
    fn block_comment_code_does_not_count() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
pub fn getter() -> usize {
    /*
    if true { loop {} }
    pub fn fake() {}
    */
    1
}
"#,
            Edition::Edition2024,
        );
        assert_eq!(parsed.functions.len(), 1);
        assert_eq!(parsed.functions[0].complexity, 1);
    }

    #[test]
    fn cfg_test_attr_on_top_level_function_is_excluded() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
#[cfg(test)]
fn helper() {}

pub fn production() {}
"#,
            Edition::Edition2024,
        );
        assert!(parsed.functions.iter().any(|function| function.is_test));
        assert_eq!(
            parsed
                .functions
                .iter()
                .filter(|function| !function.is_test)
                .count(),
            1
        );
    }

    #[test]
    fn trait_method_declaration_without_body_is_not_a_candidate() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
trait Named {
    fn name(&self) -> &str;
}

impl Named for Bar {
    fn name(&self) -> &str { "bar" }
}
"#,
            Edition::Edition2024,
        );
        assert_eq!(parsed.functions.len(), 1);
        assert_eq!(parsed.functions[0].qualified_name, "Bar::name");
    }

    #[test]
    fn impl_owner_uses_self_ty_for_generics_and_where_clauses() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
impl<T> Wrapper<T> {
    fn get(&self) {}
}

impl Named for Bar where Bar: Sized {
    fn name(&self) {}
}
"#,
            Edition::Edition2024,
        );
        let names = parsed
            .functions
            .iter()
            .map(|function| function.qualified_name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Wrapper::get"));
        assert!(names.contains(&"Bar::name"));
    }

    #[test]
    fn nested_local_fn_is_not_counted_in_outer_complexity_or_callees() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = parse_source(
            root,
            &file,
            r#"
pub fn outer() {
    fn inner() {
        if true {
            expensive();
        }
    }
    inner();
}
"#,
            Edition::Edition2024,
        );
        let outer = parsed
            .functions
            .iter()
            .find(|function| function.name == "outer")
            .expect("outer");
        let inner = parsed
            .functions
            .iter()
            .find(|function| function.name == "inner")
            .expect("inner");
        assert_eq!(outer.complexity, 1);
        assert_eq!(outer.callees, vec!["inner"]);
        assert_eq!(inner.complexity, 2);
    }
}
