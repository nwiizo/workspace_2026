use ra_ap_syntax::ast::{self, AstNode, HasArgList};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::is_in_test_context;
use crate::lints::{LintContext, LintRule};

pub struct StringAsError;

impl LintRule for StringAsError {
    fn id(&self) -> &'static str {
        "string-as-error"
    }

    fn description(&self) -> &'static str {
        "Avoid stringly-typed errors: prefer typed errors via `thiserror`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(call) = ast::CallExpr::cast(node) else {
                continue;
            };
            // We only care about `Err(...)`-shaped calls.
            let Some(callee) = call.expr() else { continue };
            let callee_text = callee.syntax().text().to_string();
            if callee_text != "Err" {
                continue;
            }
            let Some(args) = call.arg_list() else {
                continue;
            };
            let Some(arg) = args.args().next() else {
                continue;
            };
            if !looks_like_string_error(&arg) {
                continue;
            }
            if is_in_test_context(call.syntax()) {
                continue;
            }
            let range = call.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "stringly-typed error returned from `Err(..)`",
                ctx.source,
                range,
                Some("define a typed error with `thiserror::Error` and return that variant".into()),
            ));
        }
    }
}

fn looks_like_string_error(arg: &ast::Expr) -> bool {
    let text = arg.syntax().text().to_string();
    let trimmed = text.trim();
    // "literal".to_string() / .into() / .to_owned()
    if trimmed.starts_with('"')
        && (trimmed.ends_with(".to_string()")
            || trimmed.ends_with(".to_owned()")
            || trimmed.ends_with(".into()"))
    {
        return true;
    }
    // bare string literal (only if context allows `&str` errors)
    if trimmed.starts_with('"') && trimmed.ends_with('"') && !trimmed.contains('\n') {
        return true;
    }
    // format!(..)
    if trimmed.starts_with("format!(") || trimmed.starts_with("format !(") {
        return true;
    }
    false
}
