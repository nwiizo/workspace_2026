//! Web Security Toolkit
//!
//! A comprehensive collection of tools for web security testing, CTF, and penetration testing.
//!
//! # Features
//!
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
