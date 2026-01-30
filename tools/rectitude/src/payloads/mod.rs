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
//! - Encoding utilities

pub mod encoding;
pub mod jwt;
pub mod nosql;
pub mod sqli;
pub mod ssrf;
pub mod traversal;
pub mod xss;
pub mod xxe;
