use std::fs;
use std::path::Path;
use std::process::Command;

use cargo_trait_surface::baseline::diff_against_ref;
use cargo_trait_surface::{Config, IssueType, Severity, analyze_path};
use tempfile::TempDir;

fn write_project(source: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join("src")).expect("src dir");
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("manifest");
    fs::write(dir.path().join("src/lib.rs"), source).expect("lib");
    dir
}

fn issue_count(path: &Path, issue_type: IssueType) -> usize {
    let analysis = analyze_path(path, &Config::default()).expect("analysis");
    analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == issue_type)
        .count()
}

fn issues_of(path: &Path, issue_type: IssueType) -> Vec<cargo_trait_surface::Issue> {
    let analysis = analyze_path(path, &Config::default()).expect("analysis");
    analysis
        .issues
        .into_iter()
        .filter(|issue| issue.issue_type() == issue_type)
        .collect()
}

#[test]
fn detects_all_wave_one_issue_types() {
    let dir = write_project(
        r#"
use std::fmt::Debug;

pub trait Huge {
    type A; type B; type C; type D; type E;
    fn a(&self); fn b(&self); fn c(&self); fn d(&self); fn e(&self);
    fn f(&self); fn g(&self); fn h(&self); fn i(&self); fn j(&self); fn k(&self);
}

pub trait OnlyOne { fn run(&self); }
pub struct Concrete;
impl OnlyOne for Concrete { fn run(&self) {} }

pub trait ObjectRisk {
    fn construct(&self) -> Self;
    fn generic<T>(&self, value: T);
}
pub fn takes_dyn(value: &dyn ObjectRisk) { let _ = value; }

pub trait Blanket {}
impl<T: Clone> Blanket for T {}

pub fn direct_file(file: std::fs::File) { let _ = file; }
"#,
    );
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analysis");
    let types = analysis
        .issues
        .iter()
        .map(|issue| issue.issue_type())
        .collect::<Vec<_>>();
    assert!(types.contains(&IssueType::OversizedTrait));
    assert!(types.contains(&IssueType::SingleImplAbstraction));
    assert!(types.contains(&IssueType::ObjectSafetyRisk));
    assert!(types.contains(&IssueType::BroadBlanketImpl));
    assert!(types.contains(&IssueType::UnmockableBoundary));
    let single = analysis
        .issues
        .iter()
        .find(|issue| issue.issue_type() == IssueType::SingleImplAbstraction)
        .expect("single impl issue");
    assert_eq!(single.severity, Severity::Low);
    assert!(analysis.issues.iter().any(|issue| {
        issue.key.source.starts_with("src/lib.rs:")
            && !issue.key.source.starts_with('/')
            && !issue.key.target.contains(':')
    }));
}

#[test]
fn negative_cases_do_not_report_false_positives() {
    let dir = write_project(
        r#"
/* pub trait Commented { fn fake(&self); } */

pub trait TwoImpls { fn run(&self); }
pub struct A;
pub struct B;
impl TwoImpls for A { fn run(&self) {} }
impl TwoImpls for B { fn run(&self) {} }

pub trait TestOnly { fn run(&self); }
#[cfg(test)]
mod tests {
    use super::*;
    struct TestImpl;
    impl TestOnly for TestImpl { fn run(&self) {} }
}

pub trait ObjectLooking {
    fn construct(&self) -> Self;
}
pub struct ObjectA;
pub struct ObjectB;
impl ObjectLooking for ObjectA {
    fn construct(&self) -> Self { ObjectA }
}
impl ObjectLooking for ObjectB {
    fn construct(&self) -> Self { ObjectB }
}
"#,
    );
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analysis");
    assert!(!analysis.traits.iter().any(|tr| tr.name == "Commented"));
    assert!(!analysis.issues.iter().any(|issue| issue.issue_type()
        == IssueType::SingleImplAbstraction
        && issue.key.target == "one-production-impl"));
    assert_eq!(
        issue_count(dir.path(), IssueType::ObjectSafetyRisk),
        0,
        "dyn-free object-looking trait must not report object-safety-risk"
    );
}

#[test]
fn unmockable_boundary_scans_signatures_not_function_bodies() {
    let dir = write_project(
        r#"
pub fn read_config(path: String) -> String {
    let _f: std::fs::File = todo!();
    path
}

pub fn direct_file(file: std::fs::File) { let _ = file; }
"#,
    );
    let issues = issues_of(dir.path(), IssueType::UnmockableBoundary);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].key.source.ends_with(":direct_file"));
}

#[test]
fn broad_blanket_impl_detects_unconstrained_multichar_and_where_bounds() {
    let dir = write_project(
        r#"
pub trait AnyMarker {}
impl<T> AnyMarker for T {}

pub trait CloneMarker {}
impl<Value: Clone> CloneMarker for Value {}

pub trait WhereMarker {}
impl<Item> WhereMarker for Item where Item: Send + Sync {}
"#,
    );
    let issues = issues_of(dir.path(), IssueType::BroadBlanketImpl);
    let targets = issues
        .iter()
        .map(|issue| issue.key.target.as_str())
        .collect::<Vec<_>>();
    assert!(
        targets
            .iter()
            .any(|target| target.contains("impl<unconstrained> for T"))
    );
    assert!(
        targets
            .iter()
            .any(|target| target.contains("impl<Clone> for Value"))
    );
    assert!(
        targets
            .iter()
            .any(|target| target.contains("impl<Send + Sync> for Item"))
    );
}

#[test]
fn object_safety_handles_self_parameters_self_sized_and_async_trait() {
    let dir = write_project(
        r#"
pub trait Compare {
    fn same_as(&self, other: Self);
}
pub fn compare_dyn(value: &dyn Compare) { let _ = value; }

pub trait Factory {
    fn make<T>(&self) where Self: Sized;
}
pub fn factory_dyn(value: &dyn Factory) { let _ = value; }

#[async_trait]
pub trait AsyncService {
    async fn run(&self);
}
pub fn async_dyn(value: &dyn AsyncService) { let _ = value; }
"#,
    );
    let issues = issues_of(dir.path(), IssueType::ObjectSafetyRisk);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("same_as"));
}

#[test]
fn direct_cfg_test_on_items_is_excluded() {
    let dir = write_project(
        r#"
pub trait Port {}

#[cfg(test)]
struct Mock;

#[cfg(test)]
impl Port for Mock {}

#[cfg(test)]
pub struct TestFile {
    file: std::fs::File,
}
"#,
    );
    assert_eq!(issue_count(dir.path(), IssueType::UnmockableBoundary), 0);
    let singles = issues_of(dir.path(), IssueType::SingleImplAbstraction);
    assert_eq!(singles.len(), 1);
    assert!(singles[0].message.contains("no non-test implementations"));
}

#[test]
fn impl_method_issue_keys_include_enclosing_type() {
    let dir = write_project(
        r#"
pub struct Alpha;
impl Alpha {
    pub fn run(file: std::fs::File) { let _ = file; }
}

pub struct Beta;
impl Beta {
    pub fn run(file: std::fs::File) { let _ = file; }
}
"#,
    );
    let mut sources = issues_of(dir.path(), IssueType::UnmockableBoundary)
        .into_iter()
        .map(|issue| issue.key.source)
        .collect::<Vec<_>>();
    sources.sort();
    assert_eq!(
        sources,
        vec!["src/lib.rs:Alpha::run", "src/lib.rs:Beta::run"]
    );
}

#[test]
fn issue_keys_are_stable_for_reordered_bounds_and_risky_methods() {
    let left = write_project(
        r#"
pub trait Marker {}
impl<T: Send + Sync> Marker for T {}

pub trait Risk {
    fn generic<T>(&self, value: T);
    fn build(&self) -> Self;
}
pub fn dyn_risk(value: &dyn Risk) { let _ = value; }
"#,
    );
    let right = write_project(
        r#"
pub trait Marker {}
impl<T: Sync + Send> Marker for T {}

pub trait Risk {
    fn build(&self) -> Self;
    fn generic<T>(&self, value: T);
}
pub fn dyn_risk(value: &dyn Risk) { let _ = value; }
"#,
    );
    let left_keys = key_targets(left.path());
    let right_keys = key_targets(right.path());
    assert_eq!(left_keys, right_keys);
}

fn key_targets(path: &Path) -> Vec<(IssueType, String)> {
    let mut keys = analyze_path(path, &Config::default())
        .expect("analysis")
        .issues
        .into_iter()
        .filter(|issue| {
            matches!(
                issue.issue_type(),
                IssueType::BroadBlanketImpl | IssueType::ObjectSafetyRisk
            )
        })
        .map(|issue| (issue.issue_type(), issue.key.target))
        .collect::<Vec<_>>();
    keys.sort_by(|a, b| a.0.id().cmp(b.0.id()).then_with(|| a.1.cmp(&b.1)));
    keys
}

#[test]
fn zero_impl_trait_is_low_but_does_not_affect_grade() {
    let dir = write_project(
        r#"
pub trait DeadA {}
pub trait DeadB {}
pub trait DeadC {}
"#,
    );
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analysis");
    assert_eq!(
        analysis
            .issues
            .iter()
            .filter(|issue| issue.issue_type() == IssueType::SingleImplAbstraction)
            .count(),
        3
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.severity == Severity::Low)
    );
    assert_eq!(analysis.grade.to_string(), "A");
}

#[test]
fn intent_declaration_suppresses_single_impl_abstraction() {
    let dir = write_project(
        r#"
pub trait Port { fn call(&self); }
pub struct Real;
impl Port for Real { fn call(&self) {} }
"#,
    );
    fs::write(
        dir.path().join("trait-surface.toml"),
        r#"[intent]
intentional_abstractions = ["Port"]
"#,
    )
    .expect("config");
    let config = Config::load_near(dir.path()).expect("config load");
    let analysis = analyze_path(dir.path(), &config).expect("analysis");
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::SingleImplAbstraction)
    );
}

#[test]
fn line_suppression_removes_issue() {
    let dir = write_project(
        r#"
// trait-surface-allow: unmockable-boundary
pub fn direct_file(file: std::fs::File) { let _ = file; }
"#,
    );
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analysis");
    assert_eq!(analysis.suppressed_issues, 1);
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::UnmockableBoundary)
    );
}

#[test]
fn same_commit_baseline_has_no_new_or_resolved_issues() {
    let dir = write_project(
        r#"
pub trait OnlyOne { fn run(&self); }
pub struct Concrete;
impl OnlyOne for Concrete { fn run(&self) {} }
"#,
    );
    run_git(dir.path(), &["init"]);
    run_git(dir.path(), &["add", "."]);
    run_git(
        dir.path(),
        &[
            "-c",
            "user.name=Trait Surface Test",
            "-c",
            "user.email=trait-surface@example.invalid",
            "commit",
            "-m",
            "fixture",
        ],
    );
    let config = Config::default();
    let analysis = analyze_path(dir.path(), &config).expect("analysis");
    let diff = diff_against_ref(dir.path(), &config, &analysis, "HEAD").expect("baseline diff");
    assert_eq!(diff.new_issues.len(), 0);
    assert_eq!(diff.resolved_issues.len(), 0);
}

#[test]
fn cli_modes_emit_expected_surfaces() {
    let dir = write_project(
        r#"
pub trait OnlyOne { fn run(&self); }
pub struct Concrete;
impl OnlyOne for Concrete { fn run(&self) {} }
pub fn direct_file(file: std::fs::File) { let _ = file; }
"#,
    );
    let bin = env!("CARGO_BIN_EXE_cargo-trait-surface");
    assert!(run_cli(bin, dir.path(), &["--json"]).contains("\"issues\""));
    assert!(run_cli(bin, dir.path(), &["--ai"]).contains("repair plan"));
    assert!(run_cli(bin, dir.path(), &["--blind-spots"]).contains("name-resolution"));
    assert!(run_cli(bin, dir.path(), &["--trait", "OnlyOne"]).contains("Impls"));
    assert!(run_cli(bin, dir.path(), &["--jp", "--trait", "OnlyOne"]).contains("トレイト:"));
    assert!(run_cli(bin, dir.path(), &["--jp", "--summary"]).contains("評価"));

    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["--check", "--fail-on", "low"])
        .output()
        .expect("cli check");
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(!output.status.success());
    assert!(stdout.contains("check: FAIL"));
}

#[test]
fn trait_mode_not_found_exits_one_and_suggests_case_match() {
    let dir = write_project(
        r#"
pub trait Repository {}
"#,
    );
    let bin = env!("CARGO_BIN_EXE_cargo-trait-surface");
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["--trait", "repository"])
        .output()
        .expect("cli trait");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("error: trait 'repository' not found"));
    assert!(stderr.contains("Repository at src/lib.rs:"));
}

#[test]
fn trait_mode_rejects_ambiguous_names_without_merging() {
    let dir = write_project(
        r#"
pub mod alpha {
    pub trait Service {}
}

pub mod beta {
    pub trait Service {}
}
"#,
    );
    let bin = env!("CARGO_BIN_EXE_cargo-trait-surface");
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["--trait", "Service"])
        .output()
        .expect("cli trait");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("error: trait 'Service' is ambiguous; matches:"));
    assert!(stderr.matches("src/lib.rs:").count() >= 2);
    assert!(output.stdout.is_empty());
}

#[test]
fn trait_mode_conflicts_with_check_and_baseline() {
    let dir = write_project(
        r#"
pub trait Repository {}
"#,
    );
    let bin = env!("CARGO_BIN_EXE_cargo-trait-surface");
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["--trait", "Repository", "--check"])
        .output()
        .expect("cli trait");
    assert_eq!(output.status.code(), Some(2));

    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["--trait", "Repository", "--baseline", "HEAD"])
        .output()
        .expect("cli trait");
    assert_eq!(output.status.code(), Some(2));
}

fn run_cli(bin: &str, path: &Path, args: &[&str]) -> String {
    let output = Command::new(bin)
        .current_dir(path)
        .args(args)
        .output()
        .expect("cli");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8")
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
