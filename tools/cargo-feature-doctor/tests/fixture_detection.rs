use std::fs;
use std::path::Path;
use std::process::Command;

use cargo_feature_doctor::baseline::{diff, diff_against_ref};
use cargo_feature_doctor::{Config, IssueType, analyze_path};
use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn risky_dep(dir: &Path) {
    write(
        &dir.join("serde/Cargo.toml"),
        r#"
        [package]
        name = "serde"
        version = "1.0.0"
        edition = "2024"
        "#,
    );
    write(&dir.join("serde/src/lib.rs"), "pub trait Serialize {}\n");
    write(
        &dir.join("risky-dep/Cargo.toml"),
        r#"
        [package]
        name = "risky-dep"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        serde = { path = "../serde", optional = true }

        [features]
        default = ["heavy"]
        heavy = ["dep:serde"]
        "#,
    );
    write(&dir.join("risky-dep/src/lib.rs"), "pub fn dep() {}\n");
}

fn fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    risky_dep(dir.path());
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "fixture"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        risky-dep = { path = "risky-dep" }
        serde = { path = "serde", optional = true }

        [features]
        default = []
        rt-tokio = []
        rt-async-std = []
        tls-native = []
        tls-rustls = []
        native-tls = []
        rustls-tls = []
        serde = ["dep:serde"]
        narrow = []
        remove-api = []
        "#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        pub fn leak(value: serde::Serialize) {}

        #[cfg(all(feature = "narrow", not(feature = "serde")))]
        pub fn narrow_only() {}

        #[cfg(not(feature = "remove-api"))]
        pub fn removed_when_enabled() {}

        /*
        #[cfg(feature = "ghost")]
        pub fn ghost() {}
        */
        "#,
    );
    dir
}

fn negative_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    risky_dep(dir.path());
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "negative"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        risky-dep = { path = "risky-dep", default-features = false }
        serde = { path = "serde", optional = true }

        [features]
        default = []
        rt-tokio = []
        rt-async-std = []
        serde = ["dep:serde"]
        "#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        #[cfg(all(feature = "rt-tokio", feature = "rt-async-std"))]
        compile_error!("choose one runtime");

        #[cfg(feature = "serde")]
        pub fn gated(value: serde::Serialize) {}

        pub fn stable_api() {}

        /*
        #[cfg(all(feature = "ghost", not(feature = "serde")))]
        pub fn ghost() {}
        */
        "#,
    );
    dir
}

fn renamed_gate_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    risky_dep(dir.path());
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "renamed-gate"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        serde = { path = "serde", optional = true }

        [features]
        default = []
        serde-support = ["dep:serde"]
        "#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        #[cfg(feature = "serde-support")]
        pub fn gated(value: serde::Serialize) {}
        "#,
    );
    dir
}

fn logical_gate_fixture() -> TempDir {
    let dir = renamed_gate_fixture();
    let manifest = fs::read_to_string(dir.path().join("Cargo.toml")).expect("manifest");
    write(
        &dir.path().join("Cargo.toml"),
        &manifest.replace(
            "serde-support = [\"dep:serde\"]",
            "serde-support = [\"dep:serde\"]\nother = []",
        ),
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        #[cfg(not(feature = "serde-support"))]
        pub fn negated(value: serde::Serialize) {}

        #[cfg(any(feature = "serde-support", feature = "other"))]
        pub fn broad(value: serde::Serialize) {}

        #[cfg(all(feature = "serde-support", not(feature = "other")))]
        pub fn strict(value: serde::Serialize) {}
        "#,
    );
    dir
}

fn precision_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    risky_dep(dir.path());
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "precision"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        serde = { path = "serde", optional = true }

        [features]
        default = []
        serde = ["dep:serde"]
        fs = []
        "#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        #[doc(cfg(feature = "fs"))]
        pub fn documented() {}

        mod private {
            pub fn hidden(value: serde::Serialize) {}
        }

        pub struct Public;

        impl Public {
            pub fn body_only() {
                let _ = "serde::Serialize";
            }

            /// Mentions serde::Serialize in docs only.
            pub fn doc_only() {}
        }

        #[cfg(not(unix))]
        pub fn platform_only() {}
        "#,
    );
    dir
}

fn same_name_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    risky_dep(dir.path());
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "same-name"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        serde = { path = "serde", optional = true }

        [features]
        default = []
        serde = ["dep:serde"]
        "#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        pub mod a {
            pub fn run(value: serde::Serialize) {}
        }

        pub mod b {
            pub fn run(value: serde::Serialize) {}
        }
        "#,
    );
    dir
}

fn indirect_default_leak_fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        &dir.path().join("tokio-rt/Cargo.toml"),
        r#"
        [package]
        name = "tokio-rt"
        version = "0.1.0"
        edition = "2024"
        "#,
    );
    write(&dir.path().join("tokio-rt/src/lib.rs"), "pub fn rt() {}\n");
    write(
        &dir.path().join("runtime-dep/Cargo.toml"),
        r#"
        [package]
        name = "runtime-dep"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        tokio-rt = { path = "../tokio-rt", optional = true }

        [features]
        default = ["runtime-tokio"]
        runtime-tokio = ["dep:tokio-rt"]
        "#,
    );
    write(
        &dir.path().join("runtime-dep/src/lib.rs"),
        "pub fn dep() {}\n",
    );
    write(
        &dir.path().join("Cargo.toml"),
        r#"
        [package]
        name = "indirect-default"
        version = "0.1.0"
        edition = "2024"
        publish = false

        [dependencies]
        runtime-dep = { path = "runtime-dep" }
        "#,
    );
    write(&dir.path().join("src/lib.rs"), "pub fn stable() {}\n");
    dir
}

#[test]
fn detects_five_issue_types() {
    let dir = fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze fixture");
    let types = analysis
        .issues
        .iter()
        .map(|issue| issue.issue_type())
        .collect::<std::collections::HashSet<_>>();
    assert!(types.contains(&IssueType::DefaultLeak));
    assert!(types.contains(&IssueType::ExclusiveUndeclared));
    assert!(types.contains(&IssueType::UntestedCfgPath));
    assert!(types.contains(&IssueType::OptionalDepExposure));
    assert!(types.contains(&IssueType::NonAdditiveFeature));
}

#[test]
fn negatives_do_not_trigger() {
    let dir = negative_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze negative fixture");
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::DefaultLeak)
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::ExclusiveUndeclared)
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::OptionalDepExposure)
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| !issue.key.target.contains("ghost"))
    );
}

#[test]
fn doc_cfg_private_mod_impl_body_docs_and_non_feature_cfg_are_not_false_positives() {
    let dir = precision_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze precision fixture");
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::OptionalDepExposure),
        "{:#?}",
        analysis.issues
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::UntestedCfgPath),
        "{:#?}",
        analysis.issues
    );
}

#[test]
fn optional_dep_gate_accepts_feature_that_enables_dep() {
    let dir = renamed_gate_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze renamed fixture");
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::OptionalDepExposure),
        "{:#?}",
        analysis.issues
    );
}

#[test]
fn optional_dep_gate_requires_logical_implication() {
    let dir = logical_gate_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze logical fixture");
    let exposed = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::OptionalDepExposure)
        .map(|issue| issue.key.source.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        exposed.contains("src/lib.rs:negated"),
        "{:#?}",
        analysis.issues
    );
    assert!(
        exposed.contains("src/lib.rs:broad"),
        "{:#?}",
        analysis.issues
    );
    assert!(
        !exposed.contains("src/lib.rs:strict"),
        "{:#?}",
        analysis.issues
    );
}

#[test]
fn same_named_items_keep_distinct_stable_keys() {
    let dir = same_name_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze same-name fixture");
    let sources = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::OptionalDepExposure)
        .map(|issue| issue.key.source.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(sources.len(), 2, "{:#?}", analysis.issues);
    assert!(sources.contains("src/lib.rs:a::run"));
    assert!(sources.contains("src/lib.rs:b::run"));
}

#[test]
fn indirect_default_feature_leak_is_detected() {
    let dir = indirect_default_leak_fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze indirect fixture");
    let leak = analysis
        .issues
        .iter()
        .find(|issue| issue.issue_type() == IssueType::DefaultLeak)
        .expect("default leak");
    assert!(leak.features.iter().any(|feature| feature == "tokio-rt"));
}

#[test]
fn native_tls_and_rustls_tls_are_exclusive_candidates() {
    let dir = fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze fixture");
    assert!(analysis.issues.iter().any(|issue| {
        issue.issue_type() == IssueType::ExclusiveUndeclared
            && issue.key.target == "native-tls+rustls-tls"
    }));
}

#[test]
fn suppressions_and_config_allow_work() {
    let dir = fixture();
    write(
        &dir.path().join("feature-doctor.toml"),
        r#"allow = ["default-leak"]"#,
    );
    write(
        &dir.path().join("src/lib.rs"),
        r#"
        // feature-doctor-allow: optional-dep-exposure
        pub fn leak(value: serde::Serialize) {}

        #[cfg(all(feature = "narrow", not(feature = "serde")))]
        pub fn narrow_only() {}

        #[cfg(not(feature = "remove-api"))]
        pub fn removed_when_enabled() {}
        "#,
    );
    let config = Config::load_near(dir.path()).expect("load config");
    let analysis = analyze_path(dir.path(), &config).expect("analyze suppressed fixture");
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::DefaultLeak)
    );
    assert!(
        analysis
            .issues
            .iter()
            .all(|issue| issue.issue_type() != IssueType::OptionalDepExposure)
    );
    assert_eq!(analysis.suppressed_issues, 1);
}

#[test]
fn identical_analysis_has_no_baseline_delta() {
    let dir = fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze fixture");
    let delta = diff(&analysis, &analysis);
    assert_eq!(delta.new_issues.len(), 0);
    assert_eq!(delta.resolved_issues.len(), 0);
    assert_eq!(delta.unchanged, analysis.issues.len());
}

#[test]
fn same_commit_baseline_ref_has_no_delta() {
    let dir = fixture();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "fixture@example.com"],
        vec!["config", "user.name", "Fixture"],
        vec!["add", "."],
        vec!["commit", "-m", "fixture"],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let config = Config::default();
    let analysis = analyze_path(dir.path(), &config).expect("analyze fixture");
    let delta = diff_against_ref(dir.path(), &config, &analysis, "HEAD").expect("baseline diff");
    assert_eq!(delta.new_issues.len(), 0);
    assert_eq!(delta.resolved_issues.len(), 0);
    assert_eq!(delta.unchanged, analysis.issues.len());
}

#[test]
fn cli_modes_render() {
    let dir = fixture();
    let bin = env!("CARGO_BIN_EXE_cargo-feature-doctor");
    for args in [
        vec!["--json"],
        vec!["--ai"],
        vec!["--blind-spots"],
        vec!["--matrix"],
        vec!["--suggest-hack"],
        vec!["--jp", "--summary"],
    ] {
        let output = Command::new(bin)
            .args(args)
            .arg(dir.path())
            .output()
            .expect("run cargo-feature-doctor");
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }
    let output = Command::new(bin)
        .arg("--check")
        .arg("--fail-on")
        .arg("high")
        .arg(dir.path())
        .output()
        .expect("run check");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("check: FAIL"));
}

#[test]
fn cli_check_applies_to_early_output_modes() {
    let dir = fixture();
    let bin = env!("CARGO_BIN_EXE_cargo-feature-doctor");
    for args in [
        vec!["--blind-spots", "--check"],
        vec!["--matrix", "--check"],
        vec!["--suggest-hack", "--check"],
    ] {
        let output = Command::new(bin)
            .args(args)
            .arg(dir.path())
            .output()
            .expect("run cargo-feature-doctor");
        assert!(
            !output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("check: FAIL"));
    }
}

#[test]
fn suggest_hack_preserves_not_feature_polarity() {
    let dir = fixture();
    let bin = env!("CARGO_BIN_EXE_cargo-feature-doctor");
    let output = Command::new(bin)
        .arg("--suggest-hack")
        .arg(dir.path())
        .output()
        .expect("run cargo-feature-doctor");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--features \"narrow\""), "{stdout}");
    assert!(!stdout.contains("--features \"narrow serde\""), "{stdout}");
    assert!(stdout.contains("keep serde disabled"), "{stdout}");
}

#[test]
fn matrix_omits_default_feature_row_and_supports_japanese() {
    let dir = fixture();
    let bin = env!("CARGO_BIN_EXE_cargo-feature-doctor");
    let output = Command::new(bin)
        .args(["--matrix", "--jp"])
        .arg(dir.path())
        .output()
        .expect("run cargo-feature-doctor");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cfg 参照"), "{stdout}");
    assert!(
        !stdout.lines().any(|line| line.starts_with("default |")),
        "{stdout}"
    );
}

#[test]
fn blind_spots_disclose_cross_file_cfg_limit() {
    let dir = fixture();
    let analysis = analyze_path(dir.path(), &Config::default()).expect("analyze fixture");
    assert!(
        analysis
            .blind_spots
            .blind_spots
            .iter()
            .any(|blind| blind.id == "cross-file-cfg-mod-propagation")
    );
}
