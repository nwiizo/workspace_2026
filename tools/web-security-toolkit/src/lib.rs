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
pub use headers::{analyze_headers, recommended_headers, HeaderCheck, Severity as HeaderSeverity};
pub use headers::generate_report as generate_headers_report;
pub use http_client::{SecurityClient, SecurityResponse, CookieInfo, HttpError};
pub use idor::{IdorEndpoint, IdorTestResult, IdorType, generate_id_variations, common_idor_endpoints, juice_shop_idor_endpoints};
pub use param_tampering::{TamperTest, TamperCategory, negative_value_tests, mass_assignment_tests, juice_shop_tampering_tests};
pub use scanner::{Scanner, ScanConfig, ScanResult, Finding, ScanSummary};
pub use scanner::generate_report as generate_scan_report;
