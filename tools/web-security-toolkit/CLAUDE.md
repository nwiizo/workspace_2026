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
├── ssti.rs                 # Server-Side Template Injection
├── prototype_pollution.rs  # JS prototype pollution payloads
├── svg.rs                  # SVG XSS/XXE/SSRF payloads
│
├── # High-Difficulty Challenge Tools
├── bruteforce.rs           # PIN patterns, rate limit bypass
├── hashids.rs              # Hashids encode/decode, salt discovery
├── keepass.rs              # KeePass KDBX cracking
├── totp.rs                 # TOTP generation, 2FA bypass
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
    ├── http-client         # HTTP client CLI
    ├── hashids-tool        # Hashids encode/decode CLI
    ├── keepass-crack       # KeePass cracker CLI
    ├── totp-tool           # TOTP/2FA utility CLI
    ├── ssti-gen            # SSTI payload generator CLI
    ├── svg-gen             # SVG attack payload CLI
    └── bruteforce-gen      # Brute force utility CLI
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

# Hashids (Continue Codes)
hashids-tool encode 1,2,3 --salt "my secret"
hashids-tool decode "abc123" --salt "my secret"
hashids-tool discover "someHashid"
hashids-tool juice-shop --imaginary
hashids-tool juice-shop --decode "continueCode"

# KeePass Cracker
keepass-crack info database.kdbx
keepass-crack crack database.kdbx
keepass-crack crack database.kdbx --extended
keepass-crack crack database.kdbx --keyfile image.jpg
keepass-crack decrypt database.kdbx -p "password"
keepass-crack extract database.kdbx -p "password" --format json

# TOTP/2FA
totp-tool generate JBSWY3DPEHPK3PXP
totp-tool window SECRET --size 2
totp-tool bypasses
totp-tool juice-shop

# SSTI
ssti-gen detect
ssti-gen rce jinja2 "id"
ssti-gen juice-shop

# SVG Attacks
svg-gen xss
svg-gen generate xss "alert(1)" -o xss.svg
svg-gen juice-shop -o exploit.svg

# Brute Force
bruteforce-gen pins --list
bruteforce-gen numeric 4 0 9999 --list
bruteforce-gen rate-limit
bruteforce-gen security-question pet
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
- `hmac`, `sha2`, `sha1`, `md5` - Cryptography
- `aes`, `cbc`, `salsa20` - Encryption (KeePass)
- `harsh` - Hashids encoding
- `flate2` - Gzip compression
- `rayon` - Parallel processing
- `zip` - Zip file creation
- `thiserror` - Error handling
