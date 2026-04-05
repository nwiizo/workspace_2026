mod common;

#[test]
fn test_unsafe_basic_detects_unsafe_function() {
    let result = common::run_on_fixture("unsafe_basic.rs", "text");
    // The analysis output goes to stderr
    assert!(
        result.stderr.contains("unsafe"),
        "expected unsafe findings in stderr, got:\n{}",
        result.stderr
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
    // clone_unnecessary.rs has no unsafe code
    assert!(
        !result.stderr.contains("RG001") && !result.stderr.contains("RG002"),
        "expected no unsafe findings for safe code, got:\n{}",
        result.stderr
    );
}
