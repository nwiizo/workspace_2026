use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::SyntaxKind;

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct NeedlessReturn;

impl LintRule for NeedlessReturn {
    fn id(&self) -> &'static str {
        "needless-return"
    }

    fn description(&self) -> &'static str {
        "Drop trailing `return` from the tail expression of a function/closure body"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(block) = ast::BlockExpr::cast(node) else {
                continue;
            };
            // Only the body block of a function or closure.
            let Some(parent) = block.syntax().parent() else {
                continue;
            };
            if !matches!(parent.kind(), SyntaxKind::FN | SyntaxKind::CLOSURE_EXPR) {
                continue;
            }
            // The tail expression of the block, if any.
            let Some(stmts) = block.stmt_list() else {
                continue;
            };
            // Case 1: tail expression is a `return EXPR;` written as a stmt.
            // Case 2: tail expression slot has a `return EXPR` (no semicolon).
            if let Some(tail) = stmts.tail_expr() {
                if let ast::Expr::ReturnExpr(ret) = tail {
                    let range = ret.syntax().text_range();
                    out.push(Diagnostic::from_range(
                        ctx.file,
                        self.id(),
                        Severity::Note,
                        "trailing `return` is redundant",
                        ctx.source,
                        range,
                        Some(
                            "drop `return` and let the tail expression be the function value"
                                .into(),
                        ),
                    ));
                }
                continue;
            }
            // Case where the last statement is `return X;`.
            let last = stmts.statements().last();
            let ret = match last {
                Some(ast::Stmt::ExprStmt(es)) => match es.expr() {
                    Some(ast::Expr::ReturnExpr(r)) => Some(r),
                    _ => None,
                },
                _ => None,
            };
            if let Some(ret) = ret {
                let range = ret.syntax().text_range();
                out.push(Diagnostic::from_range(
                    ctx.file,
                    self.id(),
                    Severity::Note,
                    "trailing `return` is redundant",
                    ctx.source,
                    range,
                    Some(
                        "drop `return` and the trailing `;` so the value is the tail expression"
                            .into(),
                    ),
                ));
            }
        }
    }
}
