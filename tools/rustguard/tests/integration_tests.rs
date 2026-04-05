mod common;

#[test]
fn test_unsafe_basic_detects_unsafe_function() {
    let result = common::run_on_fixture("unsafe_basic.rs", "text");
    assert!(
        result.stderr.contains("unsafe"),
        "expected unsafe findings in stderr, got:\n{}",
        result.stderr
    );
    // No error-severity findings by default, so exit code should be 0
    assert_eq!(
        result.exit_code, 0,
        "expected exit code 0 for info/warning findings"
    );
}

#[test]
fn test_unsafe_basic_json_output() {
    let result = common::run_on_fixture("unsafe_basic.rs", "json");
    assert!(
        result.stderr.contains("RG001") || result.stderr.contains("RG002"),
        "expected RG001 or RG002 rule IDs in JSON output, got:\n{}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("\"safety_comment_coverage_percent\""),
        "expected summary with safety_comment_coverage in JSON output",
    );
}

#[test]
fn test_unsafe_ffi_detects_extern_call() {
    let result = common::run_on_fixture("unsafe_ffi.rs", "text");
    assert!(
        result.stderr.contains("unsafe"),
        "expected unsafe findings for FFI fixture, got:\n{}",
        result.stderr
    );
}

#[test]
fn test_safe_code_no_unsafe_findings() {
    let result = common::run_on_fixture("clone_unnecessary.rs", "text");
    assert!(
        !result.stderr.contains("RG001") && !result.stderr.contains("RG002"),
        "expected no unsafe findings for safe code, got:\n{}",
        result.stderr
    );
    assert_eq!(result.exit_code, 0, "expected exit code 0 for clean code");
}
