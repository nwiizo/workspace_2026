use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{SyntaxKind, SyntaxNode, TextSize};

use crate::issue::{IssueType, RiskAxis};

use super::erasure::{ErasureKind, classify_type};
use super::{CONDITION_HIGH, FileContext, Finding, compact_path, excluded_test_context, is_ident};

const ANY_EXTRACTORS: &[&str] = &["downcast_ref", "downcast_mut", "downcast", "is"];
const JSON_EXTRACTORS: &[&str] = &[
    "as_str",
    "as_u64",
    "as_i64",
    "as_f64",
    "as_bool",
    "as_array",
    "as_object",
    "get",
    "pointer",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidenKind {
    Any,
    Json,
}

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::WidenThenAssert) {
        return;
    }
    for func in root.descendants().filter_map(ast::Fn::cast) {
        if excluded_test_context(func.syntax()) {
            continue;
        }
        let Some(body) = func.body() else {
            continue;
        };
        for let_stmt in body.syntax().descendants().filter_map(ast::LetStmt::cast) {
            let Some((kind, display)) = widened_binding(&let_stmt, ctx) else {
                continue;
            };
            let Some(name) = binding_name(&let_stmt) else {
                continue;
            };
            let after = let_stmt.syntax().text_range().end();
            if !reasserted_later(body.syntax(), &name, kind, after) {
                continue;
            }
            findings.push(Finding {
                issue_type: IssueType::WidenThenAssert,
                risk: RiskAxis::Evidence,
                condition: CONDITION_HIGH,
                file: ctx.file.to_path_buf(),
                rel_path: ctx.rel_path.to_string(),
                line: ctx.line(let_stmt.syntax().text_range()),
                function: ctx.identity_for(let_stmt.syntax()),
                target: name.clone(),
                message: format!(
                    "`{name}` widens a known value to `{display}` and later re-asserts it"
                ),
                remediation:
                    "Keep the concrete type; if a wide type is needed at a boundary, convert at the boundary and do not read it back."
                        .to_string(),
            });
        }
    }
}

// Values born erased by parsing external input are boundary parsing, the
// pattern anti-slop recommends, not a widening of locally known evidence.
const BOUNDARY_PARSERS: &[&str] = &["from_str", "from_slice", "from_reader", ".parse"];

fn widened_binding(let_stmt: &ast::LetStmt, ctx: &FileContext<'_>) -> Option<(WidenKind, String)> {
    let initializer = let_stmt.initializer()?;
    let initializer_text = compact_path(initializer.syntax().text().to_string());
    if let Some(ty) = let_stmt.ty() {
        let erasure = classify_type(&ty, ctx)?;
        if initializer_text.contains("downcast")
            || BOUNDARY_PARSERS
                .iter()
                .any(|marker| initializer_text.contains(marker))
        {
            return None;
        }
        return match erasure.kind {
            ErasureKind::Any => Some((WidenKind::Any, erasure.display)),
            ErasureKind::Value => Some((WidenKind::Json, erasure.display)),
            ErasureKind::Dictionary => None,
        };
    }
    if let ast::Expr::MacroExpr(macro_expr) = &initializer {
        let path = macro_expr
            .macro_call()
            .and_then(|call| call.path())
            .map(|path| compact_path(path.syntax().text().to_string()))?;
        let resolved = ctx.aliases.resolve_path(&path);
        if matches!(resolved.as_str(), "json" | "serde_json::json") {
            return Some((WidenKind::Json, "serde_json::Value".to_string()));
        }
    }
    if let ast::Expr::CallExpr(call) = &initializer {
        let path = call
            .expr()
            .map(|expr| compact_path(expr.syntax().text().to_string()))?;
        let resolved = ctx.aliases.resolve_path(&path);
        if matches!(resolved.as_str(), "serde_json::to_value" | "to_value") {
            return Some((WidenKind::Json, "serde_json::Value".to_string()));
        }
    }
    None
}

fn binding_name(let_stmt: &ast::LetStmt) -> Option<String> {
    let pat = let_stmt.pat()?;
    let text = compact_path(pat.syntax().text().to_string());
    let name = text.strip_prefix("mut").unwrap_or(&text).to_string();
    is_ident(&name).then_some(name)
}

fn reasserted_later(body: &SyntaxNode, name: &str, kind: WidenKind, after: TextSize) -> bool {
    let extractors: &[&str] = match kind {
        WidenKind::Any => ANY_EXTRACTORS,
        WidenKind::Json => JSON_EXTRACTORS,
    };
    for node in body.descendants() {
        if node.text_range().start() <= after {
            continue;
        }
        if kind == WidenKind::Json
            && node.kind() == SyntaxKind::INDEX_EXPR
            && compact_path(node.text().to_string()).starts_with(&format!("{name}["))
        {
            return true;
        }
        let Some(method) = ast::MethodCallExpr::cast(node) else {
            continue;
        };
        let Some(method_name) = method.name_ref().map(|method| method.text().to_string()) else {
            continue;
        };
        if !extractors.contains(&method_name.as_str()) {
            continue;
        }
        let receiver = method
            .receiver()
            .map(|receiver| compact_path(receiver.syntax().text().to_string()))
            .unwrap_or_default();
        if receiver == name
            || receiver.starts_with(&format!("{name}."))
            || receiver.starts_with(&format!("{name}["))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn json_roundtrip_in_one_function_is_reported() {
        let findings = parse_fixture(
            r#"
            use serde_json::json;
            fn port() -> String {
                let config = json!({ "port": 8080 });
                config["port"].to_string()
            }
            "#,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::WidenThenAssert)
        );
    }

    #[test]
    fn boundary_parsed_value_is_not_reported() {
        let findings = parse_fixture(
            r#"
            fn edition(source: &str) -> Option<String> {
                let value: toml::Value = toml::from_str(source).ok()?;
                value.get("package")?.as_str().map(str::to_string)
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::WidenThenAssert)
        );
    }

    #[test]
    fn concrete_value_without_reassert_is_not_reported() {
        let findings = parse_fixture(
            r#"
            use serde_json::json;
            fn payload() -> serde_json::Value {
                let body = json!({ "ok": true });
                body
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::WidenThenAssert)
        );
    }
}
