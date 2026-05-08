use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::is_in_test_context;
use crate::lints::{LintContext, LintRule};

pub struct NoPanic;

impl LintRule for NoPanic {
    fn id(&self) -> &'static str {
        "no-panic"
    }

    fn description(&self) -> &'static str {
        "Disallow `panic!`, `unimplemented!`, `todo!` macros in non-test code"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        const FORBIDDEN: &[&str] = &["panic", "unimplemented", "todo", "unreachable"];
        for node in ctx.tree.syntax().descendants() {
            let Some(mac) = ast::MacroCall::cast(node) else {
                continue;
            };
            let Some(path) = mac.path() else { continue };
            let path_text = path.syntax().text().to_string();
            let last = path_text.rsplit("::").next().unwrap_or("");
            if !FORBIDDEN.contains(&last) {
                continue;
            }
            if is_in_test_context(mac.syntax()) {
                continue;
            }
            let range = mac.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                format!("`{last}!` should not appear in production code"),
                ctx.source,
                range,
                Some("return a `Result` instead of panicking".into()),
            ));
        }
    }
}
