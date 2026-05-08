use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct LazyStaticMacro;

impl LintRule for LazyStaticMacro {
    fn id(&self) -> &'static str {
        "lazy-static-macro"
    }

    fn description(&self) -> &'static str {
        "`lazy_static!` is obsolete in Rust 1.80+; use `std::sync::LazyLock` / `OnceLock`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(mac) = ast::MacroCall::cast(node) else {
                continue;
            };
            let Some(path) = mac.path() else { continue };
            let last = path
                .syntax()
                .text()
                .to_string()
                .rsplit("::")
                .next()
                .unwrap_or("")
                .to_string();
            if last != "lazy_static" {
                continue;
            }
            let range = mac.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "`lazy_static!` is obsolete; replace with `std::sync::LazyLock` / `OnceLock`",
                ctx.source,
                range,
                Some(
                    "use `LazyLock::new(|| ...)` for derived constants, `OnceLock` for one-shot \
                     config installation"
                        .into(),
                ),
            ));
        }
    }
}
