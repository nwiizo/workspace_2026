//! Security payloads for various attack vectors
//!
//! This module provides ready-to-use payloads for:
//! - SQL Injection
//! - XSS (Cross-Site Scripting)
//! - XXE (XML External Entity)
//! - NoSQL Injection
//! - Path Traversal
//! - JWT Manipulation
//! - SSRF (Server-Side Request Forgery)
//! - Redirect/Allowlist Bypass
//! - Zip Slip (Archive Path Traversal)
//! - Encoding utilities

pub mod encoding;
pub mod jwt;
pub mod nosql;
pub mod redirect;
pub mod sqli;
pub mod ssrf;
pub mod traversal;
pub mod xss;
pub mod xxe;

#[cfg(feature = "zip-payloads")]
pub mod zipslip;
