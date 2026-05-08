use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

use super::util::is_in_test_context;

pub struct NoUnwrap;

impl LintRule for NoUnwrap {
    fn id(&self) -> &'static str {
        "no-unwrap"
    }

    fn description(&self) -> &'static str {
        "Disallow .unwrap() in non-test code (rust-best-practices: security.md)"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(method) = ast::MethodCallExpr::cast(node) else {
                continue;
            };
            let Some(name) = method.name_ref() else {
                continue;
            };
            if name.text() != "unwrap" {
                continue;
            }
            if is_in_test_context(method.syntax()) {
                continue;
            }
            let range = method.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Error,
                "`.unwrap()` is forbidden in production code".to_string(),
                ctx.source,
                range,
                Some("use `?`, `.context(...)?`, or `.unwrap_or(...)` instead".into()),
            ));
        }
    }
}
