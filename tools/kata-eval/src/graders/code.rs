//! `code` grader — evaluates Python/JS-flavored boolean expressions against
//! `output`, via an embedded rhai engine. waxa-compat shims (`len(x)`,
//! `'a' in x`, `'a' not in x`) are translated up-front so existing waxa
//! YAML works unchanged.

use anyhow::Result;
use regex::Regex;
use rhai::{Dynamic, Engine};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::sync::{LazyLock, OnceLock};

/// Operation cap for the embedded rhai engine. A grader assertion that runs
/// past this — e.g. an accidentally infinite loop or a quadratic blowup over
/// the executor output — fails fast instead of hanging the trial.
const RHAI_MAX_OPERATIONS: u64 = 50_000;
/// String-allocation cap. The script literal embeds the executor output, so
/// the budget has to cover that plus headroom for split/contains intermediates.
const RHAI_MAX_STRING_SIZE: usize = 4 * 1024 * 1024;

static ENGINE: LazyLock<Engine> = LazyLock::new(build_engine);

pub(crate) fn evaluate(
    config: &BTreeMap<String, Value>,
    output: &str,
) -> Result<(bool, f64, Option<String>)> {
    let assertions = match config.get("assertions") {
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut failures: Vec<String> = Vec::new();
    for expr in &assertions {
        let translated = translate(expr);
        // Wrap as `let output = <literal>; <expr>` so all method calls dispatch
        // on a String value rather than a Scope-bound constant (rhai treats
        // scope constants as immutable, breaking `.split` etc.). Because the
        // script contains a `let` statement, we must use `eval` rather than
        // `eval_expression` (which only accepts a single expression).
        let script = format!(
            "let output = {literal}; {translated}",
            literal = rhai_string_literal(output)
        );
        match ENGINE.eval::<Dynamic>(&script) {
            Ok(d) => {
                let truthy = d.as_bool().unwrap_or(false);
                if !truthy {
                    failures.push(format!("Failed: {expr}"));
                }
            }
            Err(e) => failures.push(format!("Error in `{expr}`: {e}")),
        }
    }
    let pass = failures.is_empty();
    let score = if pass { 1.0 } else { 0.0 };
    let message = if pass {
        None
    } else {
        Some(failures.join("; "))
    };
    Ok((pass, score, message))
}

fn rhai_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn build_engine() -> Engine {
    let mut e = Engine::new();
    e.set_max_expr_depths(64, 64);
    e.set_max_operations(RHAI_MAX_OPERATIONS);
    e.set_max_string_size(RHAI_MAX_STRING_SIZE);
    // Python/JS shim: `len(x)` works for strings and arrays.
    e.register_fn("len", |s: &str| s.chars().count() as i64);
    e.register_fn("len", |s: rhai::ImmutableString| s.chars().count() as i64);
    e.register_fn("len", |a: rhai::Array| a.len() as i64);
    // Override rhai's in-place `trim` with a non-mutating variant that
    // returns a new ImmutableString, so chains like `s.trim().split(...)`
    // work the JS way.
    e.register_fn(
        "trim",
        |s: rhai::ImmutableString| -> rhai::ImmutableString { s.trim().into() },
    );
    e.register_fn("trim", |s: &str| -> rhai::ImmutableString {
        s.trim().into()
    });
    // Likewise: convenience wrappers used by JS-style assertions.
    e.register_fn("lower", |s: &str| -> rhai::ImmutableString {
        s.to_lowercase().into()
    });
    e.register_fn("upper", |s: &str| -> rhai::ImmutableString {
        s.to_uppercase().into()
    });
    e
}

/// Best-effort shim translator. Intentionally narrow — for richer logic, use
/// an `llm` grader or split into multiple assertions.
pub(crate) fn translate(expr: &str) -> String {
    static NOT_IN: OnceLock<Regex> = OnceLock::new();
    static IN_: OnceLock<Regex> = OnceLock::new();
    let not_in_re = NOT_IN.get_or_init(|| {
        Regex::new(r#"(['"][^'"]+['"])\s+not\s+in\s+([a-zA-Z_]\w*)"#).expect("regex")
    });
    let in_re =
        IN_.get_or_init(|| Regex::new(r#"(['"][^'"]+['"])\s+in\s+([a-zA-Z_]\w*)"#).expect("regex"));

    let mut out = expr.to_string();
    out = not_in_re.replace_all(&out, "!$2.contains($1)").to_string();
    out = in_re.replace_all(&out, "$2.contains($1)").to_string();
    out = single_to_double_quotes(&out);
    out = out.replace(".includes(", ".contains(");
    // JS `.length` → rhai `.len()` (registered as a free function on strings
    // and arrays). Property-style `.len` is not valid rhai syntax.
    out = out.replace(".length", ".len()");
    out
}

/// Convert single-quoted string literals to double-quoted so rhai (which
/// treats single quotes as char literals) accepts them. Skips characters
/// inside already-double-quoted strings.
fn single_to_double_quotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_double = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_double => {
                out.push(c);
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => {
                in_double = !in_double;
                out.push('"');
            }
            '\'' if !in_double => out.push('"'),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_one(expr: &str, output: &str) -> bool {
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "assertions".to_string(),
            Value::Sequence(vec![Value::String(expr.to_string())]),
        );
        let (pass, _, _msg) = evaluate(&cfg, output).expect("ok");
        pass
    }

    #[test]
    fn len_translation() {
        assert!(eval_one("len(output) > 5", "hello world"));
        assert!(!eval_one("len(output) > 100", "hello"));
    }

    #[test]
    fn in_translation() {
        assert!(eval_one("'lo' in output", "hello"));
        assert!(eval_one("'zz' not in output", "hello"));
    }

    #[test]
    fn split_index_translation() {
        let assertion = "len(output.split('## Self-report')[0].trim().split('\\n')) == 1";
        assert!(eval_one(assertion, "single line\n## Self-report\nfoo"));
    }

    #[test]
    fn length_translation() {
        // JS `.length` → rhai `.len()`.
        assert!(eval_one("output.length > 4", "hello"));
        assert!(!eval_one("output.length > 100", "hello"));
    }

    #[test]
    fn rhai_op_cap_kills_runaway_assertions() {
        // 200_000 iterations exceeds RHAI_MAX_OPERATIONS (50_000); rhai must
        // bail out with an error rather than hang.
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "assertions".to_string(),
            Value::Sequence(vec![Value::String(
                "let n = 0; for i in 0..200000 { n += 1; } n > 0".into(),
            )]),
        );
        let (pass, _, msg) = evaluate(&cfg, "").expect("ok");
        assert!(!pass);
        let msg = msg.expect("error message");
        assert!(
            msg.to_lowercase().contains("operation") || msg.to_lowercase().contains("too many"),
            "expected operation-limit error, got {msg:?}"
        );
    }
}
