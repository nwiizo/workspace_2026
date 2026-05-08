use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct TracingFormat;

const TRACING_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error"];

impl LintRule for TracingFormat {
    fn id(&self) -> &'static str {
        "tracing-format"
    }

    fn description(&self) -> &'static str {
        "Prefer structured fields over format strings in tracing macros"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(mac) = ast::MacroCall::cast(node) else {
                continue;
            };
            let Some(path) = mac.path() else { continue };
            let path_text = path.syntax().text().to_string();
            let normalized = path_text.replace(' ', "");
            // Match `tracing::info`, `info`, `log::info`, etc.
            let last = normalized.rsplit("::").next().unwrap_or("");
            if !TRACING_MACROS.contains(&last) {
                continue;
            }
            // Heuristic: only flag tracing-like calls, not arbitrary user macros.
            let is_tracing_like = normalized.starts_with("tracing::")
                || normalized.starts_with("log::")
                || normalized == last;
            if !is_tracing_like {
                continue;
            }
            let Some(tt) = mac.token_tree() else { continue };
            let body = tt.syntax().text().to_string();
            // Strip outer delimiters.
            let inner = body
                .trim()
                .trim_start_matches(['(', '[', '{'])
                .trim_end_matches([')', ']', '}'])
                .trim();
            if !looks_like_format_only(inner) {
                continue;
            }
            let range = mac.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                "tracing-format",
                Severity::Note,
                format!("`{last}!` uses positional formatting; prefer structured fields"),
                ctx.source,
                range,
                Some(
                    "use field syntax: `tracing::info!(user_id = %id, \"message\")` so log \
                     aggregators can index fields"
                        .into(),
                ),
            ));
        }
    }
}

/// Return true if the macro arguments look like a positional format string
/// (i.e. starts with a string literal containing `{}` and has no `key = value`
/// fields preceding it).
fn looks_like_format_only(inner: &str) -> bool {
    // Find the first quoted string.
    let bytes = inner.as_bytes();
    let mut i = 0;
    let mut saw_eq_before_string = false;
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 && bytes.get(i + 1) != Some(&b'=') => {
                saw_eq_before_string = true;
            }
            b'"' => {
                // Scan to closing quote.
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                let lit = &inner[start..j.min(bytes.len())];
                if saw_eq_before_string {
                    return false;
                }
                return lit.contains("{}") || lit.contains("{:?}") || lit.contains("{:#?}");
            }
            _ => {}
        }
        i += 1;
    }
    false
}
