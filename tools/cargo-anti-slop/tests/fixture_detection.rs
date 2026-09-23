use std::fs;
use std::path::Path;

use cargo_anti_slop::analyzer::analyze_path;
use cargo_anti_slop::config::Config;
use cargo_anti_slop::issue::{IssueType, Severity};
use tempfile::TempDir;

fn write_project(source: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("src")).expect("src dir");
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"
            [package]
            name = "fixture"
            version = "0.1.0"
            edition = "2024"
        "#,
    )
    .expect("manifest");
    fs::write(dir.path().join("src/lib.rs"), source).expect("source");
    dir
}

fn issue_types(path: &Path) -> Vec<IssueType> {
    let analysis = analyze_path(path, &Config::default()).expect("analysis");
    analysis
        .issues
        .into_iter()
        .map(|issue| issue.issue_type())
        .collect()
}

fn analysis(path: &Path) -> cargo_anti_slop::Analysis {
    analyze_path(path, &Config::load_near(path).expect("config")).expect("analysis")
}

fn write_config(dir: &TempDir, source: &str) {
    fs::write(dir.path().join("anti-slop.toml"), source).expect("config");
}

fn write_multi_project(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"
            [package]
            name = "fixture"
            version = "0.1.0"
            edition = "2024"
        "#,
    )
    .expect("manifest");
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, content).expect("source");
    }
    dir
}

const ALL_RULES_FIXTURE: &str = r#"
use std::any::Any;
use std::collections::HashMap;
use serde_json::{json, Value};

pub type RawConfig = Value;

pub struct UserData {
    pub attributes: HashMap<String, Value>,
}

pub fn ingest(payload: Value) -> Value {
    payload
}

pub fn dispatch(event: &dyn Any) -> u64 {
    if event.is::<u8>() {
        1
    } else if let Some(port) = event.downcast_ref::<u16>() {
        *port as u8 as u64
    } else {
        0
    }
}

pub fn boot(raw: &str) -> u16 {
    let config = json!({ "port": 8080 });
    let port = config["port"].to_string();
    std::fs::remove_file("state.json").ok();
    let _ = port;
    raw.parse::<u16>().unwrap_or_default()
}

pub fn validate_locale(raw: &str) -> bool {
    raw.len() == 2
}

pub fn attach(customer_id: u64, order_id: u64) -> bool {
    customer_id == order_id
}

#[derive(Default)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(raw: &str) -> Result<Self, String> {
        if raw.is_empty() {
            Err("empty".to_string())
        } else {
            Ok(Self(raw.to_string()))
        }
    }
}
"#;

#[test]
fn detects_all_wave_one_issue_types() {
    let dir = write_project(ALL_RULES_FIXTURE);
    let types = issue_types(dir.path());
    for expected in [
        IssueType::ChainedCast,
        IssueType::ErasedSignature,
        IssueType::ErasedAlias,
        IssueType::ErasedDictionary,
        IssueType::RuntimeDowncast,
        IssueType::WidenThenAssert,
        IssueType::DefaultSwallow,
        IssueType::VagueSymbolName,
        IssueType::BoolValidation,
        IssueType::RawIdParams,
        IssueType::ConstructorBypass,
    ] {
        assert!(
            types.contains(&expected),
            "missing {expected:?}; got {types:?}"
        );
    }
}

#[test]
fn negative_fixtures_are_not_reported() {
    let dir = write_project(
        r#"
        use std::collections::HashMap;

        pub struct ServerConfig {
            pub port: u16,
            pub labels: HashMap<String, String>,
        }

        pub fn parse_config(raw: &str) -> Result<ServerConfig, std::num::ParseIntError> {
            Ok(ServerConfig {
                port: raw.parse::<u16>()?,
                labels: HashMap::new(),
            })
        }

        pub fn widen(value: u8) -> u64 {
            u64::from(value)
        }
        "#,
    );
    let types = issue_types(dir.path());
    assert!(types.is_empty(), "expected no issues, got {types:?}");
}

#[test]
fn test_code_is_excluded() {
    let dir = write_project(
        r#"
        #[cfg(test)]
        mod tests {
            use serde_json::{json, Value};

            pub fn helper(payload: Value) -> Value {
                let body = json!({ "ok": true });
                let _ = body["ok"].as_bool();
                payload
            }

            #[test]
            fn roundtrip() {
                let raw = "8080".parse::<u16>().unwrap_or_default();
                let _ = raw as u8 as u64;
            }
        }
        "#,
    );
    let types = issue_types(dir.path());
    assert!(types.is_empty(), "expected no issues, got {types:?}");
}

#[test]
fn trait_impl_signatures_are_skipped_but_trait_decl_is_checked() {
    let dir = write_project(
        r#"
        use serde_json::Value;

        pub trait Exporter {
            fn export(&self) -> Value;
        }

        pub struct Csv;

        impl Exporter for Csv {
            fn export(&self) -> Value {
                Value::Null
            }
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let erased: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::ErasedSignature)
        .collect();
    assert_eq!(erased.len(), 1, "got {erased:?}");
    assert!(erased[0].key.source.contains("Exporter::export"));
}

#[test]
fn suppression_comment_is_honored() {
    let dir = write_project(
        r#"
        use serde_json::Value;

        // anti-slop-allow: erased-signature
        pub fn passthrough(payload: Value) -> Value {
            payload
        }
        "#,
    );
    let analysis = analysis(dir.path());
    assert_eq!(analysis.suppressed_issues, 2);
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::ErasedSignature)
    );
}

#[test]
fn config_allow_disables_issue_type() {
    let dir = write_project(
        r#"
        pub struct UserData {
            pub id: u64,
        }
        "#,
    );
    write_config(&dir, r#"allow = ["vague-symbol-name"]"#);
    let analysis = analysis(dir.path());
    assert!(analysis.issues.is_empty(), "got {:?}", analysis.issues);
}

#[test]
fn config_extends_vague_words() {
    let dir = write_project(
        r#"
        pub struct PaymentBlob {
            pub id: u64,
        }
        "#,
    );
    write_config(&dir, r#"vague_words = ["Blob"]"#);
    let analysis = analysis(dir.path());
    assert!(
        analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::VagueSymbolName)
    );
}

#[test]
fn pub_erased_signature_scores_higher_than_private() {
    let dir = write_project(
        r#"
        use serde_json::Value;

        pub fn public_boundary(payload: Value) -> u8 {
            let _ = payload;
            0
        }

        fn private_boundary(payload: Value) -> u8 {
            let _ = payload;
            0
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let severity_for = |source_fragment: &str| -> Severity {
        analysis
            .issues
            .iter()
            .find(|issue| issue.key.source.contains(source_fragment))
            .map(|issue| issue.severity)
            .expect("issue present")
    };
    assert!(severity_for("public_boundary") > severity_for("private_boundary"));
}

#[test]
fn orphan_file_is_reported_and_masks_its_content_findings() {
    let dir = write_multi_project(&[
        ("src/lib.rs", "mod used;\n"),
        ("src/used.rs", "pub fn ok() {}\n"),
        (
            "src/legacy.rs",
            "use serde_json::Value;\npub fn ingest(payload: Value) -> Value {\n    payload\n}\n",
        ),
    ]);
    let analysis = analysis(dir.path());
    let orphans: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::OrphanFile)
        .collect();
    assert_eq!(orphans.len(), 1, "{:?}", analysis.issues);
    assert_eq!(orphans[0].rel_path, "src/legacy.rs");
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::ErasedSignature),
        "orphan content findings should be masked: {:?}",
        analysis.issues
    );
}

#[test]
fn orphan_file_suppression_comment_is_honored() {
    let dir = write_multi_project(&[
        ("src/lib.rs", "mod used;\n"),
        ("src/used.rs", "pub fn ok() {}\n"),
        (
            "src/keep.rs",
            "// anti-slop-allow: orphan-file -- staged for the next wave\n",
        ),
    ]);
    let analysis = analysis(dir.path());
    assert_eq!(analysis.suppressed_issues, 1);
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::OrphanFile)
    );
}

#[test]
fn bin_roots_and_module_dirs_are_not_orphans() {
    let dir = write_multi_project(&[
        ("src/main.rs", "mod app;\nfn main() {}\n"),
        ("src/app.rs", "pub mod sub;\n"),
        ("src/app/sub.rs", "pub fn ok() {}\n"),
        ("src/bin/extra.rs", "fn main() {}\n"),
    ]);
    let analysis = analysis(dir.path());
    assert!(
        !analysis
            .issues
            .iter()
            .any(|issue| issue.issue_type() == IssueType::OrphanFile),
        "{:?}",
        analysis.issues
    );
}

#[test]
fn stable_keys_do_not_move_with_lines() {
    let with_padding = write_project(&format!("\n\n\n{ALL_RULES_FIXTURE}"));
    let without_padding = write_project(ALL_RULES_FIXTURE);
    let mut left: Vec<String> = analysis(with_padding.path())
        .issues
        .iter()
        .map(|issue| {
            format!(
                "{}|{}|{}",
                issue.key.issue_type, issue.key.source, issue.key.target
            )
        })
        .collect();
    let mut right: Vec<String> = analysis(without_padding.path())
        .issues
        .iter()
        .map(|issue| {
            format!(
                "{}|{}|{}",
                issue.key.issue_type, issue.key.source, issue.key.target
            )
        })
        .collect();
    left.sort();
    right.sort();
    assert_eq!(left, right);
}
