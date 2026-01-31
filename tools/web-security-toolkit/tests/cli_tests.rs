//! Integration tests for CLI tools
//!
//! These tests verify that CLI tools work correctly with various inputs.

use std::process::Command;

/// Helper using cargo run
fn run_cargo(bin: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["run", "--bin", bin, "--"])
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", bin, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Some CLIs output to stderr for help/errors but still succeed conceptually
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("error") || stderr.contains("Error") {
            Err(stderr.to_string())
        } else {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    }
}

// ============================================================================
// Encoder Tests
// ============================================================================

mod encoder_tests {
    use super::*;

    #[test]
    fn test_encode_base64() {
        let output = run_cargo("encoder", &["encode", "base64", "hello"]).unwrap();
        assert!(output.contains("aGVsbG8="));
    }

    #[test]
    fn test_decode_base64() {
        let output = run_cargo("encoder", &["decode", "base64", "aGVsbG8="]).unwrap();
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_encode_hex() {
        let output = run_cargo("encoder", &["encode", "hex", "abc"]).unwrap();
        assert!(output.contains("616263"));
    }

    #[test]
    fn test_decode_hex() {
        let output = run_cargo("encoder", &["decode", "hex", "616263"]).unwrap();
        assert!(output.contains("abc"));
    }

    #[test]
    fn test_rot13() {
        let output = run_cargo("encoder", &["rot13", "hello"]).unwrap();
        assert!(output.contains("uryyb"));
    }

    #[test]
    fn test_rot13_inverse() {
        let output = run_cargo("encoder", &["rot13", "uryyb"]).unwrap();
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_juice_coupon() {
        let output = run_cargo("encoder", &["juice-coupon", "JAN", "26", "90"]).unwrap();
        assert!(output.contains("JAN26-90"));
        assert!(output.contains("Z85:"));
    }

    #[test]
    fn test_encode_z85() {
        let output = run_cargo("encoder", &["encode", "z85", "test"]).unwrap();
        // Z85 requires input length divisible by 4, so "test" should encode
        assert!(!output.is_empty());
    }
}

// ============================================================================
// JWT Tool Tests
// ============================================================================

mod jwt_tests {
    use super::*;

    #[test]
    fn test_jwt_decode() {
        // A simple JWT with alg:none
        let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.";
        let output = run_cargo("jwt-tool", &["decode", token]).unwrap();
        assert!(output.contains("Header:"));
        assert!(output.contains("Payload:"));
        assert!(output.contains("John Doe"));
    }

    #[test]
    fn test_jwt_unsigned() {
        let output = run_cargo("jwt-tool", &["unsigned", r#"{"role":"admin"}"#]).unwrap();
        assert!(output.contains("Unsigned JWT"));
        assert!(output.contains("eyJ")); // Base64 encoded header starts with eyJ
    }

    #[test]
    fn test_jwt_hs256() {
        let output = run_cargo("jwt-tool", &["hs256", r#"{"role":"admin"}"#, "secret"]).unwrap();
        assert!(output.contains("HS256 Signed JWT"));
        assert!(output.contains("eyJ"));
    }

    #[test]
    fn test_jwt_algorithms() {
        let output = run_cargo("jwt-tool", &["algorithms"]).unwrap();
        assert!(output.contains("none"));
        assert!(output.contains("HS256"));
        assert!(output.contains("RS256"));
    }

    #[test]
    fn test_jwt_juice_shop() {
        let output = run_cargo("jwt-tool", &["juice-shop"]).unwrap();
        assert!(output.contains("Juice Shop"));
    }
}

// ============================================================================
// Payload Generator Tests
// ============================================================================

mod payload_gen_tests {
    use super::*;

    #[test]
    fn test_sqli_auth_bypass() {
        let output = run_cargo("payload-gen", &["sqli", "auth-bypass"]).unwrap();
        assert!(output.contains("OR 1=1"));
        assert!(output.contains("--"));
    }

    #[test]
    fn test_sqli_union() {
        let output = run_cargo("payload-gen", &["sqli", "union", "5"]).unwrap();
        assert!(output.contains("UNION"));
        assert!(output.contains("NULL"));
    }

    #[test]
    fn test_sqli_login() {
        let output = run_cargo("payload-gen", &["sqli", "login", "admin@test.com"]).unwrap();
        assert!(output.contains("admin@test.com"));
        assert!(output.contains("'--"));
    }

    #[test]
    fn test_xss_basic() {
        let output = run_cargo("payload-gen", &["xss", "basic"]).unwrap();
        assert!(output.contains("<script>"));
        assert!(output.contains("alert"));
    }

    #[test]
    fn test_xss_bypass() {
        let output = run_cargo("payload-gen", &["xss", "bypass"]).unwrap();
        assert!(output.contains("XSS"));
    }

    #[test]
    fn test_xss_encode() {
        let output = run_cargo(
            "payload-gen",
            &["xss", "encode", "<script>alert(1)</script>"],
        )
        .unwrap();
        assert!(output.contains("URL encoded"));
        assert!(output.contains("HTML entities"));
    }

    #[test]
    fn test_xxe_file() {
        let output = run_cargo("payload-gen", &["xxe", "file", "/etc/passwd"]).unwrap();
        assert!(output.contains("<!DOCTYPE"));
        assert!(output.contains("ENTITY"));
        assert!(output.contains("/etc/passwd"));
    }

    #[test]
    fn test_xxe_ssrf() {
        let output = run_cargo("payload-gen", &["xxe", "ssrf", "http://internal:8080"]).unwrap();
        assert!(output.contains("<!DOCTYPE"));
        assert!(output.contains("http://internal:8080"));
    }

    #[test]
    fn test_nosql_auth_bypass() {
        let output = run_cargo("payload-gen", &["nosql", "auth-bypass"]).unwrap();
        assert!(output.contains("$ne"));
        assert!(output.contains("$gt"));
    }

    #[test]
    fn test_traversal() {
        let output = run_cargo("payload-gen", &["traversal", "5", "etc/passwd"]).unwrap();
        assert!(output.contains("../"));
        assert!(output.contains("etc/passwd"));
    }

    #[test]
    fn test_passwords_top() {
        let output = run_cargo("payload-gen", &["passwords", "top"]).unwrap();
        assert!(output.contains("password"));
        assert!(output.contains("123456"));
    }

    #[test]
    fn test_passwords_identify() {
        // MD5 hash of "password"
        let output = run_cargo(
            "payload-gen",
            &["passwords", "identify", "5f4dcc3b5aa765d61d8327deb882cf99"],
        )
        .unwrap();
        assert!(output.contains("MD5") || output.contains("Hash"));
    }

    #[test]
    fn test_idor_endpoints() {
        let output = run_cargo("payload-gen", &["idor", "endpoints"]).unwrap();
        assert!(output.contains("/api/"));
        assert!(output.contains("user"));
    }

    #[test]
    fn test_idor_ids() {
        let output = run_cargo("payload-gen", &["idor", "ids", "5", "10"]).unwrap();
        assert!(output.contains("1"));
        assert!(output.contains("10"));
    }

    #[test]
    fn test_tampering_negative() {
        let output = run_cargo("payload-gen", &["tampering", "negative", "quantity", "1"]).unwrap();
        assert!(output.contains("-"));
        assert!(output.contains("quantity"));
    }

    #[test]
    fn test_tampering_mass_assignment() {
        let output = run_cargo("payload-gen", &["tampering", "mass-assignment"]).unwrap();
        assert!(output.contains("admin") || output.contains("role"));
    }
}

// ============================================================================
// SSRF Scanner Tests
// ============================================================================

mod ssrf_tests {
    use super::*;

    #[test]
    fn test_localhost_variants() {
        let output = run_cargo("ssrf-scanner", &["localhost", "3000"]).unwrap();
        assert!(output.contains("127.0.0.1"));
        assert!(output.contains("localhost"));
        assert!(output.contains("3000"));
    }

    #[test]
    fn test_internal_variants() {
        let output = run_cargo("ssrf-scanner", &["internal", "80"]).unwrap();
        assert!(output.contains("10.") || output.contains("192.168.") || output.contains("172."));
    }

    #[test]
    fn test_file_variants() {
        let output = run_cargo("ssrf-scanner", &["file"]).unwrap();
        assert!(output.contains("file://"));
        assert!(output.contains("/etc/passwd") || output.contains("passwd"));
    }

    #[test]
    fn test_ip_convert() {
        let output = run_cargo("ssrf-scanner", &["ip-convert", "127.0.0.1"]).unwrap();
        assert!(output.contains("Decimal"));
        assert!(output.contains("2130706433"));
        assert!(output.contains("Hex"));
        assert!(output.contains("0x7f"));
    }

    #[test]
    fn test_juice_shop() {
        let output = run_cargo("ssrf-scanner", &["juice-shop"]).unwrap();
        assert!(output.contains("Juice Shop"));
        assert!(output.contains("localhost"));
    }
}

// ============================================================================
// Zip Payload Tests
// ============================================================================

mod zip_payload_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_list_targets() {
        let output = run_cargo("zip-payload", &["list"]).unwrap();
        assert!(output.contains("Zip Slip"));
        assert!(output.contains("../"));
    }

    #[test]
    fn test_create_payload() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test.zip");
        let output_str = output_path.to_str().unwrap();

        let output = run_cargo(
            "zip-payload",
            &[
                "create",
                "-o",
                output_str,
                "-t",
                "../../test.txt",
                "-c",
                "content",
            ],
        )
        .unwrap();

        assert!(output.contains("Created"));
        assert!(fs::metadata(&output_path).is_ok());
    }

    #[test]
    fn test_juice_shop_payload() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("exploit.zip");
        let output_str = output_path.to_str().unwrap();

        let output = run_cargo("zip-payload", &["juice-shop", "-o", output_str]).unwrap();

        assert!(output.contains("Video XSS") || output.contains("vtt"));
        assert!(fs::metadata(&output_path).is_ok());
    }
}

// ============================================================================
// Hashids Tool Tests
// ============================================================================

mod hashids_tests {
    use super::*;

    #[test]
    fn test_encode() {
        let output = run_cargo("hashids-tool", &["encode", "1,2,3", "--salt", "test"]).unwrap();
        assert!(output.contains("Encoded:"));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // First encode
        let encode_output =
            run_cargo("hashids-tool", &["encode", "42", "--salt", "mysalt"]).unwrap();
        let encoded = encode_output.trim().split_whitespace().last().unwrap();

        // Then decode
        let decode_output =
            run_cargo("hashids-tool", &["decode", encoded, "--salt", "mysalt"]).unwrap();
        assert!(decode_output.contains("42"));
    }

    #[test]
    fn test_salts() {
        let output = run_cargo("hashids-tool", &["salts"]).unwrap();
        assert!(output.contains("salt"));
    }

    #[test]
    fn test_salts_all() {
        let output = run_cargo("hashids-tool", &["salts", "--all"]).unwrap();
        assert!(output.contains("salt"));
        assert!(output.contains("Common"));
    }

    #[test]
    fn test_juice_shop_imaginary() {
        let output = run_cargo("hashids-tool", &["juice-shop", "--imaginary"]).unwrap();
        assert!(output.contains("imaginary") || output.contains("Code:"));
    }
}

// ============================================================================
// Web Scanner Tests (limited - no network calls)
// ============================================================================

mod web_scanner_tests {
    use super::*;

    #[test]
    fn test_recommended_headers() {
        let output = run_cargo("web-scanner", &["recommended-headers"]).unwrap();
        assert!(output.contains("Content-Security-Policy"));
        assert!(output.contains("Strict-Transport-Security"));
        assert!(output.contains("X-Content-Type-Options"));
    }
}

// ============================================================================
// HTTP Client Tests (limited - help only)
// ============================================================================

mod http_client_tests {
    use super::*;

    // HTTP client tests are limited as they require network access
    // We just verify the binary runs and shows help
    #[test]
    fn test_help() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "http-client", "--", "--help"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to run http-client");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("http-client") || stdout.contains("HTTP"));
    }
}

// ============================================================================
// KeePass Crack Tests (limited - help only)
// ============================================================================

mod keepass_tests {
    use super::*;

    #[test]
    fn test_wordlist_basic() {
        let output = run_cargo("keepass-crack", &["wordlist"]).unwrap();
        assert!(
            output.contains("password") || output.contains("admin") || output.contains("123456")
        );
    }

    #[test]
    fn test_wordlist_extended() {
        let output = run_cargo("keepass-crack", &["wordlist", "--extended"]).unwrap();
        // Extended should have more passwords
        let line_count = output.lines().count();
        assert!(line_count > 10);
    }
}

// ============================================================================
// TOTP Tool Tests
// ============================================================================

mod totp_tests {
    use super::*;

    #[test]
    fn test_generate() {
        let output = run_cargo("totp-tool", &["generate", "JBSWY3DPEHPK3PXP"]).unwrap();
        assert!(output.contains("TOTP Code:"));
        // Code should be 6 digits
        assert!(output.chars().filter(|c| c.is_ascii_digit()).count() >= 6);
    }

    #[test]
    fn test_window() {
        let output = run_cargo("totp-tool", &["window", "JBSWY3DPEHPK3PXP", "--size", "1"]).unwrap();
        assert!(output.contains("current"));
        assert!(output.contains("-1:"));
        assert!(output.contains("+1:"));
    }

    #[test]
    fn test_analyze() {
        let output = run_cargo("totp-tool", &["analyze", "JBSWY3DPEHPK3PXP"]).unwrap();
        assert!(output.contains("Valid Base32"));
        assert!(output.contains("true"));
    }

    #[test]
    fn test_bypasses() {
        let output = run_cargo("totp-tool", &["bypasses"]).unwrap();
        assert!(output.contains("Response manipulation") || output.contains("bypass"));
    }

    #[test]
    fn test_brute_force() {
        let output = run_cargo("totp-tool", &["brute-force"]).unwrap();
        assert!(output.contains("000000") || output.contains("Common"));
    }

    #[test]
    fn test_juice_shop() {
        let output = run_cargo("totp-tool", &["juice-shop"]).unwrap();
        assert!(output.contains("SQLi") || output.contains("totpSecret"));
    }
}

// ============================================================================
// SSTI Generator Tests
// ============================================================================

mod ssti_tests {
    use super::*;

    #[test]
    fn test_detect() {
        let output = run_cargo("ssti-gen", &["detect"]).unwrap();
        assert!(output.contains("{{7*7}}"));
        assert!(output.contains("Detection"));
    }

    #[test]
    fn test_jinja2() {
        let output = run_cargo("ssti-gen", &["jinja2"]).unwrap();
        assert!(output.contains("config"));
        assert!(output.contains("Jinja2"));
    }

    #[test]
    fn test_nodejs() {
        let output = run_cargo("ssti-gen", &["nodejs"]).unwrap();
        assert!(output.contains("process.env"));
    }

    #[test]
    fn test_rce() {
        let output = run_cargo("ssti-gen", &["rce", "ejs", "id"]).unwrap();
        assert!(output.contains("execSync"));
        assert!(output.contains("id"));
    }

    #[test]
    fn test_fuzz() {
        let output = run_cargo("ssti-gen", &["fuzz"]).unwrap();
        assert!(output.contains("{{"));
    }

    #[test]
    fn test_engines() {
        let output = run_cargo("ssti-gen", &["engines"]).unwrap();
        assert!(output.contains("Jinja2"));
        assert!(output.contains("EJS"));
        assert!(output.contains("Pug"));
    }

    #[test]
    fn test_juice_shop_ssti() {
        let output = run_cargo("ssti-gen", &["juice-shop"]).unwrap();
        assert!(output.contains("Pug") || output.contains("process"));
    }
}

// ============================================================================
// SVG Generator Tests
// ============================================================================

mod svg_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_xss_list() {
        let output = run_cargo("svg-gen", &["xss"]).unwrap();
        assert!(output.contains("XSS"));
        assert!(output.contains("onload") || output.contains("script"));
    }

    #[test]
    fn test_xxe_list() {
        let output = run_cargo("svg-gen", &["xxe"]).unwrap();
        assert!(output.contains("XXE"));
        assert!(output.contains("ENTITY"));
    }

    #[test]
    fn test_ssrf_list() {
        let output = run_cargo("svg-gen", &["ssrf"]).unwrap();
        assert!(output.contains("SSRF"));
    }

    #[test]
    fn test_generate_xss() {
        let output = run_cargo("svg-gen", &["generate", "xss", "alert(1)"]).unwrap();
        assert!(output.contains("<svg"));
        assert!(output.contains("alert(1)"));
    }

    #[test]
    fn test_generate_ssrf() {
        let output = run_cargo("svg-gen", &["generate", "ssrf", "http://internal:8080"]).unwrap();
        assert!(output.contains("<svg"));
        assert!(output.contains("http://internal:8080"));
    }

    #[test]
    fn test_generate_xxe() {
        let output = run_cargo("svg-gen", &["generate", "xxe", "/etc/passwd"]).unwrap();
        assert!(output.contains("ENTITY"));
        assert!(output.contains("/etc/passwd"));
    }

    #[test]
    fn test_bypass() {
        let output = run_cargo("svg-gen", &["bypass"]).unwrap();
        assert!(output.contains("Content-Type"));
        assert!(output.contains("svg"));
    }

    #[test]
    fn test_juice_shop_svg() {
        let output = run_cargo("svg-gen", &["juice-shop"]).unwrap();
        assert!(output.contains("Cross-Site Imaging") || output.contains("SVG"));
    }

    #[test]
    fn test_generate_xss_file() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test.svg");
        let output_str = output_path.to_str().unwrap();

        let output = run_cargo(
            "svg-gen",
            &["generate", "xss", "alert('test')", "-o", output_str],
        )
        .unwrap();

        assert!(output.contains("Saved") || std::fs::metadata(&output_path).is_ok());
    }
}

// ============================================================================
// Brute Force Generator Tests
// ============================================================================

mod bruteforce_tests {
    use super::*;

    #[test]
    fn test_pins() {
        let output = run_cargo("bruteforce-gen", &["pins"]).unwrap();
        assert!(output.contains("1234") || output.contains("PIN"));
    }

    #[test]
    fn test_pins_list() {
        let output = run_cargo("bruteforce-gen", &["pins", "--list"]).unwrap();
        assert!(output.contains("1234"));
        assert!(output.contains("0000"));
    }

    #[test]
    fn test_numeric() {
        let output = run_cargo("bruteforce-gen", &["numeric", "4", "0", "10"]).unwrap();
        assert!(output.contains("0000"));
        assert!(output.contains("0010"));
    }

    #[test]
    fn test_rate_limit() {
        let output = run_cargo("bruteforce-gen", &["rate-limit"]).unwrap();
        assert!(output.contains("X-Forwarded-For"));
    }

    #[test]
    fn test_ip_rotation() {
        let output = run_cargo("bruteforce-gen", &["ip-rotation", "10"]).unwrap();
        assert!(output.contains("0.0.0"));
    }

    #[test]
    fn test_security_question() {
        let output = run_cargo("bruteforce-gen", &["security-question", "pet"]).unwrap();
        assert!(output.contains("Zaya") || output.contains("Max") || output.contains("pet"));
    }

    #[test]
    fn test_enumeration() {
        let output = run_cargo("bruteforce-gen", &["enumeration"]).unwrap();
        assert!(output.contains("Response") || output.contains("enumeration"));
    }

    #[test]
    fn test_token_patterns() {
        let output = run_cargo("bruteforce-gen", &["token-patterns"]).unwrap();
        assert!(output.contains("Sequential") || output.contains("Timestamp"));
    }
}
