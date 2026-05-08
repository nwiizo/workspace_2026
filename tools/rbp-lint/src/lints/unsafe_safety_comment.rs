use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::SyntaxKind;

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct UnsafeSafetyComment;

impl LintRule for UnsafeSafetyComment {
    fn id(&self) -> &'static str {
        "unsafe-safety-comment"
    }

    fn description(&self) -> &'static str {
        "Every `unsafe` block must be preceded by a `// SAFETY:` comment"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(block) = ast::BlockExpr::cast(node) else {
                continue;
            };
            let Some(unsafe_tok) = block
                .syntax()
                .children_with_tokens()
                .find_map(|c| c.into_token().filter(|t| t.kind() == SyntaxKind::UNSAFE_KW))
            else {
                continue;
            };
            if has_safety_comment_before(block.syntax()) {
                continue;
            }
            let range = unsafe_tok.text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "`unsafe` block lacks a `// SAFETY:` comment",
                ctx.source,
                range,
                Some(
                    "add `// SAFETY: <invariants the caller must uphold>` immediately above this \
                     block"
                        .into(),
                ),
            ));
        }
    }
}

fn has_safety_comment_before(node: &ra_ap_syntax::SyntaxNode) -> bool {
    let mut tok = match node.first_token().and_then(|t| t.prev_token()) {
        Some(t) => t,
        None => return false,
    };
    loop {
        match tok.kind() {
            SyntaxKind::WHITESPACE => {
                if tok.text().matches('\n').count() > 1 {
                    return false;
                }
            }
            SyntaxKind::COMMENT => {
                let upper = tok.text().to_uppercase();
                return upper.contains("SAFETY");
            }
            _ => return false,
        }
        tok = match tok.prev_token() {
            Some(t) => t,
            None => return false,
        };
    }
}
