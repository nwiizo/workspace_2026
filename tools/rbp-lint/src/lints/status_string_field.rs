use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct StatusStringField;

/// Field names that are "stringly status-like" if typed as `String`.
const STATUS_NAMES: &[&str] = &["status", "state", "kind", "phase", "stage", "type_"];

impl LintRule for StatusStringField {
    fn id(&self) -> &'static str {
        "status-string-field"
    }

    fn description(&self) -> &'static str {
        "Public `status: String` (or similar) should be a typed `enum`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(strukt) = ast::Struct::cast(node) else {
                continue;
            };
            let Some(ast::FieldList::RecordFieldList(fields)) = strukt.field_list() else {
                continue;
            };
            for field in fields.fields() {
                if field.visibility().is_none() {
                    continue;
                }
                let Some(name) = field.name() else { continue };
                let name_text = name.text().to_string();
                if !STATUS_NAMES.contains(&name_text.as_str()) {
                    continue;
                }
                let Some(ty) = field.ty() else { continue };
                let ty_text = ty.syntax().text().to_string().replace([' ', '\n'], "");
                if ty_text != "String" && ty_text != "&str" {
                    continue;
                }
                let range = field.syntax().text_range();
                out.push(Diagnostic::from_range(
                    ctx.file,
                    self.id(),
                    Severity::Note,
                    format!("`{name_text}: {ty_text}` is stringly-typed; define an `enum`"),
                    ctx.source,
                    range,
                    Some(
                        "model the closed set of values as `enum Status { Pending, Verified, .. }`; \
                         see rust-types-as-walls (Smart Constructor / Make Illegal States Unrepresentable)"
                            .into(),
                    ),
                ));
            }
        }
    }
}
