use ra_ap_syntax::ast::{self, AstNode, HasArgList};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::is_in_test_context;
use crate::lints::{LintContext, LintRule};

pub struct NoExpect;

impl LintRule for NoExpect {
    fn id(&self) -> &'static str {
        "no-expect"
    }

    fn description(&self) -> &'static str {
        "Disallow .expect() in non-test code; flag empty messages always"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(method) = ast::MethodCallExpr::cast(node) else {
                continue;
            };
            let Some(name) = method.name_ref() else {
                continue;
            };
            if name.text() != "expect" {
                continue;
            }

            let in_tests = is_in_test_context(method.syntax());
            let arg_text = method
                .arg_list()
                .map(|a| a.syntax().text().to_string())
                .unwrap_or_default();
            let empty_msg = arg_text
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim()
                .trim_matches('"')
                .is_empty();

            // Always flag empty messages.
            if empty_msg {
                let range = method.syntax().text_range();
                out.push(Diagnostic::from_range(
                    ctx.file,
                    self.id(),
                    Severity::Warning,
                    "`.expect(\"\")` has an empty message; explain the invariant",
                    ctx.source,
                    range,
                    Some("supply a message describing why this cannot fail".into()),
                ));
                continue;
            }

            if in_tests {
                continue;
            }
            let range = method.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "`.expect()` should be avoided in production code",
                ctx.source,
                range,
                Some("propagate with `?` or use `.unwrap_or(...)` / `.context(...)?`".into()),
            ));
        }
    }
}
