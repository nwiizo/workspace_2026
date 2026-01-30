# CLAUDE.md

Web Security Toolkit - Rust-based web security testing library and CLI tools.

## Purpose

General-purpose security testing toolkit for:
- CTF competitions
- Penetration testing
- Security assessments
- Educational purposes

## Architecture

```
src/
├── lib.rs                  # Library entry point
│
├── # Payload Generation Modules
├── encoding.rs             # Z85, Base64, Hex, ROT13
├── jwt.rs                  # JWT manipulation, algorithm confusion
├── sqli.rs                 # SQL injection payloads
├── xss.rs                  # XSS payloads with bypass techniques
├── xxe.rs                  # XML External Entity payloads
├── nosql.rs                # MongoDB injection payloads
├── ssrf.rs                 # SSRF bypass techniques
├── traversal.rs            # Path traversal payloads
├── zip_payload.rs          # Zip Slip attacks
├── passwords.rs            # Password lists, hash identification
│
├── # Testing Tools
├── idor.rs                 # IDOR testing utilities
├── param_tampering.rs      # Parameter manipulation tests
│
├── # Scanning Tools
├── http_client.rs          # Security-focused HTTP client
├── headers.rs              # Security headers analysis
├── scanner.rs              # Automated vulnerability scanner
│
└── bin/
    ├── encoder             # Encoding/decoding CLI
    ├── jwt-tool            # JWT manipulation CLI
    ├── payload-gen         # Payload generation CLI
    ├── ssrf-scanner        # SSRF scanner CLI
    ├── zip-payload         # Zip Slip generator CLI
    ├── web-scanner         # Vulnerability scanner CLI
    └── http-client         # HTTP client CLI
```

## Commands

### Build

```sh
cargo build --release
```

### Test

```sh
cargo test
```

### Format & Lint

```sh
cargo fmt && cargo clippy -- -D warnings
```

## CLI Quick Reference

```sh
# Encoding
encoder juice-coupon JAN 26 90
encoder encode base64 "secret"

# JWT
jwt-tool unsigned '{"role": "admin"}'
jwt-tool decode "eyJhbGciOi..."

# Payloads
payload-gen sqli juice-shop
payload-gen xss bypass
payload-gen xxe file /etc/passwd
payload-gen idor juice-shop
payload-gen tampering juice-shop

# Scanner
web-scanner scan https://example.com
web-scanner check-headers https://example.com

# HTTP Client
http-client get https://api.com --jwt "token"
http-client post https://api.com -d '{"key":"value"}'
```

## Module Guidelines

### Adding New Payloads

1. Create test functions with `#[test]` attribute
2. Include Juice Shop specific helpers if applicable
3. Add doc examples with `///` comments

### Example Pattern

```rust
/// Generate payloads for X attack
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::module::function;
///
/// let payloads = function();
/// assert!(!payloads.is_empty());
/// ```
pub fn function() -> Vec<Payload> {
    vec![
        Payload::new("name", "payload", Category::X),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        let payloads = function();
        assert!(!payloads.is_empty());
    }
}
```

## Test Coverage

Each module has:
- Unit tests for all public functions
- Doc tests for examples
- Juice Shop specific test cases where applicable

## Dependencies

- `clap` - CLI argument parsing
- `reqwest` - HTTP client
- `serde/serde_json` - JSON handling
- `base64`, `hex`, `z85` - Encoding
- `hmac`, `sha2`, `md5` - Cryptography
- `zip` - Zip file creation
- `thiserror` - Error handling
