use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct RawIdField;

const STRINGY: &[&str] = &["String", "&str", "&'staticstr"];
const INTEGRAL: &[&str] = &["u8", "u16", "u32", "u64", "u128", "i32", "i64", "usize"];

impl LintRule for RawIdField {
    fn id(&self) -> &'static str {
        "raw-id-field"
    }

    fn description(&self) -> &'static str {
        "Public `*_id` / `id` fields of raw `String` or integer types should be newtypes"
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
                if !is_id_name(&name_text) {
                    continue;
                }
                let Some(ty) = field.ty() else { continue };
                let ty_text = ty.syntax().text().to_string().replace([' ', '\n'], "");
                let is_raw =
                    STRINGY.contains(&ty_text.as_str()) || INTEGRAL.contains(&ty_text.as_str());
                if !is_raw {
                    continue;
                }
                let range = field.syntax().text_range();
                out.push(Diagnostic::from_range(
                    ctx.file,
                    self.id(),
                    Severity::Note,
                    format!("`{name_text}: {ty_text}` is a raw identifier; wrap it in a newtype"),
                    ctx.source,
                    range,
                    Some(
                        "define `pub struct UserId(String)` (or similar) and expose accessors so \
                         `UserId` and `OrderId` cannot be confused"
                            .into(),
                    ),
                ));
            }
        }
    }
}

fn is_id_name(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}
