# CLI Tools Documentation

Web Security Toolkit provides 13 CLI tools for security testing.

## Tools Overview

| Tool | Description |
|------|-------------|
| [encoder](encoder.md) | Multi-format encoder/decoder (Z85, Base64, Hex, ROT13) |
| [jwt-tool](jwt-tool.md) | JWT manipulation and attack tool |
| [payload-gen](payload-gen.md) | Security payload generator (SQLi, XSS, XXE, etc.) |
| [ssrf-scanner](ssrf-scanner.md) | SSRF payload generator and IP bypass tool |
| [web-scanner](web-scanner.md) | Web vulnerability scanner |
| [zip-payload](zip-payload.md) | Zip Slip attack payload generator |
| [http-client](http-client.md) | Security-focused HTTP client |
| [hashids-tool](hashids-tool.md) | Hashids encoder/decoder |
| [keepass-crack](keepass-crack.md) | KeePass KDBX password cracker |
| [totp-tool](totp-tool.md) | TOTP/2FA utility and bypass techniques |
| [ssti-gen](ssti-gen.md) | Server-Side Template Injection payloads |
| [svg-gen](svg-gen.md) | SVG XSS/XXE/SSRF payload generator |
| [bruteforce-gen](bruteforce-gen.md) | Brute force utilities and wordlists |

## Installation

Build all tools:
```bash
cargo build --release
```

Binaries are located in `target/release/`.

## Quick Reference

### Encoding & Decoding
```bash
encoder encode base64 "secret"
encoder decode z85 "encoded"
encoder juice-coupon JAN 26 90
```

### JWT Attacks
```bash
jwt-tool decode "eyJhbGci..."
jwt-tool unsigned '{"role":"admin"}'
```

### Payload Generation
```bash
payload-gen sqli auth-bypass
payload-gen xss bypass
payload-gen xxe file /etc/passwd
```

### Web Scanning
```bash
web-scanner check-headers https://example.com
web-scanner test-cors https://api.example.com
```

### HTTP Requests
```bash
http-client get https://api.example.com --jwt "token"
http-client post https://api.example.com -d '{"key":"value"}'
```

### SSRF Testing
```bash
ssrf-scanner localhost 3000
ssrf-scanner ip-convert 127.0.0.1
```

### Hashids
```bash
hashids-tool encode 1,2,3 --salt "secret"
hashids-tool juice-shop --imaginary
```

### KeePass Cracking
```bash
keepass-crack info database.kdbx
keepass-crack crack database.kdbx --extended
```

### TOTP/2FA
```bash
totp-tool generate JBSWY3DPEHPK3PXP
totp-tool window SECRET --size 2
totp-tool bypasses
```

### SSTI
```bash
ssti-gen detect
ssti-gen rce jinja2 "id"
ssti-gen juice-shop
```

### SVG Attacks
```bash
svg-gen xss
svg-gen generate xss "alert(1)" -o xss.svg
svg-gen juice-shop -o exploit.svg
```

### Brute Force
```bash
bruteforce-gen pins --list
bruteforce-gen rate-limit
bruteforce-gen security-question pet
```

## Use Cases

- **CTF Competitions**: Quick access to common attack payloads
- **Penetration Testing**: Automated payload generation and scanning
- **Security Education**: Learn about vulnerabilities and attack vectors

## Legal Notice

These tools are for authorized security testing only. Always obtain proper permission before testing any system.
