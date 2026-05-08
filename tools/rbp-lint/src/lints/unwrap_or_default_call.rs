use ra_ap_syntax::ast::{self, AstNode, HasArgList};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct UnwrapOrDefaultCall;

impl LintRule for UnwrapOrDefaultCall {
    fn id(&self) -> &'static str {
        "unwrap-or-default-call"
    }

    fn description(&self) -> &'static str {
        "`.unwrap_or(Default::default())` / `.unwrap_or(T::default())` → `.unwrap_or_default()`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(method) = ast::MethodCallExpr::cast(node) else {
                continue;
            };
            let Some(name) = method.name_ref() else {
                continue;
            };
            if name.text() != "unwrap_or" && name.text() != "or" {
                continue;
            }
            let Some(args) = method.arg_list() else {
                continue;
            };
            let Some(arg) = args.args().next() else {
                continue;
            };
            let arg_text = arg.syntax().text().to_string();
            let trimmed = arg_text.trim();
            // Match `Default::default()`, `T::default()`, `<T>::default()`.
            let is_default_call = trimmed.ends_with("::default()");
            if !is_default_call {
                continue;
            }
            let suggestion = if name.text() == "unwrap_or" {
                "use `.unwrap_or_default()`"
            } else {
                "use `.or_default()` (Option) or pull the default out of the chain"
            };
            let range = method.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Note,
                format!(
                    "redundant `Default::default()` argument to `{}`",
                    name.text()
                ),
                ctx.source,
                range,
                Some(suggestion.into()),
            ));
        }
    }
}
