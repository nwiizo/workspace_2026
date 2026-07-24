use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use cargo_boundary::BoundaryReport;
use cargo_boundary::IssueType;
use tempfile::TempDir;

#[test]
fn fixture_detects_all_issue_types_and_suppression() {
    let fixture = fixture_crate();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let output = cmd
        .arg("boundary")
        .arg("--all")
        .arg("--json")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: BoundaryReport = serde_json::from_slice(&output).expect("valid report json");
    assert!(
        report
            .issues
            .iter()
            .flat_map(|issue| &issue.locations)
            .all(|location| location.file.is_relative()),
        "JSON issue locations should be relative: {:#?}",
        report.issues
    );
    let issue_types: Vec<IssueType> = report
        .issues
        .iter()
        .map(|issue| issue.key.issue_type)
        .collect();

    for expected in [
        IssueType::LayerViolation,
        IssueType::InternalCrossing,
        IssueType::PubLeak,
        IssueType::ForbiddenImport,
    ] {
        assert!(
            issue_types.contains(&expected),
            "expected {expected:?}, got {:#?}",
            report.issues
        );
    }

    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.key.target.contains("allowed")),
        "suppressed layer violation should not be reported: {:#?}",
        report.issues
    );
    assert!(
        !report.issues.iter().any(|issue| issue
            .locations
            .iter()
            .any(|location| location.snippet == "sqlx::Row")),
        "suppressed forbidden import should not be reported: {:#?}",
        report.issues
    );
    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.key.target == "SuppressedPub"),
        "suppressed pub leak should not be reported: {:#?}",
        report.issues
    );
    assert!(
        report.issues.iter().any(|issue| issue.key.target == "sqlx"),
        "fully qualified sqlx::query reference should be reported: {:#?}",
        report.issues
    );
    assert!(
        report
            .issues
            .iter()
            .filter(|issue| issue.key.target == "crate::infrastructure::db::Db")
            .all(|issue| issue.occurrences == 1),
        "use imports must not be double-counted as path refs: {:#?}",
        report.issues
    );
    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.key.issue_type == IssueType::PubLeak
                && matches!(issue.key.target.as_str(), "main_entry" | "run")),
        "method and bare call references should suppress pub-leak: {:#?}",
        report.issues
    );
}

fn fixture_crate() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "boundary-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        dir.path().join("boundary.toml"),
        r#"
[[layers]]
name = "domain"
rank = 0
paths = ["domain"]

[[layers]]
name = "application"
rank = 1
paths = ["application"]

[[layers]]
name = "infrastructure"
rank = 2
paths = ["infrastructure"]

[[forbidden_imports]]
layer = "domain"
crates = ["sqlx", "reqwest"]
"#,
    );
    write(
        dir.path().join("src/lib.rs"),
        r#"
pub mod application;
pub mod domain;
pub mod infrastructure;
"#,
    );
    write(
        dir.path().join("src/domain/mod.rs"),
        r#"
pub mod internal;

use crate::infrastructure::db::Db;
// boundary-allow: layer-violation
use crate::infrastructure::allowed::Allowed;
// boundary-allow: forbidden-import
use sqlx::Row;
use reqwest::Client;

pub struct UnusedDomainPub;
// boundary-allow: pub-leak
pub struct SuppressedPub;
pub struct Worker;

impl Worker {
    pub fn run(&self) {}
}

pub fn main_entry() {}

pub fn build(_: Db, _: Allowed, _: Client) {
    sqlx::query("select 1");
    main_entry();
    Worker.run();
}
"#,
    );
    write(
        dir.path().join("src/domain/internal.rs"),
        r#"
pub struct Secret;
"#,
    );
    write(
        dir.path().join("src/application/mod.rs"),
        r#"
pub mod usecase;
"#,
    );
    write(
        dir.path().join("src/application/usecase.rs"),
        r#"
use crate::domain::internal::Secret;

pub fn call(_: Secret) {}
"#,
    );
    write(
        dir.path().join("src/infrastructure/mod.rs"),
        r#"
pub mod allowed;
pub mod db;
"#,
    );
    write(
        dir.path().join("src/infrastructure/db.rs"),
        r#"
pub struct Db;
"#,
    );
    write(
        dir.path().join("src/infrastructure/allowed.rs"),
        r#"
pub struct Allowed;
"#,
    );
    dir
}

#[test]
fn comments_and_generics_are_parsed_as_cst() {
    let fixture = cst_fixture();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let output = cmd
        .arg("--all")
        .arg("--json")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: BoundaryReport = serde_json::from_slice(&output).expect("valid report json");
    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.key.target.contains("Commented")),
        "block comments must not create issues: {:#?}",
        report.issues
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.key.target == "crate::infrastructure::DbHandle"),
        "generic type argument path should be reported: {:#?}",
        report.issues
    );
}

fn cst_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "cst-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        dir.path().join("boundary.toml"),
        r#"
[[layers]]
name = "domain"
rank = 0
paths = ["domain"]

[[layers]]
name = "infrastructure"
rank = 2
paths = ["infrastructure"]
"#,
    );
    write(
        dir.path().join("src/lib.rs"),
        r#"
pub mod domain;
pub mod infrastructure;
"#,
    );
    write(
        dir.path().join("src/domain/mod.rs"),
        r#"
/*
use crate::infrastructure::Commented;
pub struct CommentedPub;
*/
pub fn generic(_: Vec<crate::infrastructure::DbHandle>) {}
"#,
    );
    write(
        dir.path().join("src/infrastructure/mod.rs"),
        r#"
pub struct DbHandle;
"#,
    );
    dir
}

#[test]
fn allow_rules_are_additive_with_rank_rules() {
    let fixture = additive_allow_fixture();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let output = cmd
        .arg("--all")
        .arg("--json")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: BoundaryReport = serde_json::from_slice(&output).expect("valid report json");
    assert!(
        !report.issues.iter().any(|issue| {
            issue.key.issue_type == IssueType::LayerViolation
                && issue.key.source == "infrastructure"
                && issue.key.target == "crate::domain::Thing"
        }),
        "rank-allowed dependency should remain allowed even when [[allow]] exists: {:#?}",
        report.issues
    );
}

fn additive_allow_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "allow-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        dir.path().join("boundary.toml"),
        r#"
[[layers]]
name = "domain"
rank = 0
paths = ["domain"]

[[layers]]
name = "application"
rank = 1
paths = ["application"]

[[layers]]
name = "infrastructure"
rank = 2
paths = ["infrastructure"]

[[allow]]
from = "domain"
to = "infrastructure"
"#,
    );
    write(
        dir.path().join("src/lib.rs"),
        r#"
pub mod domain;
pub mod infrastructure;
"#,
    );
    write(dir.path().join("src/domain/mod.rs"), "pub struct Thing;\n");
    write(
        dir.path().join("src/infrastructure/mod.rs"),
        "use crate::domain::Thing;\npub fn save(_: Thing) {}\n",
    );
    dir
}

#[test]
fn heuristic_layers_prefer_directory_over_file_stem() {
    let fixture = heuristic_fixture();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let stdout = cmd
        .arg("--layers")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(stdout).expect("utf8");
    assert!(
        text.contains("infrastructure/repository/models.rs -> infrastructure"),
        "expected infrastructure layer evidence, got:\n{text}"
    );
}

fn heuristic_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "heuristic-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(dir.path().join("src/lib.rs"), "pub mod infrastructure;\n");
    write(
        dir.path().join("src/infrastructure/mod.rs"),
        "pub mod repository;\n",
    );
    write(
        dir.path().join("src/infrastructure/repository/mod.rs"),
        "pub mod models;\n",
    );
    write(
        dir.path().join("src/infrastructure/repository/models.rs"),
        "pub struct Row;\n",
    );
    dir
}

#[test]
fn baseline_diff_and_ratchet_check_work_in_git_repo() {
    let fixture = baseline_crate();
    git(&fixture, ["init"]);
    git(&fixture, ["config", "user.email", "test@example.invalid"]);
    git(&fixture, ["config", "user.name", "Boundary Test"]);
    git(&fixture, ["add", "."]);
    git(&fixture, ["commit", "-m", "baseline"]);

    write(
        fixture.path().join("src/domain/mod.rs"),
        r#"
use crate::infrastructure::db::Db;

pub fn leak(_: Db) {}
"#,
    );

    let mut json_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let output = json_cmd
        .arg("--all")
        .arg("--json")
        .arg("--baseline")
        .arg("HEAD")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: BoundaryReport = serde_json::from_slice(&output).expect("valid baseline json");
    let diff = report.baseline.expect("baseline diff exists");
    assert!(
        diff.new_issues
            .iter()
            .any(|issue| issue.key.issue_type == IssueType::LayerViolation),
        "expected a new layer violation, got {diff:#?}"
    );

    let mut check_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    check_cmd
        .arg("--check")
        .arg("--baseline")
        .arg("HEAD")
        .arg("--fail-on=medium")
        .arg(fixture.path())
        .assert()
        .failure()
        .code(1);
}

#[test]
fn baseline_same_commit_has_no_false_low_resolved() {
    let fixture = baseline_crate();
    git(&fixture, ["init"]);
    git(&fixture, ["config", "user.email", "test@example.invalid"]);
    git(&fixture, ["config", "user.name", "Boundary Test"]);
    git(&fixture, ["add", "."]);
    git(&fixture, ["commit", "-m", "baseline"]);

    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let output = cmd
        .arg("--json")
        .arg("--baseline")
        .arg("HEAD")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: BoundaryReport = serde_json::from_slice(&output).expect("valid baseline json");
    let diff = report.baseline.expect("baseline diff exists");
    assert_eq!(diff.new_issues.len(), 0, "unexpected new: {diff:#?}");
    assert_eq!(
        diff.resolved_issues.len(),
        0,
        "unexpected resolved: {diff:#?}"
    );
}

#[test]
fn check_fail_on_low_uses_all_issues_even_when_low_hidden() {
    let fixture = low_only_fixture();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let stdout = cmd
        .arg("--check")
        .arg("--fail-on=low")
        .arg(fixture.path())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(stdout).expect("utf8");
    assert!(
        text.contains("hint: 1 low-severity issues hidden, use --all"),
        "missing hidden low hint:\n{text}"
    );
    assert!(
        text.contains("check: FAIL (fail-on=low, 1 issue(s) at/above threshold)"),
        "missing check failure line:\n{text}"
    );
    let breakdown = text.find("Breakdown:").expect("breakdown line");
    let check = text.find("check: FAIL").expect("check line");
    let hint = text.find("hint:").expect("hint line");
    assert!(
        breakdown < check && check < hint,
        "check line should appear immediately after summary before hints:\n{text}"
    );
}

#[test]
fn json_check_includes_gate_object() {
    let fixture = low_only_fixture();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let stdout = cmd
        .arg("--json")
        .arg("--check")
        .arg("--fail-on=low")
        .arg(fixture.path())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&stdout).expect("valid json");
    assert_eq!(json["gate"]["passed"], false);
    assert_eq!(json["gate"]["fail_on"], "low");
    assert_eq!(json["gate"]["failing"], 1);
}

fn low_only_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "low-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(dir.path().join("src/lib.rs"), "pub struct OnlyLow;\n");
    dir
}

#[test]
fn gitignore_is_respected_and_empty_rust_tree_is_explicit() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "ignored-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(dir.path().join(".gitignore"), "vendor/\n");
    write(
        dir.path().join("vendor/cache/src/lib.rs"),
        "pub struct Ignored;\n",
    );

    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let stderr = cmd
        .arg(dir.path())
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(stderr).expect("utf8");
    assert!(
        text.contains("error: no Rust source files found under"),
        "{text}"
    );
}

#[test]
fn output_modes_and_japanese_are_consistent() {
    let fixture = fixture_crate();
    let mut ai_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let ai = ai_cmd
        .arg("--ai")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ai = String::from_utf8(ai).expect("utf8");
    assert_eq!(ai.matches("Blind spot manifest").count(), 1, "{ai}");

    let mut blind_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let blind = blind_cmd
        .arg("--blind-spots")
        .arg("--jp")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let blind = String::from_utf8(blind).expect("utf8");
    assert!(blind.contains("解析器は構文上の path"), "{blind}");

    let mut layers_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let layers = layers_cmd
        .arg("--layers")
        .arg("--jp")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let layers = String::from_utf8(layers).expect("utf8");
    assert!(layers.contains("層構造:"), "{layers}");

    let mut jp_cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let jp = jp_cmd
        .arg("--all")
        .arg("--jp")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let jp = String::from_utf8(jp).expect("utf8");
    assert!(jp.contains("修正:"), "{jp}");
    assert!(!jp.contains("fix:"), "{jp}");
}

#[test]
fn mode_conflicts_warn_about_ignored_flags() {
    let fixture = fixture_crate();
    let mut cmd = Command::cargo_bin("cargo-boundary").expect("binary exists");
    let stderr = cmd
        .arg("--json")
        .arg("--summary")
        .arg(fixture.path())
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(stderr).expect("utf8");
    assert!(stderr.contains("warning: ignoring --summary; using --json"));
}

fn baseline_crate() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "baseline-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        dir.path().join("boundary.toml"),
        r#"
[[layers]]
name = "domain"
rank = 0
paths = ["domain"]

[[layers]]
name = "infrastructure"
rank = 2
paths = ["infrastructure"]
"#,
    );
    write(
        dir.path().join("src/lib.rs"),
        r#"
pub mod domain;
pub mod infrastructure;
"#,
    );
    write(
        dir.path().join("src/domain/mod.rs"),
        r#"
pub fn ok() {}
"#,
    );
    write(
        dir.path().join("src/infrastructure/mod.rs"),
        r#"
pub mod db;
"#,
    );
    write(
        dir.path().join("src/infrastructure/db.rs"),
        r#"
pub struct Db;
"#,
    );
    dir
}

fn git<const N: usize>(dir: &TempDir, args: [&str; N]) {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(dir.path())
        .status()
        .expect("git runs");
    assert!(status.success(), "git command failed");
}

fn write(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, contents).expect("write fixture");
}
