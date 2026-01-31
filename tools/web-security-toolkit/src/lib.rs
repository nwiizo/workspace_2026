//! Web Security Toolkit
//!
//! A comprehensive collection of tools for web security testing, CTF, and penetration testing.
//!
//! # Features
//!
//! ## Payload Generation
//! - **Encoding**: Z85, Base64, Hex encoding/decoding, ROT13
//! - **JWT**: JWT manipulation, unsigned JWT, algorithm confusion attacks
//! - **SQLi**: SQL injection payload generation for multiple databases
//! - **XSS**: Cross-site scripting payloads with filter bypass techniques
//! - **XXE**: XML External Entity payloads for file read, SSRF, DoS
//! - **NoSQL**: MongoDB injection payloads
//! - **Traversal**: Path traversal and LFI payloads
//! - **SSRF**: Server-side request forgery bypass techniques
//! - **Zip Payload**: Zip Slip payloads for path traversal
//! - **Passwords**: Common passwords, hash identification, credential utilities
//!
//! ## Practical Tools
//! - **HTTP Client**: Security-focused HTTP client with cookie/JWT support
//! - **Headers**: Security headers analysis and recommendations
//! - **Scanner**: Automated vulnerability scanner
//!
//! # Example
//!
//! ```rust
//! use web_security_toolkit::encoding::z85_encode;
//! use web_security_toolkit::ssrf::generate_localhost_variants;
//! use web_security_toolkit::jwt::create_unsigned_jwt;
//! use serde_json::json;
//!
//! // Encoding
//! let encoded = z85_encode("test");
//!
//! // SSRF bypass
//! let variants = generate_localhost_variants(8080);
//!
//! // Unsigned JWT
//! let jwt = create_unsigned_jwt(&json!({"role": "admin"}));
//! ```

// Payload generation modules
pub mod encoding;
pub mod jwt;
pub mod nosql;
pub mod passwords;
pub mod sqli;
pub mod ssrf;
pub mod traversal;
pub mod xss;
pub mod xxe;
pub mod zip_payload;

// High-difficulty challenge tools
pub mod bruteforce;
pub mod hashids;
pub mod keepass;
pub mod prototype_pollution;
pub mod ssti;
pub mod svg;
pub mod totp;

// Practical tools
pub mod headers;
pub mod http_client;
pub mod idor;
pub mod param_tampering;
pub mod scanner;

pub use encoding::*;
pub use jwt::*;
pub use nosql::*;
pub use passwords::*;
pub use sqli::*;
pub use ssrf::*;
pub use traversal::*;
pub use xss::*;
pub use xxe::*;
pub use zip_payload::*;

// Re-export specific items to avoid ambiguity
pub use headers::generate_report as generate_headers_report;
pub use headers::{analyze_headers, recommended_headers, HeaderCheck, Severity as HeaderSeverity};
pub use http_client::{CookieInfo, HttpError, SecurityClient, SecurityResponse};
pub use idor::{
    common_idor_endpoints, generate_id_variations, juice_shop_idor_endpoints, IdorEndpoint,
    IdorTestResult, IdorType,
};
pub use param_tampering::{
    juice_shop_tampering_tests, mass_assignment_tests, negative_value_tests, TamperCategory,
    TamperTest,
};
pub use scanner::generate_report as generate_scan_report;
pub use scanner::{Finding, ScanConfig, ScanResult, ScanSummary, Scanner};

// High-difficulty tools re-exports
pub use bruteforce::{
    common_pins, generate_ip_rotation, numeric_sequence, rate_limit_bypasses,
    security_question_wordlist, EnumerationIndicator, RateLimitBypass,
};
pub use hashids::{
    common_salts, decode_continue_code, decode_hashid, discover_salt, encode_hashid,
    generate_continue_code, generate_forged_continue_codes, generate_imaginary_challenge_codes,
    juice_shop_salts, try_decode_with_salts,
};
pub use keepass::{
    common_passwords, crack_kdbx, crack_kdbx_with_keyfile, crack_kdbx_with_progress, decrypt_kdbx,
    extended_passwords, extract_entries, parse_entries, try_password, try_password_with_keyfile,
    InnerStreamCipher, KdbxEntry, KdbxFile, KdbxHeader, KeePassError,
};
pub use prototype_pollution::{
    basic_payloads as pp_basic_payloads, dos_payloads, nodejs_rce_payloads, query_string_payloads,
    PollutionCategory, PrototypePollutionPayload,
};
pub use ssti::{
    detection_payloads as ssti_detection_payloads, generate_rce_payload, jinja2_payloads,
    juice_shop_ssti, nodejs_payloads as ssti_nodejs_payloads, ssti_fuzz_payloads, SstiPayload,
    SstiPurpose, TemplateEngine,
};
pub use svg::{
    cross_site_imaging_payloads, generate_svg_ssrf, generate_svg_xss, generate_svg_xxe,
    svg_ssrf_payloads, svg_xss_payloads, svg_xxe_payloads, SvgCategory, SvgPayload,
};
pub use totp::{
    analyze_secret, brute_force_codes, generate_totp, generate_totp_at, generate_totp_window,
    two_factor_bypasses, BypassTechnique, TwoFactorBypass,
};
