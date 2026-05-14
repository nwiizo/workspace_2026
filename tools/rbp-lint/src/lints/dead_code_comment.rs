use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::{attr_full_text, attr_path_string};
use crate::lints::{LintContext, LintRule};

pub struct DeadCodeComment;

impl LintRule for DeadCodeComment {
    fn id(&self) -> &'static str {
        "dead-code-comment"
    }

    fn description(&self) -> &'static str {
        "`#[allow(dead_code)]` must be justified by a preceding comment"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(attr) = ast::Attr::cast(node) else {
                continue;
            };
            if attr_path_string(&attr) != "allow" {
                continue;
            }
            let attr_text = attr_full_text(&attr);
            if !attr_text.contains("dead_code") {
                continue;
            }
            if has_preceding_comment(attr.syntax()) {
                continue;
            }
            let range = attr.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "`#[allow(dead_code)]` should be preceded by a comment explaining why",
                ctx.source,
                range,
                Some(
                    "add a `// reason: ...` line above this attribute, or remove the dead code"
                        .into(),
                ),
            ));
        }
    }
}

fn has_preceding_comment(node: &ra_ap_syntax::SyntaxNode) -> bool {
    let mut tok = match node.first_token().and_then(|t| t.prev_token()) {
        Some(t) => t,
        None => return false,
    };
    loop {
        match tok.kind() {
            SyntaxKind::WHITESPACE => {
                let text = tok.text();
                // More than one newline means the comment is not adjacent.
                if text.matches('\n').count() > 1 {
                    return false;
                }
            }
            SyntaxKind::COMMENT => return true,
            _ => return false,
        }
        tok = match tok.prev_token() {
            Some(t) => t,
            None => return false,
        };
    }
}
