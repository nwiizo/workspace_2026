use std::collections::HashSet;

use ra_ap_syntax::ast::{self, AstNode, HasName};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct ArcCloneExplicit;

impl LintRule for ArcCloneExplicit {
    fn id(&self) -> &'static str {
        "arc-clone-explicit"
    }

    fn description(&self) -> &'static str {
        "Prefer `Arc::clone(&x)` over `x.clone()` for `Arc`-typed bindings"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        let arc_bindings = collect_arc_bindings(ctx.tree.syntax());
        if arc_bindings.is_empty() {
            return;
        }
        for node in ctx.tree.syntax().descendants() {
            let Some(method) = ast::MethodCallExpr::cast(node) else {
                continue;
            };
            let Some(name) = method.name_ref() else {
                continue;
            };
            if name.text() != "clone" {
                continue;
            }
            let Some(receiver) = method.receiver() else {
                continue;
            };
            let recv_text = receiver.syntax().text().to_string();
            if !arc_bindings.contains(recv_text.trim()) {
                continue;
            }
            let range = method.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Note,
                format!(
                    "`{recv_text}.clone()` on an `Arc` binding; prefer `Arc::clone(&{recv_text})`"
                ),
                ctx.source,
                range,
                Some(
                    "explicit `Arc::clone(&x)` makes the refcount bump visible vs. a deep clone"
                        .into(),
                ),
            ));
        }
    }
}

/// Walk all `let NAME = Arc::new(...)` bindings and collect the binding names.
/// Heuristic only — we do not model assignments or fields.
fn collect_arc_bindings(root: &ra_ap_syntax::SyntaxNode) -> HashSet<String> {
    let mut out = HashSet::new();
    for node in root.descendants() {
        let Some(let_stmt) = ast::LetStmt::cast(node) else {
            continue;
        };
        let Some(init) = let_stmt.initializer() else {
            continue;
        };
        if !looks_like_arc_constructor(&init) {
            continue;
        }
        let Some(pat) = let_stmt.pat() else { continue };
        if let ast::Pat::IdentPat(ident) = pat {
            if let Some(name) = ident.name() {
                out.insert(name.text().to_string());
            }
        }
    }
    out
}

fn looks_like_arc_constructor(expr: &ast::Expr) -> bool {
    let text = expr.syntax().text().to_string();
    let head = text.trim_start();
    head.starts_with("Arc::new(")
        || head.starts_with("std::sync::Arc::new(")
        || head.starts_with("Arc::from(")
}
