use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct BoolOptionPair;

impl LintRule for BoolOptionPair {
    fn id(&self) -> &'static str {
        "bool-option-pair"
    }

    fn description(&self) -> &'static str {
        "`is_paid: bool` + `payment_id: Option<_>` is an unrepresentable-state smell"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(strukt) = ast::Struct::cast(node) else {
                continue;
            };
            let Some(ast::FieldList::RecordFieldList(fields)) = strukt.field_list() else {
                continue;
            };

            // Index public fields by stem. A bool flag named `is_paid` /
            // `paid` / `has_payment` / `verified` pairs with an `Option<_>`
            // field whose name overlaps the stem.
            let mut bool_flags: Vec<(String, String, ast::RecordField)> = Vec::new();
            let mut option_fields: Vec<(String, String, ast::RecordField)> = Vec::new();

            for field in fields.fields() {
                if field.visibility().is_none() {
                    continue;
                }
                let Some(name) = field.name() else { continue };
                let name_text = name.text().to_string();
                let Some(ty) = field.ty() else { continue };
                let ty_text = ty.syntax().text().to_string().replace([' ', '\n'], "");
                if ty_text == "bool" {
                    let stem = strip_bool_prefix(&name_text);
                    bool_flags.push((name_text.clone(), stem, field));
                } else if ty_text.starts_with("Option<") {
                    let stem = strip_option_suffix(&name_text);
                    option_fields.push((name_text.clone(), stem, field));
                }
            }

            // Strict: bool/option stems overlap (e.g. `is_active` ↔ `active_id`).
            // Loose:  the struct contains at least one `is_*` / `has_*` /
            //         `was_*` bool AND at least one `_id`/`_at`/`_by`/`_url`
            //         Option field. This catches the canonical
            //         `is_paid` + `payment_id` smell from rust-types-as-walls
            //         where stems differ but state is correlated.
            let has_correlated_option = option_fields.iter().any(|(name, _, _)| {
                name.ends_with("_id")
                    || name.ends_with("_at")
                    || name.ends_with("_by")
                    || name.ends_with("_url")
            });
            for (bname, bstem, bfield) in &bool_flags {
                let prefixed = bname.starts_with("is_")
                    || bname.starts_with("has_")
                    || bname.starts_with("was_");
                let strict_match = option_fields
                    .iter()
                    .find(|(_, ostem, _)| stems_overlap(bstem, ostem));
                let pair_name = strict_match
                    .map(|(n, _, _)| n.clone())
                    .or_else(|| {
                        if prefixed && has_correlated_option {
                            option_fields.first().map(|(n, _, _)| n.clone())
                        } else {
                            None
                        }
                    });
                let Some(oname) = pair_name else { continue };
                let range = bfield.syntax().text_range();
                out.push(Diagnostic::from_range(
                    ctx.file,
                    self.id(),
                    Severity::Warning,
                    format!(
                        "`{bname}: bool` and `{oname}: Option<_>` encode correlated state; \
                         illegal combinations are representable"
                    ),
                    ctx.source,
                    range,
                    Some(
                        "replace the pair with an `enum` whose variants carry the dependent data \
                         (e.g. `Paid { payment_id: PaymentId }` / `Unpaid`); see rust-types-as-walls"
                            .into(),
                    ),
                ));
            }
        }
    }
}

fn strip_bool_prefix(name: &str) -> String {
    name.strip_prefix("is_")
        .or_else(|| name.strip_prefix("has_"))
        .or_else(|| name.strip_prefix("was_"))
        .unwrap_or(name)
        .to_string()
}

fn strip_option_suffix(name: &str) -> String {
    name.strip_suffix("_id")
        .or_else(|| name.strip_suffix("_at"))
        .or_else(|| name.strip_suffix("_by"))
        .or_else(|| name.strip_suffix("_url"))
        .unwrap_or(name)
        .to_string()
}

fn stems_overlap(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a == b || a.contains(b) || b.contains(a)
}
