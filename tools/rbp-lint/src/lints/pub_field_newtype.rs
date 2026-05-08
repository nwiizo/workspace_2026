use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct PubFieldNewtype;

impl LintRule for PubFieldNewtype {
    fn id(&self) -> &'static str {
        "pub-field-newtype"
    }

    fn description(&self) -> &'static str {
        "Tuple newtype `pub struct X(pub T);` exposes the inner type — encapsulate it"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(strukt) = ast::Struct::cast(node) else {
                continue;
            };
            // Only `pub struct` (or `pub(crate)` etc.) — private structs are fine.
            if strukt.visibility().is_none() {
                continue;
            }
            let Some(ast::FieldList::TupleFieldList(fields)) = strukt.field_list() else {
                continue;
            };
            let mut iter = fields.fields();
            let Some(field) = iter.next() else { continue };
            // Newtype = exactly one field.
            if iter.next().is_some() {
                continue;
            }
            // Field must itself be `pub`.
            if field.visibility().is_none() {
                continue;
            }
            let name = strukt
                .name()
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            let range = strukt.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Note,
                format!("newtype `{name}` exposes its inner field; consider keeping it private"),
                ctx.source,
                range,
                Some(
                    "drop the inner `pub` and expose accessors instead (`fn new(...) -> Self`, \
                     `as_str(&self)`, etc.) so callers cannot bypass invariants"
                        .into(),
                ),
            ));
        }
    }
}
