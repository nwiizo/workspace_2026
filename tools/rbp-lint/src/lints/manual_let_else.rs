use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct ManualLetElse;

impl LintRule for ManualLetElse {
    fn id(&self) -> &'static str {
        "manual-let-else"
    }

    fn description(&self) -> &'static str {
        "`if let X = e { body } else { return/break/continue/panic }` should be `let-else`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(if_expr) = ast::IfExpr::cast(node) else {
                continue;
            };
            // Must be `if let PAT = EXPR { ... } else { ... }`.
            let Some(condition) = if_expr.condition() else {
                continue;
            };
            // The `condition()` accessor returns `Option<Expr>` even for
            // `if let`, but the structure is `let PAT = EXPR`. The simplest
            // way to detect the let form is to look at the source text.
            let cond_text = condition.syntax().text().to_string();
            let trimmed = cond_text.trim_start();
            if !trimmed.starts_with("let ") && !trimmed.starts_with("let\t") {
                continue;
            }
            // No `else if`; we want a plain `else { ... }`.
            let Some(ast::ElseBranch::Block(else_block)) = if_expr.else_branch() else {
                continue;
            };
            // Skip if the if-branch itself is empty — that's a different smell.
            if !block_diverges(&else_block) {
                continue;
            }
            // Only flag if the `if let` is used as a statement (not as a value).
            let parent_kind = if_expr.syntax().parent().map(|p| p.kind());
            let in_stmt = matches!(
                parent_kind,
                Some(ra_ap_syntax::SyntaxKind::EXPR_STMT)
                    | Some(ra_ap_syntax::SyntaxKind::STMT_LIST)
            );
            if !in_stmt {
                continue;
            }
            let range = if_expr.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Note,
                "`if let ... else { return/break/... }` is better written as `let-else`",
                ctx.source,
                range,
                Some(
                    "rewrite as `let PAT = EXPR else { return ...; };` to flatten the success path"
                        .into(),
                ),
            ));
        }
    }
}

/// Heuristic: the block ends with a diverging expression (return / break /
/// continue / panic-like macro / `loop { }` without break).
fn block_diverges(block: &ast::BlockExpr) -> bool {
    let text = block.syntax().text().to_string();
    let body = text
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    let last_line = body.lines().last().unwrap_or("").trim();
    let stripped = last_line.trim_end_matches(';').trim();
    stripped.starts_with("return")
        || stripped == "break"
        || stripped.starts_with("break ")
        || stripped == "continue"
        || stripped.starts_with("panic!")
        || stripped.starts_with("todo!")
        || stripped.starts_with("unimplemented!")
        || stripped.starts_with("unreachable!")
        || stripped.starts_with("std::process::exit")
        || stripped.starts_with("process::exit")
}
