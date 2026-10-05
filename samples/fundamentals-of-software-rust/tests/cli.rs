use std::process::{Command, Output};

fn configuration(port: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_configuration"));
    command.env_remove("APP_PORT");
    if let Some(port) = port {
        command.env("APP_PORT", port);
    }
    command.output().unwrap()
}

#[test]
fn configuration_reads_the_child_environment_without_mutating_the_test_runner() {
    let default = configuration(None);
    assert!(default.status.success());
    assert_eq!(
        String::from_utf8(default.stdout).unwrap(),
        "Configured port: 8080\n"
    );
    assert!(default.stderr.is_empty());
    let custom = configuration(Some("3000"));
    assert!(custom.status.success());
    assert_eq!(
        String::from_utf8(custom.stdout).unwrap(),
        "Configured port: 3000\n"
    );
}

#[test]
fn invalid_environment_exits_with_a_diagnostic_and_no_success_output() {
    for port in ["", "0", "65536", "accidentally-pasted-secret"] {
        let result = configuration(Some(port));
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        let diagnostic = String::from_utf8(result.stderr).unwrap();
        assert!(diagnostic.contains("APP_PORT must be a decimal integer between 1 and 65535"));
        assert!(!diagnostic.contains("accidentally-pasted-secret"));
    }
}

#[cfg(unix)]
#[test]
fn non_unicode_environment_is_reported() {
    use std::os::unix::ffi::OsStringExt;
    let result = Command::new(env!("CARGO_BIN_EXE_configuration"))
        .env("APP_PORT", std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("APP_PORT must contain valid Unicode")
    );
}

#[test]
fn pricing_demo_matches_the_book_examples() {
    let result = Command::new(env!("CARGO_BIN_EXE_pricing"))
        .output()
        .unwrap();
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("500.00 -> 450.00"));
    assert!(output.contains("10% discount -> 31.50"));
}
