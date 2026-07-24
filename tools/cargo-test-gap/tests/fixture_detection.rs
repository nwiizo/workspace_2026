use std::fs;
use std::path::Path;
use std::process::Command;

use cargo_test_gap::{AnalyzeOptions, Issue, Severity, analyze_path, diff_against_ref};
use tempfile::TempDir;

#[test]
fn test_functions_and_cfg_test_helpers_are_not_reported() {
    let temp = basic_crate(
        r#"
#[test]
fn direct_test() {}

#[cfg(test)]
mod tests {
    fn helper() {}
}

pub fn production() -> usize { 1 }
"#,
    );
    init_git(temp.path());
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    assert!(!has_function(&analysis.issues, "direct_test"));
    assert!(!has_function(&analysis.issues, "helper"));
    assert!(has_function(&analysis.issues, "production"));
}

#[test]
fn test_reachability_lowers_high_churn_public_function_risk() {
    let temp = basic_crate(
        r#"
pub fn hot(value: usize) -> Result<usize, ()> {
    if value > 10 { Ok(value) } else { Ok(value + 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_hot() {
        let _ = hot(1);
    }
}
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 5);
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    let issue = issue_named(&analysis.issues, "hot");
    assert!(issue.coverage >= 100.0);
    assert!(issue.risk < 5.0, "risk should drop, got {}", issue.risk);
    assert!(issue.severity <= Severity::Low);
}

#[test]
fn integration_tests_are_reachability_roots() {
    let temp = basic_crate(
        r#"
pub fn core(value: usize) -> Result<usize, ()> {
    if value > 0 { Ok(value) } else { Err(()) }
}
"#,
    );
    fs::create_dir_all(temp.path().join("tests")).expect("tests dir");
    fs::write(
        temp.path().join("tests/integration.rs"),
        r#"
use fixture::core;

#[test]
fn covers_core_from_integration_test() {
    let _ = core(1);
}
"#,
    )
    .expect("integration test");
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    let issue = issue_named(&analysis.issues, "core");
    assert_eq!(issue.coverage, 100.0);
    assert!(issue.risk < 5.0, "risk should drop, got {}", issue.risk);
}

#[test]
fn untested_complex_public_result_function_ranks_above_getter() {
    let temp = basic_crate(
        r#"
pub fn uncovered(value: usize) -> Result<usize, ()> {
    match value {
        0 => Ok(0),
        1 => Ok(1),
        2 => Ok(2),
        3 => Ok(3),
        4 => Ok(4),
        5 => Ok(5),
        6 => Ok(6),
        7 => Ok(7),
        8 => Ok(8),
        _ => Err(()),
    }
}

pub fn getter() -> usize { 1 }
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    assert_eq!(analysis.issues[0].function, "uncovered");
    assert!(analysis.issues[0].risk > issue_named(&analysis.issues, "getter").risk);
}

#[test]
fn simple_getter_with_zero_churn_is_low_even_with_all() {
    let temp = basic_crate("pub fn getter() -> usize { 1 }\n");
    init_git(temp.path());
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    let issue = issue_named(&analysis.issues, "getter");
    assert_eq!(issue.churn, 0.0);
    assert_eq!(issue.complexity, 1);
    assert_eq!(issue.severity, Severity::Low);
}

#[test]
fn doc_comment_churn_keeps_baseline_key_stable() {
    let temp = basic_crate(
        r#"
/// v1
pub fn documented(value: usize) -> Result<usize, ()> {
    if value > 0 { Ok(value) } else { Err(()) }
}
"#,
    );
    init_git(temp.path());
    replace_lib(
        temp.path(),
        r#"
/// v2
pub fn documented(value: usize) -> Result<usize, ()> {
    if value > 0 { Ok(value) } else { Err(()) }
}
"#,
    );
    commit_all(temp.path(), "doc only");
    let options = AnalyzeOptions::default();
    let analysis = analyze_path(temp.path(), &options).expect("analysis");
    let diff = diff_against_ref(temp.path(), &options, &analysis, "HEAD").expect("diff");
    assert_eq!(diff.new_issues.len(), 0);
    assert_eq!(diff.resolved_issues.len(), 0);
    assert_eq!(diff.unchanged, analysis.issues.len());
}

#[test]
fn complexity_bucket_crossing_keeps_baseline_key_stable() {
    let temp = basic_crate(
        r#"
pub fn documented(value: usize) -> Result<usize, ()> {
    if value > 0 { Ok(value) } else { Err(()) }
}
"#,
    );
    init_git(temp.path());
    replace_lib(
        temp.path(),
        r#"
pub fn documented(value: usize) -> Result<usize, ()> {
    if value == 0 {
        Ok(0)
    } else if value == 1 {
        Ok(1)
    } else if value == 2 {
        Ok(2)
    } else {
        Err(())
    }
}
"#,
    );
    let options = AnalyzeOptions::default();
    let analysis = analyze_path(temp.path(), &options).expect("analysis");
    let diff = diff_against_ref(temp.path(), &options, &analysis, "HEAD").expect("diff");
    assert_eq!(diff.new_issues.len(), 0);
    assert_eq!(diff.resolved_issues.len(), 0);
    assert_eq!(diff.unchanged, analysis.issues.len());
}

#[test]
fn same_named_functions_in_different_impls_have_distinct_keys() {
    let temp = basic_crate(
        r#"
pub struct A;
pub struct B;

impl A {
    pub fn run(&self) -> Result<(), ()> { Ok(()) }
}

impl B {
    pub fn run(&self) -> Result<(), ()> { Ok(()) }
}
"#,
    );
    init_git(temp.path());
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    let keys = analysis
        .issues
        .iter()
        .map(|issue| issue.key.source.as_str())
        .collect::<Vec<_>>();
    assert!(keys.iter().any(|key| key.contains("A::run")));
    assert!(keys.iter().any(|key| key.contains("B::run")));
}

#[test]
fn local_suppression_applies_only_to_adjacent_function() {
    let temp = basic_crate(
        r#"
// test-gap-allow: test-gap
pub fn allowed() -> Result<(), ()> {
    if true { Ok(()) } else { Err(()) }
}

pub fn neighbor() -> Result<(), ()> {
    if true { Ok(()) } else { Err(()) }
}
"#,
    );
    init_git(temp.path());
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    assert_eq!(analysis.suppressed_issues, 1);
    assert!(!has_function(&analysis.issues, "allowed"));
    assert!(has_function(&analysis.issues, "neighbor"));
}

#[test]
fn llvm_cov_json_lowers_covered_function_risk() {
    let temp = basic_crate(
        r#"
pub fn covered(value: usize) -> Result<usize, ()> {
    if value > 10 { Ok(value) } else { Ok(value + 1) }
}
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let without = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    let cov = temp.path().join("coverage.json");
    fs::write(
        &cov,
        r#"
{
  "data": [
    {
      "functions": [
        {
          "name": "covered",
          "filenames": ["src/lib.rs"],
          "regions": [[1, 1, 3, 2, 9, 1, 0, 0, 1]]
        }
      ]
    }
  ]
}
"#,
    )
    .expect("coverage json");
    let with = analyze_path(
        temp.path(),
        &AnalyzeOptions {
            llvm_cov: Some(cov),
        },
    )
    .expect("analysis");
    let before = issue_named(&without.issues, "covered");
    let after = issue_named(&with.issues, "covered");
    assert!(after.coverage >= 100.0);
    assert!(after.risk < before.risk);
}

#[test]
fn llvm_cov_keeps_same_named_methods_separate() {
    let temp = basic_crate(
        r#"
pub struct AlphaRunner;
pub struct BetaRunner;

impl AlphaRunner {
    pub fn run(&self) -> Result<(), ()> { Ok(()) }
}

impl BetaRunner {
    pub fn run(&self) -> Result<(), ()> { Ok(()) }
}
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let cov = temp.path().join("coverage.json");
    fs::write(
        &cov,
        format!(
            r#"
{{
  "data": [
    {{
      "functions": [
        {{
          "name": "AlphaRunner::run",
          "filenames": ["{}"],
          "count": 1
        }}
      ]
    }}
  ]
}}
"#,
            temp.path().join("src/lib.rs").display()
        ),
    )
    .expect("coverage json");
    let analysis = analyze_path(
        temp.path(),
        &AnalyzeOptions {
            llvm_cov: Some(cov),
        },
    )
    .expect("analysis");
    assert_eq!(
        issue_named(&analysis.issues, "AlphaRunner::run").coverage,
        100.0
    );
    assert_eq!(
        issue_named(&analysis.issues, "BetaRunner::run").coverage,
        0.0
    );
}

#[test]
fn unmatched_llvm_cov_json_warns_and_records_blind_spot() {
    let temp = basic_crate("pub fn getter() -> usize { 1 }\n");
    init_git(temp.path());
    let cov = temp.path().join("coverage.json");
    fs::write(&cov, r#"{ "data": [{ "files": [] }] }"#).expect("coverage json");
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--llvm-cov")
        .arg(&cov)
        .arg("--json")
        .output()
        .expect("command");
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("warning: llvm-cov JSON did not match any production function"),
        "{stderr}"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("total_candidates"));
    assert!(stdout.contains("did not match any production function"));
}

#[test]
fn json_ignores_top_and_reports_total_candidates() {
    let temp = basic_crate(
        r#"
pub fn one() -> usize { 1 }
pub fn two() -> usize { 2 }
"#,
    );
    init_git(temp.path());
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--json")
        .arg("--top")
        .arg("0")
        .output()
        .expect("command");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(json["total_candidates"].as_u64(), Some(2));
    assert_eq!(json["issues"].as_array().map(Vec::len), Some(2));
}

#[test]
fn top_zero_reports_hidden_display_count_not_no_candidates() {
    let temp = basic_crate(
        r#"
pub fn risky(value: usize) -> Result<usize, ()> {
    match value {
        0 => Ok(0),
        1 => Ok(1),
        2 => Ok(2),
        3 => Ok(3),
        _ => Err(()),
    }
}
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--top")
        .arg("0")
        .output()
        .expect("command");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("0 of 1 candidates shown (--top 0)."),
        "{stdout}"
    );
    assert!(!stdout.contains("No candidates in the selected severity range."));
}

#[test]
fn ai_output_ignores_top_limit() {
    let temp = basic_crate(
        r#"
pub fn risky(value: usize) -> Result<usize, ()> {
    match value {
        0 => Ok(0),
        1 => Ok(1),
        2 => Ok(2),
        3 => Ok(3),
        _ => Err(()),
    }
}
"#,
    );
    init_git(temp.path());
    churn_file(temp.path(), 4);
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--ai")
        .arg("--top")
        .arg("0")
        .output()
        .expect("command");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("## risky at src/lib.rs:"), "{stdout}");
}

#[test]
fn human_output_orders_breakdown_suppression_gate_and_hidden_hint() {
    let temp = basic_crate(
        r#"
// test-gap-allow: test-gap
pub fn allowed() -> usize { 1 }

pub fn neighbor() -> usize { 2 }
"#,
    );
    init_git(temp.path());
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--check")
        .arg("--fail-on")
        .arg("low")
        .output()
        .expect("command");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains("Breakdown: Critical=0, High=0, Medium=0, Low=1"),
        "{stdout}"
    );
    let suppressed = stdout.find("1 issues suppressed").expect("suppressed line");
    let gate = stdout.find("check: FAIL").expect("gate line");
    let hint = stdout.find("hint: 1 low-severity").expect("hint line");
    assert!(suppressed < gate, "{stdout}");
    assert!(gate < hint, "{stdout}");
}

#[test]
fn block_comment_code_is_not_counted_as_complexity_or_exposure() {
    let temp = basic_crate(
        r#"
pub fn getter() -> usize {
    /*
    pub fn fake() -> Result<(), ()> {
        match 1 { 0 => Ok(()), 1 => Ok(()), _ => Err(()) }
    }
    */
    1
}
"#,
    );
    init_git(temp.path());
    let analysis = analyze_path(temp.path(), &AnalyzeOptions::default()).expect("analysis");
    assert_eq!(analysis.issues.len(), 1);
    let issue = issue_named(&analysis.issues, "getter");
    assert_eq!(issue.complexity, 1);
    assert_eq!(issue.exposure, 4.0);
}

#[test]
fn missing_llvm_cov_path_exits_with_error_prefix_and_code_one() {
    let temp = basic_crate("pub fn getter() -> usize { 1 }\n");
    init_git(temp.path());
    let binary = env!("CARGO_BIN_EXE_cargo-test-gap");
    let output = Command::new(binary)
        .arg(temp.path())
        .arg("--llvm-cov")
        .arg(temp.path().join("missing.json"))
        .output()
        .expect("command");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.starts_with("error: "), "{stderr}");
}

fn basic_crate(lib_rs: &str) -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    write_manifest(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("src");
    replace_lib(temp.path(), lib_rs);
    temp
}

fn write_manifest(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("manifest");
}

fn replace_lib(root: &Path, source: &str) {
    fs::write(root.join("src/lib.rs"), source).expect("lib");
}

fn init_git(root: &Path) {
    run_git(root, &["init"]);
    run_git(root, &["config", "user.email", "test@example.com"]);
    run_git(root, &["config", "user.name", "Test User"]);
    commit_all(root, "initial");
}

fn churn_file(root: &Path, count: usize) {
    for idx in 0..count {
        let mut source = fs::read_to_string(root.join("src/lib.rs")).expect("read lib");
        source.push_str(&format!("\n// churn {idx}\n"));
        replace_lib(root, &source);
        commit_all(root, &format!("churn {idx}"));
    }
}

fn commit_all(root: &Path, message: &str) {
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-m", message]);
}

fn run_git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn issue_named<'a>(issues: &'a [Issue], name: &str) -> &'a Issue {
    issues
        .iter()
        .find(|issue| issue.function == name || issue.function.ends_with(&format!("::{name}")))
        .expect("issue")
}

fn has_function(issues: &[Issue], name: &str) -> bool {
    issues
        .iter()
        .any(|issue| issue.function == name || issue.function.ends_with(&format!("::{name}")))
}
