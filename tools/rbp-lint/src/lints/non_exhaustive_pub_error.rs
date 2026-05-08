use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName, HasVisibility};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::util::attr_path_string;
use crate::lints::{LintContext, LintRule};

pub struct NonExhaustivePubError;

impl LintRule for NonExhaustivePubError {
    fn id(&self) -> &'static str {
        "non-exhaustive-pub-error"
    }

    fn description(&self) -> &'static str {
        "`pub enum *Error` should be `#[non_exhaustive]` to keep SemVer flexibility"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(en) = ast::Enum::cast(node) else {
                continue;
            };
            // Only public enums.
            if en.visibility().is_none() {
                continue;
            }
            let Some(name) = en.name() else { continue };
            let name_text = name.text().to_string();
            // Heuristic: error-shaped names only. Avoid noisy hits on every
            // public enum.
            if !name_text.ends_with("Error") && !name_text.ends_with("Err") {
                continue;
            }
            let already_non_exhaustive = en
                .attrs()
                .any(|attr| attr_path_string(&attr) == "non_exhaustive");
            if already_non_exhaustive {
                continue;
            }
            let range = en.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                format!(
                    "public error enum `{name_text}` is not `#[non_exhaustive]`; adding a \
                     variant later will break downstream `match`"
                ),
                ctx.source,
                range,
                Some(
                    "annotate with `#[non_exhaustive]` so callers must include a `_ =>` arm and \
                     adding a variant later stays SemVer-compatible"
                        .into(),
                ),
            ));
        }
    }
}
