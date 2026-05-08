use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::is_in_test_context;
use crate::lints::{LintContext, LintRule};

pub struct DebugPrint;

const DEBUG_MACROS: &[&str] = &["println", "print", "eprintln", "eprint", "dbg"];

impl LintRule for DebugPrint {
    fn id(&self) -> &'static str {
        "debug-print"
    }

    fn description(&self) -> &'static str {
        "Avoid `println!`/`dbg!` in production code; use `tracing` instead"
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
            if !DEBUG_MACROS.contains(&last.as_str()) {
                continue;
            }
            if is_in_test_context(mac.syntax()) {
                continue;
            }
            // `main.rs` of binaries is a common acceptable place for `println!`
            // — leave that judgement to the reader and just warn.
            let range = mac.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                format!("`{last}!` should not appear in library code; use `tracing` macros"),
                ctx.source,
                range,
                Some("`tracing::info!`/`debug!`/`error!` route through subscribers and structured fields".into()),
            ));
        }
    }
}
