use std::fs;
use std::path::Path;
use std::process::Command;

use cargo_async_smell::analyzer::{Runtime, analyze_path};
use cargo_async_smell::baseline::diff_against_ref;
use cargo_async_smell::config::Config;
use cargo_async_smell::issue::{IssueType, Severity};
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
    let analysis = analyze_path(path, &Config::default(), Runtime::Tokio).expect("analysis");
    analysis
        .issues
        .into_iter()
        .map(|issue| issue.issue_type())
        .collect()
}

fn analysis(path: &Path) -> cargo_async_smell::Analysis {
    analyze_path(
        path,
        &Config::load_near(path).expect("config"),
        Runtime::Tokio,
    )
    .expect("analysis")
}

fn write_config(dir: &TempDir, source: &str) {
    fs::write(dir.path().join("async-smell.toml"), source).expect("config");
}

#[test]
fn detects_all_wave_one_issue_types() {
    let dir = write_project(
        r#"
        use std::time::Duration;

        struct Client;
        impl Client {
            async fn send(&self) {}
        }

        async fn work(_: u8) {}

        pub async fn risky(lock: std::sync::Mutex<u8>, client: Client) {
            let guard = lock.lock();
            std::thread::sleep(Duration::from_millis(1));
            client.send().await;
            for item in [1, 2, 3] {
                tokio::spawn(async move {
                    work(item).await;
                });
            }
            tokio::spawn(async move {
                loop {
                    work(1).await;
                }
            });
            work(*guard.as_ref().unwrap_or(&0)).await;
        }
        "#,
    );
    let types = issue_types(dir.path());
    for expected in [
        IssueType::GuardAcrossAwait,
        IssueType::BlockingInAsync,
        IssueType::UnboundedSpawn,
        IssueType::DetachedTask,
        IssueType::MissingTimeout,
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
        use std::time::Duration;

        struct Client;
        impl Client {
            async fn send(&self) {}
        }

        async fn work() {}

        pub async fn safe(lock: std::sync::Mutex<u8>, client: Client) {
            /* std::thread::sleep(Duration::from_secs(1)); */

            {
                let guard = lock.lock();
                drop(guard);
                work().await;
            }

            let handle = tokio::spawn(async move {
                work().await;
            });
            let _kept = handle;

            tokio::time::timeout(Duration::from_secs(1), client.send()).await;
        }

        #[cfg(test)]
        async fn ignored_fn(client: Client) {
            client.send().await;
        }

        #[cfg(test)]
        mod tests {
            async fn ignored(client: super::Client) {
                client.send().await;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        "#,
    );
    let types = issue_types(dir.path());
    assert!(
        !types.contains(&IssueType::GuardAcrossAwait),
        "drop before await should suppress guard finding: {types:?}"
    );
    assert!(
        !types.contains(&IssueType::UnboundedSpawn),
        "retained JoinHandle should not be unbounded-spawn: {types:?}"
    );
    assert!(
        !types.contains(&IssueType::MissingTimeout),
        "timeout-wrapped call and cfg(test) should be ignored: {types:?}"
    );
    assert!(
        !types.contains(&IssueType::BlockingInAsync),
        "block comment and cfg(test) should be ignored: {types:?}"
    );
}

#[test]
fn tokio_lock_await_and_short_guard_type_are_not_reported() {
    let dir = write_project(
        r#"
        use tokio::sync::MutexGuard;

        struct State;
        impl State {
            async fn ok(&self) {
                let g = self.state.lock().await;
                work().await;
                drop(g);
            }
        }

        struct Wrapper {
            state: tokio::sync::Mutex<u8>,
        }

        async fn typed(_: MutexGuard<'_, u8>) {
            work().await;
        }

        async fn work() {}
        "#,
    );
    let types = issue_types(dir.path());
    assert!(
        !types.contains(&IssueType::GuardAcrossAwait),
        "tokio lock().await must not be reported as sync guard: {types:?}"
    );
}

#[test]
fn try_lock_and_if_let_try_lock_are_reported_at_lower_severity() {
    let dir = write_project(
        r#"
        async fn work() {}

        pub async fn risky(m: std::sync::Mutex<u8>) {
            let g = m.try_lock().unwrap();
            work().await;
            drop(g);
        }

        pub async fn risky_if(m: std::sync::Mutex<u8>) {
            if let Ok(g) = m.try_lock() {
                work().await;
                drop(g);
            }
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let guards: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::GuardAcrossAwait)
        .collect();
    assert_eq!(
        guards.len(),
        2,
        "expected let and if-let try_lock findings: {guards:?}"
    );
    assert!(
        guards.iter().all(|issue| issue.severity <= Severity::High),
        "try_lock findings should be lower than critical without volatility: {guards:?}"
    );
}

#[test]
fn drop_matching_is_exact_and_accepts_mem_drop_paths() {
    let dir = write_project(
        r#"
        async fn work() {}

        pub async fn exact(m: std::sync::Mutex<u8>, other: std::sync::Mutex<u8>) {
            let g = m.lock().unwrap();
            let other_g = other.lock().unwrap();
            drop(other_g);
            work().await;
            drop(g);
        }

        pub async fn std_mem_drop(m: std::sync::Mutex<u8>) {
            let g = m.lock().unwrap();
            std::mem::drop(g);
            work().await;
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let guard_targets: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::GuardAcrossAwait)
        .map(|issue| issue.key.target.as_str())
        .collect();
    assert_eq!(
        guard_targets,
        vec!["g"],
        "only unrelated drop must leave g live: {guard_targets:?}"
    );
}

#[test]
fn nested_break_does_not_cancel_outer_spawn_loop() {
    let dir = write_project(
        r#"
        async fn work() {}

        pub fn helper() {
            tokio::spawn(async move {
                loop {
                    for item in [1] {
                        let _ = item;
                        break;
                    }
                    work().await;
                }
            });
        }
        "#,
    );
    let types = issue_types(dir.path());
    assert!(
        types.contains(&IssueType::DetachedTask),
        "inner break must not prove outer loop exits: {types:?}"
    );
}

#[test]
fn non_async_spawn_aliases_and_cancellation_contexts_are_handled() {
    let dir = write_project(
        r#"
        use std::time::Duration;
        use tokio::spawn;
        use tokio::task::spawn as task_spawn;
        use tokio::time::timeout;

        async fn work(_: u8) {}

        pub fn helper() {
            for item in [1, 2, 3] {
                spawn(async move {
                    work(item).await;
                });
            }
            task_spawn(async move {
                loop {
                    work(1).await;
                }
            });
            timeout(Duration::from_secs(1), tokio::spawn(async move {
                loop {}
            }));
            tokio::select! {
                _ = tokio::spawn(async move { loop {} }) => {}
            }
            let mut set = tokio::task::JoinSet::new();
            set.spawn(async move { loop {} });
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let types: Vec<_> = analysis
        .issues
        .iter()
        .map(|issue| issue.issue_type())
        .collect();
    assert!(
        types.contains(&IssueType::UnboundedSpawn),
        "bare spawn alias in non-async fn should be detected: {types:?}"
    );
    assert!(
        types.contains(&IssueType::DetachedTask),
        "task::spawn alias with infinite loop should be detected: {types:?}"
    );
    let detached = types
        .iter()
        .filter(|issue_type| **issue_type == IssueType::DetachedTask)
        .count();
    assert_eq!(
        detached, 1,
        "timeout/select/JoinSet cancellation contexts should not add detached findings: {types:?}"
    );
}

#[test]
fn blocking_call_aliases_and_configured_methods_are_detected() {
    let dir = write_project(
        r#"
        use std::fs;

        struct Conn;
        impl Conn {
            fn execute(&self) {}
        }

        pub async fn handler(conn: Conn) {
            fs::create_dir_all("/tmp/example").unwrap();
            conn.execute();
        }
        "#,
    );
    write_config(
        &dir,
        r#"
        blocking_calls = ["execute"]
        "#,
    );
    let analysis = analysis(dir.path());
    let targets: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::BlockingInAsync)
        .map(|issue| issue.key.target.as_str())
        .collect();
    assert!(
        targets.contains(&"std::fs::create_dir_all") && targets.contains(&"execute"),
        "expected std::fs alias and configured method findings, got {targets:?}"
    );
}

#[test]
fn reqwest_timeout_chain_and_channels_are_not_missing_timeout() {
    let dir = write_project(
        r#"
        use std::time::Duration;
        use tokio::sync::mpsc;

        struct Client;
        impl Client {
            fn get(&self, _: &str) -> Request { Request }
            fn post(&self, _: &str) -> Request { Request }
        }
        struct Request;
        impl Request {
            fn timeout(self, _: Duration) -> Self { self }
            async fn send(self) {}
        }

        pub async fn ok(client: Client) {
            let (tx, mut rx) = mpsc::channel(1);
            tx.send(1).await.unwrap();
            rx.recv().await;
            client.get("https://example.com").timeout(Duration::from_secs(1)).send().await;
        }

        pub async fn risky(client: Client) {
            client.post("https://example.com").send().await;
        }
        "#,
    );
    let analysis = analysis(dir.path());
    let timeout_lines: Vec<_> = analysis
        .issues
        .iter()
        .filter(|issue| issue.issue_type() == IssueType::MissingTimeout)
        .map(|issue| issue.line)
        .collect();
    assert_eq!(
        timeout_lines.len(),
        1,
        "only reqwest-like send without timeout should be reported: {timeout_lines:?}"
    );
}

#[test]
fn impl_qualified_keys_are_stable_when_same_named_method_is_inserted() {
    let before = write_project(
        r#"
        struct Client;
        impl Client { async fn send(&self) {} }
        struct A;
        impl A {
            pub async fn helper(client: Client) {
                client.send().await;
            }
        }
        struct B;
        impl B {
            pub async fn helper(client: Client) {
                client.send().await;
            }
        }
        "#,
    );
    let after = write_project(
        r#"
        struct Client;
        impl Client { async fn send(&self) {} }
        struct C;
        impl C {
            pub async fn helper(client: Client) {
                client.send().await;
            }
        }
        struct A;
        impl A {
            pub async fn helper(client: Client) {
                client.send().await;
            }
        }
        struct B;
        impl B {
            pub async fn helper(client: Client) {
                client.send().await;
            }
        }
        "#,
    );
    let before_keys: Vec<_> = analysis(before.path())
        .issues
        .into_iter()
        .filter(|issue| {
            issue.key.source.contains("A::helper") || issue.key.source.contains("B::helper")
        })
        .map(|issue| issue.key.source)
        .collect();
    let after_keys: Vec<_> = analysis(after.path())
        .issues
        .into_iter()
        .filter(|issue| {
            issue.key.source.contains("A::helper") || issue.key.source.contains("B::helper")
        })
        .map(|issue| issue.key.source)
        .collect();
    assert_eq!(before_keys, after_keys);
    assert!(after_keys.iter().all(|key| key.starts_with("src/lib.rs:")));
}

#[test]
fn suppress_comment_removes_issue() {
    let dir = write_project(
        r#"
        struct Client;
        impl Client {
            async fn send(&self) {}
        }

        // async-smell-allow: missing-timeout
        pub async fn allowed(client: Client) {
            client.send().await;
        }
        "#,
    );
    let analysis = analyze_path(dir.path(), &Config::default(), Runtime::Tokio).expect("analysis");
    assert_eq!(analysis.issues.len(), 0);
    assert_eq!(analysis.suppressed_issues, 1);
}

#[test]
fn identical_commit_baseline_has_no_new_or_resolved_issues() {
    let dir = write_project(
        r#"
        struct Client;
        impl Client {
            async fn send(&self) {}
        }

        pub async fn risky(client: Client) {
            client.send().await;
        }
        "#,
    );
    run_git(dir.path(), &["init"]);
    run_git(dir.path(), &["config", "user.email", "fixture@example.com"]);
    run_git(dir.path(), &["config", "user.name", "Fixture"]);
    run_git(dir.path(), &["add", "."]);
    run_git(dir.path(), &["commit", "-m", "initial"]);

    let config = Config::default();
    let analysis = analyze_path(dir.path(), &config, Runtime::Tokio).expect("analysis");
    let diff = diff_against_ref(dir.path(), &config, &analysis, "HEAD", Runtime::Tokio)
        .expect("baseline diff");
    assert_eq!(diff.new_issues.len(), 0);
    assert_eq!(diff.resolved_issues.len(), 0);
    assert_eq!(diff.unchanged, analysis.issues.len());
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
