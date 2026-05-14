use std::collections::BTreeSet;
use std::path::PathBuf;

use rbp_lint::{all_lints, lint_source};

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p
}

fn run(name: &str) -> Vec<rbp_lint::Diagnostic> {
    let path = fixture_path(name);
    let source = std::fs::read_to_string(&path).expect("fixture exists");
    let lints = all_lints();
    lint_source(&path, &source, &lints)
}

#[test]
fn good_fixture_is_clean() {
    let diags = run("good.rs");
    assert!(
        diags.is_empty(),
        "expected no diagnostics, got {:#?}",
        diags
    );
}

#[test]
fn suppression_drops_only_marked_rules() {
    let diags = run("suppressed.rs");
    // Should still report exactly one no-unwrap (the unsuppressed one at the bottom)
    let unwraps: Vec<&rbp_lint::Diagnostic> =
        diags.iter().filter(|d| d.rule == "no-unwrap").collect();
    assert_eq!(
        unwraps.len(),
        1,
        "expected one unsuppressed no-unwrap, got: {:#?}",
        diags
    );
    assert!(
        unwraps[0].line >= 16,
        "expected the unsuppressed unwrap to be near line 17, got line {}",
        unwraps[0].line
    );
    // no-panic must be silenced by file-top allow
    assert!(
        !diags.iter().any(|d| d.rule == "no-panic"),
        "no-panic should be silenced by file-top allow, got: {:#?}",
        diags
    );
}

#[test]
fn bad_fixture_triggers_each_rule() {
    let diags = run("bad.rs");
    let rules: BTreeSet<&str> = diags.iter().map(|d| d.rule).collect();
    let expected = [
        "no-unwrap",
        "no-expect",
        "no-panic",
        "dead-code-comment",
        "tracing-format",
        "arc-clone-explicit",
        "hardcoded-secret",
        "unsafe-safety-comment",
        "debug-print",
        "string-as-error",
        "unbounded-channel",
        "unwrap-or-default-call",
        "needless-return",
        "lazy-static-macro",
        "manual-let-else",
        "pub-field-newtype",
        "non-exhaustive-pub-error",
        "raw-id-field",
        "status-string-field",
        "bool-option-pair",
    ];
    for rule in expected {
        assert!(
            rules.contains(rule),
            "expected rule {rule} to fire on bad.rs; got {:?}",
            rules
        );
    }
}
